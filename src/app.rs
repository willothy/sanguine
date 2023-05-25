use std::{
    sync::{atomic::AtomicBool, mpsc::Sender, Arc, RwLock},
    time::Duration,
};

pub use crate::widget::{RenderCtx, UpdateCtx};

use slotmap::SecondaryMap;

use crate::{
    error::{Error, Result},
    event::*,
    layout::*,
    surface::{term::*, *},
    Widget,
};

/// Contains configuration options for the Sanguine application.
pub struct Config {
    /// Whether or not to quit on <kbd>ctrl</kbd>+<kbd>q</kbd> `default: true`
    ///
    /// Set to false if you implement your own exit handling.
    pub ctrl_q_quit: bool,
    /// Whether or not to focus a window when the mouse hovers over it `default: false`
    pub focus_follows_hover: bool,
}

impl Config {
    /// Create a new Config struct with the given options
    pub fn new() -> Self {
        Default::default()
    }

    /// Set whether or not to quit on <kbd>ctrl</kbd>+<kbd>q</kbd>
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
    // rendered: SecondaryMap<NodeId, Vec<Vec<(Rect, Arc<RwLock<dyn Widget<U, S>>>)>>>,
    rendered: SecondaryMap<WidgetId, (Rect, Option<Vec<WidgetId>>)>,
    /// The actual terminal used for rendering
    term: BufferedTerminal<UnixTerminal>,
    /// The size of the terminal
    size: Rect,
    /// The focused node in the tree, if any
    focus: Option<WidgetId>,
    /// Sender for user events, given to widgets when `Widget::update` is called
    event_tx: Arc<std::sync::mpsc::Sender<UserEvent<U>>>,
    /// Receiver for user events, only used internally
    event_rx: std::sync::mpsc::Receiver<UserEvent<U>>,
    /// Used to signal the exit internally
    exit: Arc<AtomicBool>,
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

impl<S: Default + 'static, U: 'static> Default for App<S, U> {
    fn default() -> Self {
        let term = Capabilities::new_from_env()
            .and_then(|caps| {
                UnixTerminal::new(caps).and_then(|mut t| {
                    t.set_raw_mode()?;
                    t.enter_alternate_screen().ok();
                    BufferedTerminal::new(t)
                })
            })
            .unwrap();
        let (event_tx, event_rx) = std::sync::mpsc::channel();
        Self {
            global_event_handler: Box::new(|_, _, _| Ok(false)),
            size: Rect::from_size(term.dimensions()),
            event_tx: Arc::new(event_tx),
            exit: Arc::new(AtomicBool::new(false)),
            rendered: SecondaryMap::new(),
            layout: Layout::new(),
            focus: None,
            term,
            event_rx,
            config: Default::default(),
            state: Default::default(),
        }
    }
}

impl<S: Default + 'static, U: 'static> App<S, U> {
    /// Create a new Sanguine application with the provided layout and no global event handler.
    pub fn new(config: Config) -> Result<Self> {
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
            exit: Arc::new(AtomicBool::new(false)),
            rendered: SecondaryMap::new(),
            layout: Layout::new(),
            focus: None,
            term,
            event_rx,
            config,
            state: Default::default(),
        })
    }

    /// Create a new Sanguine app with the provided global event handler. The global event handler
    /// intercepts events before they are sent to widgets. It can return true to prevent the event
    /// from propagating to widgets, or false to allow propagation.
    pub fn new_with_handler(
        config: Config,
        handler: impl Fn(&mut App<S, U>, &Event<U>, Arc<Sender<UserEvent<U>>>) -> Result<bool> + 'static,
    ) -> Result<Self> {
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
            global_event_handler: Box::new(handler),
            size: Rect::from_size(term.dimensions()),
            event_tx: Arc::new(event_tx),
            exit: Arc::new(AtomicBool::new(false)),
            rendered: SecondaryMap::new(),
            layout: Layout::new(),
            focus: None,
            term,
            event_rx,
            config,
            state: Default::default(),
        })
    }
}

impl<S: 'static, U: 'static> App<S, U> {
    pub fn new_with_state(config: Config, state: S) -> Result<Self> {
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
            exit: Arc::new(AtomicBool::new(false)),
            rendered: SecondaryMap::new(),
            layout: Layout::new(),
            focus: None,
            term,
            event_rx,
            config,
            state,
        })
    }

    pub fn with_state(mut self, state: S) -> Self {
        self.state = state;
        self
    }

    pub fn with_handler(
        mut self,
        handler: impl Fn(&mut App<S, U>, &Event<U>, Arc<Sender<UserEvent<U>>>) -> Result<bool> + 'static,
    ) -> Self {
        self.global_event_handler = Box::new(handler);
        self
    }

    pub fn with_layout(mut self, f: impl Fn(&mut Layout<U, S>) -> Option<WidgetId>) -> Self {
        let res = f(&mut self.layout);
        if let Some(res) = res {
            self.set_focus(res).ok();
        }
        self
    }

    pub fn handler(
        &mut self,
        handler: impl Fn(&mut App<S, U>, &Event<U>, Arc<Sender<UserEvent<U>>>) -> Result<bool> + 'static,
    ) {
        self.global_event_handler = Box::new(handler);
    }

    fn get_widget(&self, id: WidgetId) -> Result<Arc<RwLock<dyn Widget<U, S>>>> {
        id.get(&self.layout).ok_or(Error::WidgetNotFound(id))
    }

    fn render_ctx(&self, node: NodeId) -> Result<(WidgetId, &Rect)> {
        Ok((
            // Retrieve widget trait object from node
            node.widget(&self.layout).ok_or(Error::NodeNotFound(node))?,
            // Retrieve computed layout for window
            self.layout.layout(node).ok_or(Error::NodeNotFound(node))?,
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
                return Ok(());
                // y -= 1;
                // x -= 1;
                if !self.global_event(&event)? {
                    let Some(node) = self.layout.node_at_pos((x, y)) else {
                        return Ok(());
                    };
                    if let Some(focus) = self.focus {
                        // let focus = if focus != node {
                        //     // Send hover events to the hovered node, but focus the window if the mouse is clicked
                        //     if *mouse_buttons != MouseButtons::NONE {
                        //         // If the node under the mouse is different from the focused node,
                        //         // focus the new node and consume the event
                        //         self.focus = Some(node);
                        //         return Ok(());
                        //     }
                        //     node
                        // } else {
                        //     focus
                        // };

                        // // If the node under the mouse is the same as the focused node,
                        // // send the event to the focused node
                        // let (mut widget, mut layout) =
                        //     self.render_ctx(focus).map(|(w, l)| (w, l.clone())).unwrap();
                        let mut widget = focus;
                        let mut layout = self.rendered.get(focus).cloned().unwrap().0;

                        // check if there are inner widgets that the event should be sent to
                        while let Some((_, Some(nested))) = self.rendered.get(widget) {
                            let child = nested.iter().find_map(|child| {
                                let rect = &self.rendered.get(*child).unwrap().0;
                                rect.contains(x as f32, y as f32).then_some((rect, *child))
                            });

                            if let Some((child_layout, child_widget)) = child {
                                layout = Rect {
                                    x: child_layout.x + 1.,
                                    y: child_layout.y + 1.,
                                    width: child_layout.width,
                                    height: child_layout.height,
                                };
                                widget = child_widget;
                            } else {
                                break;
                            }
                        }

                        let offset_event = Event::Mouse(MouseEvent {
                            x: x - layout.x as u16,
                            y: y - layout.y as u16,
                            mouse_buttons: *mouse_buttons,
                            modifiers: *modifiers,
                        });

                        widget
                            .get(&self.layout)
                            .unwrap()
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
                        self.focus = Some(node.widget(&self.layout).unwrap());
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
                        self.set_focus(leaf.widget(&self.layout).unwrap())?;
                        return Ok(())
                    };
                    // Retrieve widget trait object from node
                    let Some(widget) = focus.get(&self.layout) else {
                        return Ok(());
                    };

                    // Retrieve computed layout for window
                    let Some(layout) = self
                        .rendered.get(focus)
                        .map(|v| &v.0) 
                    else {
                        return Ok(());
                    };
                    let tx = self.event_tx.clone();

                    widget
                        .write()
                        .map_err(|_| Error::WidgetWriteLockError(focus))?
                        .update(
                            &mut UpdateCtx::new(
                                focus,
                                layout.clone(),
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
    pub fn set_focus(&mut self, widget: WidgetId) -> Result<()> {
        self.focus = Some(widget);
        Ok(())
    }

    /// Get the id of the currently focused node, if any
    pub fn get_focus(&self) -> Option<WidgetId> {
        self.focus
    }

    /// Cycle focus to the next window
    pub fn cycle_focus(&mut self) -> Result<()> {
        let current = self.get_focus().ok_or(Error::NoFocus)?;
        let next = self.inspect_layout(|l| {
            l.leaves()
                .into_iter()
                .map(|v| v.widget(&self.layout).unwrap())
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
        // TODO
        // let current = self.get_focus().ok_or(Error::NoFocus)?;
        // let available = self.inspect_layout(|l| l.adjacent_on_side(current, direction));
        // let Some(next) = available.iter().next() else {
        //     return Ok(());
        // };
        // self.set_focus(*next)?;
        Ok(())
    }

    fn render_recursive(&mut self, widget: WidgetId, layout: &Rect, screen: &mut Surface) {
        // Draw onto widget screen for composition
        let mut widget_screen = Surface::new(layout.width as usize, layout.height as usize);

        // Render widget onto widget screen
        let inner_widgets = match widget.get(&self.layout).unwrap().read() {
            Ok(widget) => widget.render(
                &RenderCtx::new(false, &self.layout, &self.state),
                &mut widget_screen,
            ),
            Err(_) => return,
        };

        // Draw widget onto background screen
        screen.draw_from_screen(&widget_screen, layout.x as usize, layout.y as usize);

        self.rendered.insert(
            widget,
            (
                layout.clone(),
                inner_widgets
                    .as_ref()
                    .map(|w| w.iter().map(|w| w.1).collect()),
            ),
        );

        if let Some(inner_widgets) = inner_widgets {
            for (inner_layout, inner_widget) in inner_widgets.into_iter() {
                self.render_recursive(
                    inner_widget,
                    &Rect {
                        x: layout.x + inner_layout.x,
                        y: layout.y + inner_layout.y,
                        width: inner_layout.width,
                        height: inner_layout.height,
                    },
                    screen,
                );
            }
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
            let leaf_layout = self.layout.layout(node).unwrap().clone();
            let leaf_widget = node.widget(&self.layout).unwrap();
            self.render_recursive(leaf_widget, &leaf_layout, &mut screen);
        }

        // Draw contents of background screen to terminal
        self.term.draw_from_screen(&screen, 0, 0);

        if let Some(focus) = self.focus {
            let layout = self.rendered.get(focus).map(|v| &v.0).unwrap();
            if let Some(cursor) = focus
                .get(&self.layout)
                .unwrap()
                .read()
                .unwrap()
                .cursor(&self.layout)
            {
                self.term.add_changes(vec![
                    Change::CursorVisibility(CursorVisibility::Visible),
                    Change::CursorPosition {
                        x: Position::Absolute(layout.x as usize + cursor.0),
                        y: Position::Absolute(layout.y as usize + cursor.1),
                    },
                ]);
            } else {
                self.term
                    .add_changes(vec![Change::CursorVisibility(CursorVisibility::Hidden)]);
            }
        }

        // Compute optimized diff and flush
        self.term
            .flush()
            .map_err(|_| Error::external("could not flush terminal"))?;

        Ok(())
    }
}
