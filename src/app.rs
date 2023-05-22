use std::{
    sync::{atomic::AtomicBool, mpsc::Sender, Arc, RwLock},
    time::Duration,
};

use slotmap::SecondaryMap;

use crate::{
    error::{Error, Result},
    event::*,
    layout::*,
    surface::{term::*, *},
    widget::{RenderCtx, UpdateCtx},
    Widget,
};

/// Contains configuration options for the Sanguine application.
pub struct Config {
    /// Whether or not to quit on ctrl-q (default: true)
    ///
    /// Set to false if you implement your own exit handling.
    pub ctrl_q_quit: bool,
    /// Whether or not to focus a window when the mouse hovers over it (default: false)
    pub focus_follows_hover: bool,
}

impl Config {
    /// Create a new Config struct with the given options
    pub fn new() -> Self {
        Default::default()
    }

    /// Set whether or not to quit on ctrl-q
    pub fn ctrl_q_quit(mut self, ctrl_q_quit: bool) -> Self {
        self.ctrl_q_quit = ctrl_q_quit;
        self
    }

    /// Set whether or not to focus a window when the mouse hovers over it
    pub fn focus_follows_hover(mut self, focus_follows_hover: bool) -> Self {
        self.focus_follows_hover = focus_follows_hover;
        self
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ctrl_q_quit: true,
            focus_follows_hover: false,
        }
    }
}

pub type GlobalHandler<S, U> =
    dyn Fn(&mut App<S, U>, &Event<U>, Arc<Sender<UserEvent<U>>>) -> Result<bool>;

/// The main application struct, responsible for managing the layout tree,
/// keeping track of focus, and rendering the widgets.
///
/// The generic type U is the type of user events that can be sent to widgets. It can be used to
/// define custom message-passing behavior between widgets.
pub struct App<S = (), U = ()> {
    /// The layout tree
    layout: Layout<U, S>,
    /// The post-render widget rects for mouse events
    rendered: SecondaryMap<NodeId, Vec<(Rect, Arc<RwLock<dyn Widget<U, S>>>)>>,
    /// The actual terminal used for rendering
    term: BufferedTerminal<UnixTerminal>,
    /// The size of the terminal
    size: Rect,
    /// The focused node in the tree, if any
    focus: Option<NodeId>,
    /// Sender for user events, given to widgets when `Widget::update` is called
    event_tx: Arc<std::sync::mpsc::Sender<UserEvent<U>>>,
    /// Receiver for user events, only used internally
    event_rx: std::sync::mpsc::Receiver<UserEvent<U>>,
    /// Used to signal the exit internally
    exit: AtomicBool,
    /// Global event handler, which intercepts events before they are propagated to the focused
    /// widget. If the handler returns `Ok(true)`, the event is considered handled and is not
    /// propagated to the widget that would otherwise receive it.
    global_event_handler: Box<GlobalHandler<S, U>>,
    /// Configuration struct
    config: Config,
    /// User state
    state: S,
}

impl<S, U> Drop for App<S, U> {
    fn drop(&mut self) {
        // Restore cursor visibility and leave alternate screen when app exits
        self.term
            .add_change(Change::CursorVisibility(CursorVisibility::Visible));
        self.term.terminal().exit_alternate_screen().unwrap();
    }
}

impl<S: 'static, U: 'static> App<S, U> {
    fn render_ctx(&self, node: NodeId) -> Result<(Arc<RwLock<dyn Widget<U, S>>>, &Rect)> {
        Ok((
            // Retrieve widget trait object from node
            self.layout
                .widget(node)
                .ok_or(Error::WidgetNotFound(node))?,
            // Retrieve computed layout for window
            self.layout
                .layout(node)
                .ok_or(Error::WidgetNotFound(node))?,
        ))
    }

    fn global_event(&mut self, event: &Event<U>) -> Result<bool> {
        if self.config.ctrl_q_quit {
            if let Event::Key(KeyEvent {
                key: KeyCode::Char('q'),
                modifiers: Modifiers::CTRL,
            }) = event
            {
                self.event_tx
                    .send(UserEvent::Exit)
                    .map_err(|_| Error::SignalSendFail)?
            }
        }

        // Safety: The function pointer is stored in self so the borrow checker doesn't like
        // us calling it with a mutable reference to self. However, the function pointer won't be changed
        // so it should be safe to call with a mutable reference to self.
        let evt = &self.global_event_handler as *const GlobalHandler<S, U>;
        unsafe { (*evt)(self, event, self.event_tx.clone()) }
    }

    fn process_event(&mut self, event: Event<U>) -> Result<()> {
        match &event {
            Event::Resize { cols, rows } => {
                self.size = Rect::from_size((*cols, *rows));
                self.term.resize(*cols, *rows);
                self.term.repaint().map_err(|_| Error::TerminalError)?;
                self.term.flush().map_err(|_| Error::TerminalError)?;
                self.layout.mark_dirty();
            }
            Event::Mouse(MouseEvent {
                mut x,
                mut y,
                mouse_buttons,
                modifiers,
            }) => {
                y -= 1;
                x -= 1;
                if !self.global_event(&event)? {
                    let Some(node) = self.layout.node_at_pos((x, y)) else {
                        return Ok(());
                    };
                    if let Some(focus) = self.focus {
                        let focus = if focus != node {
                            // Send hover events to the hovered node, but focus the window if the mouse is clicked
                            if *mouse_buttons != MouseButtons::NONE {
                                // If the node under the mouse is different from the focused node,
                                // focus the new node and consume the event
                                self.focus = Some(node);
                                return Ok(());
                            }
                            node
                        } else {
                            focus
                        };

                        // check if there are inner widgets that the event should be sent to
                        let children = self.rendered.get(focus).cloned().unwrap_or(vec![]);
                        let child = children
                            .iter()
                            .filter(|(rect, _)| rect.contains(x as f32, y as f32))
                            .next();

                        // If the node under the mouse is the same as the focused node,
                        // send the event to the focused node
                        let (mut widget, mut layout) =
                            self.render_ctx(focus).map(|(w, l)| (w, l.clone())).unwrap();

                        if let Some((child_layout, child_widget)) = child {
                            layout = Rect {
                                x: /* layout.x +  */child_layout.x,
                                y: /* layout.y +  */child_layout.y,
                                width: child_layout.width,
                                height: child_layout.height,
                            };
                            widget = child_widget.clone();
                        } else if children.len() > 0 {
                            return Ok(());
                        }

                        let offset_event = Event::Mouse(MouseEvent {
                            x: x - layout.x as u16,
                            y: y - layout.y as u16,
                            mouse_buttons: *mouse_buttons,
                            modifiers: *modifiers,
                        });

                        widget
                            .write()
                            .map_err(|_| Error::WidgetWriteLockError(focus))?
                            .update(
                                &mut UpdateCtx::new(
                                    focus,
                                    layout,
                                    &mut self.layout,
                                    self.event_tx.clone(),
                                    &mut self.state,
                                ),
                                offset_event,
                            )?;
                    } else if *mouse_buttons == MouseButtons::LEFT
                        || self.config.focus_follows_hover
                    {
                        // If there's no focus, focus the node under the mouse
                        self.focus = Some(node);
                    }
                }
            }
            Event::User(UserEvent::Exit) => {
                self.exit.store(true, std::sync::atomic::Ordering::SeqCst);
            }
            // Anything that doesn't need special handling (keys, paste, user events)
            _ => {
                // Handle global events
                if !self.global_event(&event)? {
                    let Some(focus) = self.focus else {
                        // If there's no focus, we can't do anything
                        let Some(leaf) = self.layout.leaves().first().cloned() else {
                            return Ok(());
                        };
                        self.set_focus(leaf)?;
                        return Ok(())
                    };
                    // Retrieve widget trait object from node
                    let Some(widget) = self
                        .layout
                        .widget(focus) else {
                            return Ok(());
                        };

                    // Retrieve computed layout for window
                    let Some(layout) = self
                        .layout
                        .layout(focus)
                        .cloned() else {
                            return Ok(());
                        };
                    let tx = self.event_tx.clone();

                    widget
                        .write()
                        .map_err(|_| Error::WidgetWriteLockError(focus))?
                        .update(
                            &mut UpdateCtx::new(
                                focus,
                                layout,
                                &mut self.layout,
                                tx,
                                &mut self.state,
                            ),
                            event,
                        )?;
                };
            }
        }

        Ok(())
    }

    fn handle_user_events(&mut self) -> Result<()> {
        if let Ok(event) = self.event_rx.try_recv() {
            self.process_event(Event::User(event))?;
        }
        Ok(())
    }

    fn handle_input_events(&mut self) -> Result<()> {
        while let Some(event) = self
            .term
            .terminal()
            .poll_input(Some(Duration::from_millis(15)))
            .map_err(|_| Error::PollInputFailed)?
        {
            use termwiz::input::InputEvent;
            let translated = match event {
                InputEvent::Key(k) => Event::Key(k),
                InputEvent::Mouse(m) => Event::Mouse(m),
                InputEvent::Resized { rows, cols } => Event::Resize { rows, cols },
                InputEvent::Paste(s) => Event::Paste(s),
                _ => continue,
            };
            self.process_event(translated)?;
        }
        Ok(())
    }

    /// Calls a closure, passing in a mutable reference to the layout.
    pub fn update_layout<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut Layout<U, S>) -> R,
        R: Sized,
    {
        f(&mut self.layout)
    }

    /// Calls a closure, passing in an immutable reference to the layout.
    pub fn inspect_layout<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&Layout<U, S>) -> R,
        R: Sized,
    {
        f(&self.layout)
    }

    /// Handles and propagates events, returning whether or not the app should continue running.
    ///
    /// This should be used as the condition (or part of the condition) for an application's render loop.
    pub fn handle_events(&mut self) -> Result<bool> {
        self.handle_user_events()?;
        self.handle_input_events()?;
        Ok(!self.exit.load(std::sync::atomic::Ordering::SeqCst))
    }

    /// Sets the focus to the given node.
    pub fn set_focus(&mut self, node: NodeId) -> Result<()> {
        if self.layout.is_container(node) {
            return Err(Error::ExpectedLeaf(node));
        }
        self.focus = Some(node);
        Ok(())
    }

    /// Get the id of the currently focused node, if any
    pub fn get_focus(&self) -> Option<NodeId> {
        self.focus
    }

    /// Cycle focus to the next window
    pub fn cycle_focus(&mut self) -> Result<()> {
        let current = self.get_focus().ok_or(Error::NoFocus)?;
        let next = self.inspect_layout(|l| {
            l.leaves()
                .into_iter()
                .cycle()
                .skip_while(|v| *v != current)
                .nth(1)
                .ok_or(Error::NoFocus)
        })?;
        self.set_focus(next)?;
        Ok(())
    }

    /// Focus the window in the given direction from the currently focused one
    pub fn focus_direction(&mut self, direction: Direction) -> Result<()> {
        let current = self.get_focus().ok_or(Error::NoFocus)?;
        let available = self.inspect_layout(|l| l.adjacent_on_side(current, direction));
        let Some(next) = available.iter().next() else {
            return Ok(());
        };
        self.set_focus(*next)?;
        Ok(())
    }

    fn render_recursive(
        &mut self,
        owner: NodeId,
        inner_widget: Option<Arc<RwLock<dyn Widget<U, S>>>>,
        inner_layout: Option<Rect>,
        mut screen: &mut Surface,
    ) {
        let layout = match inner_layout {
            Some(layout) => layout,
            None => {
                if let Some(layout) = self.layout.layout(owner) {
                    layout.clone()
                } else {
                    return;
                }
            }
        };
        let widget = match inner_widget.clone() {
            Some(widget) => widget,
            None => {
                if let Some(widget) = self.layout.widget(owner) {
                    widget.clone()
                } else {
                    return;
                }
            }
        };

        // Draw onto widget screen for composition
        let mut widget_screen = Surface::new(layout.width as usize, layout.height as usize);

        // Render widget onto widget screen
        let focused = self.focus.map(|f| f == owner).unwrap_or(false);
        let inner_widgets = match widget.read() {
            Ok(widget) => widget.render(
                &RenderCtx::new(focused, &self.layout, &self.state),
                &mut widget_screen,
            ),
            Err(_) => return,
        };

        // Draw widget onto background screen
        screen.draw_from_screen(&widget_screen, layout.x as usize, layout.y as usize);
        if inner_widget.is_some() {
            self.rendered.get_mut(owner).unwrap().push((
                Rect {
                    x: layout.x,
                    y: layout.y,
                    width: layout.width,
                    height: layout.height,
                },
                widget,
            ));
        } else {
            self.rendered.insert(owner, vec![]);
        }

        if let Some(inner_widgets) = inner_widgets {
            inner_widgets.into_iter().for_each(|(rect, widget)| {
                self.render_recursive(
                    owner,
                    Some(widget.clone()),
                    Some(Rect {
                        x: layout.x + rect.x,
                        y: layout.y + rect.y,
                        width: rect.width,
                        height: rect.height,
                    }),
                    &mut screen,
                );
                self.rendered.get_mut(owner).unwrap().push((
                    Rect {
                        x: layout.x + rect.x,
                        y: layout.y + rect.y,
                        width: rect.width,
                        height: rect.height,
                    },
                    widget,
                ));
            });
        }
    }

    /// Render the entire application to the terminal
    pub fn render(&mut self) -> Result<()> {
        self.rendered.clear();
        self.layout.compute(&self.size);

        // Create temporary background screen
        let mut screen = Surface::new(self.size.width as usize, self.size.height as usize);

        let leaves = self.layout.leaves();
        let floats = self.layout.floats();

        for node in leaves.into_iter().chain(floats) {
            self.render_recursive(node, None, None, &mut screen);
        }

        // Draw contents of background screen to terminal
        self.term.draw_from_screen(&screen, 0, 0);

        if let Some(focus) = self.focus {
            if let Some(layout) = self.layout.layout(focus) {
                if let Some(cursor) = self.layout.widget(focus).unwrap().read().unwrap().cursor() {
                    if let Some(child) = cursor.0 {
                        let child = self.rendered.get(focus).unwrap().get(child).unwrap();
                        // let cursor = child.1.read().unwrap().cursor().unwrap();
                        self.term.add_changes(vec![
                            Change::CursorVisibility(CursorVisibility::Visible),
                            Change::CursorPosition {
                                x: Position::Absolute((child.0.x) as usize + cursor.1),
                                y: Position::Absolute((child.0.y) as usize + cursor.2),
                            },
                        ]);
                    } else {
                        self.term.add_changes(vec![
                            Change::CursorVisibility(CursorVisibility::Visible),
                            Change::CursorPosition {
                                x: Position::Absolute(layout.x as usize + cursor.1),
                                y: Position::Absolute(layout.y as usize + cursor.2),
                            },
                        ]);
                    }
                } else {
                    self.term
                        .add_changes(vec![Change::CursorVisibility(CursorVisibility::Hidden)]);
                }
            }
        }

        // Compute optimized diff and flush
        self.term
            .flush()
            .map_err(|_| Error::external("could not flush terminal"))?;

        Ok(())
    }

    /// Create a new Sanguine application with the provided layout and no global event handler.
    pub fn new(layout: Layout<U, S>, config: Config, state: S) -> Result<Self> {
        let term = Capabilities::new_from_env()
            .and_then(|caps| {
                UnixTerminal::new(caps).and_then(|mut t| {
                    t.set_raw_mode()?;
                    t.enter_alternate_screen()?;
                    BufferedTerminal::new(t)
                })
            })
            .map_err(|_| Error::TerminalError)?;
        let (event_tx, event_rx) = std::sync::mpsc::channel();

        Ok(App {
            global_event_handler: Box::new(|_, _, _| Ok(false)),
            size: Rect::from_size(term.dimensions()),
            event_tx: Arc::new(event_tx),
            exit: AtomicBool::new(false),
            rendered: SecondaryMap::new(),
            focus: None,
            layout,
            term,
            event_rx,
            config,
            state,
        })
    }

    /// Create a new Sanguine app with the provided global event handler. The global event handler
    /// intercepts events before they are sent to widgets. It can return true to prevent the event
    /// from propagating to widgets, or false to allow propagation.
    pub fn with_global_handler(
        layout: Layout<U, S>,
        config: Config,
        state: S,
        handler: impl Fn(&mut App<S, U>, &Event<U>, Arc<Sender<UserEvent<U>>>) -> Result<bool> + 'static,
    ) -> Result<Self> {
        let mut new = Self::new(layout, config, state)?;
        new.global_event_handler = Box::new(handler);
        Ok(new)
    }
}
