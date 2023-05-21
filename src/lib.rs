//! # Sanguine
//!
//! A library for building terminal applications in Rust.
//!
//! Sanguine provides a layout engine and a set of widgets that can be used to build dynamic,
//! layout-based applications such as text editors and other complex tui applications.
//!
//! ## Example:
//!
//! > Note: Check out the demo by cloning [the repo](https://github.com/willothy/sanguine) and running `cargo run --example demo` from the crate root.
//!
//! ```rust
//! use sanguine::prelude::*;
//!
//! pub fn main() -> Result<()> {
//!     // Create the layout struct
//!     let mut layout = Layout::new();
//!
//!     // Create two TextBox widgets, wrapped by Border widgets
//!     let editor_1 = Border::new("textbox 1".to_owned(), TextBox::new());
//!     let editor_2 = Border::new("textbox 2".to_owned(), TextBox::new());
//!
//!     // Add the first editor to the layout
//!     let left = layout.add_leaf(editor_1);
//!
//!     // Clone the first editor to add it to the layout again
//!     // This widget will be *shared* between the two windows, meaning that changes to the underlying
//!     // buffer will be shown in both windows and focusing on either window will allow you to edit
//!     // the same buffer.
//!     let top_right = layout.clone_leaf(left);
//!
//!     // Add the second editor to the layout
//!     let bot_right = layout.add_leaf(editor_2);
//!
//!     // Create a container to hold the two right hand side editors
//!     let right = layout.add_with_children(
//!         // The container will be a vertical layout
//!         Axis::Vertical,
//!         // The container will take up all available space
//!         Some(SizeHint::fill()),
//!         // The container will contain the cloned first editor, and the second editor
//!         [top_right, bot_right],
//!     );
//!
//!     // Get the root node of the layout
//!     let root = layout.root();
//!     // Ensure that the root container is laid out horizontally
//!     layout.set_direction(root, Axis::Horizontal);
//!
//!     // Add the left window (leaf) and the right container to the root
//!     layout.add_child(root, left);
//!     layout.add_child(root, right);
//!
//!     // Create the sanguine app, providing a handler for *global* input events.
//!     // In this case, we only handle occurrences of Shift+Tab, which we use to cycle focus.
//!     // If Shift+Tab is pressed, we return true to signal that the event should not be
//!     // propagated.
//!     let mut app = App::with_global_handler(
//!         layout,
//!         // The default config is fine for this example
//!         Config::default(),
//!         |state: &mut App, event: &Event, _| {
//!             if let Event::Input(InputEvent::Key(KeyEvent {
//!                 key: KeyCode::Tab,
//!                 modifiers: Modifiers::SHIFT,
//!             })) = event
//!             {
//!                 state.cycle_focus()?;
//!                 return Ok(true);
//!             }
//!             Ok(false)
//!         },
//!     )?;
//!     // Set the initial focus to the left node.
//!     // Only windows can be focused, attempting to focus a container will throw an error.
//!     app.set_focus(left)?;
//!
//!     // The main render loop, which will run until the user closes the application (defaults to Ctrl-q).
//!     while app.handle_events()? {
//!         app.render()?;
//!     }
//!
//!     Ok(())
//! }
//!
//! ```

use std::{
    sync::{atomic::AtomicBool, mpsc::Sender, Arc, RwLock},
    time::Duration,
    unreachable,
};

use allocator::*;
use error::{Error, Result};
use event::*;
use layout::*;
use surface::{term::*, *};
pub use widget::Widget;

/// Re-exports from termwiz relating to `termwiz::surface::Surface`
pub mod surface {
    pub use termwiz::surface::{Change, CursorShape, CursorVisibility, Position, Surface};
    pub use termwiz::terminal::Terminal;

    pub mod term {
        pub use termwiz::caps::Capabilities;
        pub use termwiz::terminal::{buffered::BufferedTerminal, UnixTerminal};
    }
}

/// Types relating to input and event handling
pub mod event {
    pub use termwiz::input::{InputEvent, KeyCode, KeyEvent, Modifiers, MouseButtons, MouseEvent};

    #[derive(Debug)]
    pub enum UserEvent<U> {
        Exit,
        User(U),
    }

    /// An event that can be sent to a widget or handled by the global event handler.
    #[derive(Debug)]
    pub enum Event<U> {
        Input(InputEvent),
        User(UserEvent<U>),
    }
}

/// Commonly used types from Sanguine and termwiz
pub mod prelude {
    pub use crate::allocator::NodeId;
    pub use crate::error::*;
    pub use crate::event::*;
    pub use crate::layout::*;
    pub use crate::surface::{Change, Position, Surface, Terminal};
    pub use crate::widgets::{Border, TextBox};
    pub use crate::{App, Config};
}

mod allocator;
pub mod error;
pub mod layout;
mod widget;
pub mod widgets;

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

pub type GlobalHandler<U> =
    dyn Fn(&mut App<U>, &Event<U>, Arc<Sender<UserEvent<U>>>) -> Result<bool>;

/// The main application struct, responsible for managing the layout tree,
/// keeping track of focus, and rendering the widgets.
pub struct App<U = ()> {
    /// The layout tree
    layout: Layout<U>,
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
    global_event_handler: Box<GlobalHandler<U>>,
    /// Configuration struct
    config: Config,
}

impl<U> Drop for App<U> {
    fn drop(&mut self) {
        // Restore cursor visibility and leave alternate screen when app exits
        self.term
            .add_change(Change::CursorVisibility(CursorVisibility::Visible));
        self.term.terminal().exit_alternate_screen().unwrap();
    }
}

impl<U: 'static> App<U> {
    fn render_ctx(&self, node: NodeId) -> Result<(Arc<RwLock<dyn Widget<U>>>, &Rect)> {
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
            if let Event::Input(InputEvent::Key(KeyEvent {
                key: KeyCode::Char('q'),
                modifiers: Modifiers::CTRL,
            })) = event
            {
                self.event_tx
                    .send(UserEvent::Exit)
                    .map_err(|_| Error::SignalSendFail)?
            }
        }

        // Safety: The function pointer is stored in self so the borrow checker doesn't like
        // us calling it with a mutable reference to self. However, the function pointer won't be changed
        // so it should be safe to call with a mutable reference to self.
        let evt = &self.global_event_handler as *const GlobalHandler<U>;
        unsafe { (*evt)(self, event, self.event_tx.clone()) }
    }

    fn process_event(&mut self, event: Event<U>) -> Result<()> {
        match &event {
            Event::Input(input_event) => match &input_event {
                InputEvent::Resized { cols, rows } => {
                    self.size = Rect::from_size((*cols, *rows));
                    self.term.resize(*cols, *rows);
                    self.term.repaint().map_err(|_| Error::TerminalError)?;
                    self.term.flush().map_err(|_| Error::TerminalError)?;
                    self.layout.mark_dirty();
                }
                InputEvent::Wake => {}
                InputEvent::PixelMouse(_event) => {}
                InputEvent::Mouse(MouseEvent {
                    x,
                    y,
                    mouse_buttons,
                    ..
                }) => {
                    if !self.global_event(&event)? {
                        let Some(node) = self.layout.node_at_pos((*x, *y)) else {
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
                            // If the node under the mouse is the same as the focused node,
                            // send the event to the focused node
                            let (widget, layout) = self.render_ctx(focus)?;

                            let event = match event {
                                Event::Input(InputEvent::Mouse(MouseEvent {
                                    x,
                                    y,
                                    mouse_buttons,
                                    modifiers,
                                })) => Event::Input(InputEvent::Mouse(MouseEvent {
                                    x: x - layout.x as u16,
                                    y: y - layout.y as u16,
                                    mouse_buttons,
                                    modifiers,
                                })),
                                _ => unreachable!(),
                            };

                            widget
                                .write()
                                .map_err(|_| Error::WidgetWriteLockError(focus))?
                                .update(layout, event, self.event_tx.clone())?;
                        } else if *mouse_buttons == MouseButtons::LEFT
                            || self.config.focus_follows_hover
                        {
                            // If there's no focus, focus the node under the mouse
                            self.focus = Some(node);
                        }
                    };
                }
                _ => {
                    // Handle global key events
                    if !self.global_event(&event)? {
                        let Some(focus) = self.focus else {
                            // If there's no focus, we can't do anything
                            return Ok(());
                        };
                        let (widget, layout) = self.render_ctx(focus)?;

                        widget
                            .write()
                            .map_err(|_| Error::WidgetWriteLockError(focus))?
                            .update(layout, event, self.event_tx.clone())?;
                    };
                }
            },
            Event::User(user_event) => {
                match user_event {
                    UserEvent::Exit => {
                        self.exit.store(true, std::sync::atomic::Ordering::SeqCst);
                    }
                    UserEvent::User(_) => {
                        if !self.global_event(&event)? {
                            let Some(focus) = self.focus else {
                                // If there's no focus, we can't do anything
                                return Ok(());
                            };
                            let (widget, layout) = self.render_ctx(focus)?;
                            widget
                                .write()
                                .map_err(|_| Error::WidgetWriteLockError(focus))?
                                .update(layout, event, self.event_tx.clone())?;
                        };
                    }
                }
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
            .poll_input(Some(Duration::from_millis(5)))
            .map_err(|_| Error::PollInputFailed)?
        {
            self.process_event(Event::Input(event))?;
        }
        Ok(())
    }

    /// Calls a closure, passing in a mutable reference to the layout.
    pub fn update_layout<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut Layout<U>) -> R,
        R: Sized,
    {
        f(&mut self.layout)
    }

    /// Calls a closure, passing in an immutable reference to the layout.
    pub fn inspect_layout<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&Layout<U>) -> R,
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
        if !self.layout.is_leaf(node) {
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

    pub fn focus_direction(&mut self, direction: Direction) -> Result<()> {
        let current = self.get_focus().ok_or(Error::NoFocus)?;
        let available = self.inspect_layout(|l| l.adjacent_on_side(current, direction));
        let Some(next) = available.iter().next() else {
            return Ok(());
        };
        self.set_focus(*next)?;
        Ok(())
    }

    /// Render the entire application to the terminal
    pub fn render(&mut self) -> Result<()> {
        self.layout.compute(&self.size);

        // Create temporary background screen
        let mut screen = Surface::new(self.size.width as usize, self.size.height as usize);

        // Retrieve leaves (windows) from layout
        self.layout.leaves().into_iter().for_each(|node| {
            let Ok((widget, layout)) = self.render_ctx(node) else {
                // Do nothing if widget or layout is missing
                // TODO: Log error
                return;
            };

            // Draw onto widget screen for composition
            let mut widget_screen = Surface::new(layout.width as usize, layout.height as usize);

            // Render widget onto widget screen
            let Ok(widget) = widget.read() else {
                return
            };

            if let Some(focus) = self.focus {
                widget.render(&self.layout, &mut widget_screen, node == focus);
            } else {
                widget.render(&self.layout, &mut widget_screen, false);
            }

            // Draw widget onto background screen
            screen.draw_from_screen(&widget_screen, layout.x as usize, layout.y as usize);
        });

        // Draw contents of background screen to terminal
        self.term.draw_from_screen(&screen, 0, 0);

        if let Some(focus) = self.focus {
            let layout = self.layout.layout(focus).unwrap();
            if let Some(cursor) = self.layout.widget(focus).unwrap().read().unwrap().cursor() {
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

    /// Create a new Sanguine application with the provided layout and no global event handler.
    pub fn new(layout: Layout<U>, config: Config) -> Result<Self> {
        let term = Capabilities::new_from_env()
            .and_then(|caps| {
                UnixTerminal::new(caps).and_then(|mut t| {
                    t.set_raw_mode()?;
                    t.enter_alternate_screen()?;
                    BufferedTerminal::new(t)
                })
            })
            .map_err(|_| Error::TerminalError)?;
        let (exit_tx, exit_rx) = std::sync::mpsc::channel();

        Ok(App {
            global_event_handler: Box::new(|_, _, _| Ok(false)),
            size: Rect::from_size(term.dimensions()),
            event_tx: Arc::new(exit_tx),
            exit: AtomicBool::new(false),
            focus: None,
            layout,
            term,
            event_rx: exit_rx,
            config,
        })
    }

    /// Create a new Sanguine app with the provided global event handler. The global event handler
    /// intercepts events before they are sent to widgets. It can return true to prevent the event
    /// from propagating to widgets, or false to allow propagation.
    pub fn with_global_handler(
        layout: Layout<U>,
        config: Config,
        handler: impl Fn(&mut App<U>, &Event<U>, Arc<Sender<UserEvent<U>>>) -> Result<bool> + 'static,
    ) -> Result<Self> {
        let mut new = Self::new(layout, config)?;
        new.global_event_handler = Box::new(handler);
        Ok(new)
    }
}
