//! Types relating to input and event handling

pub use termwiz::input::{KeyCode, KeyEvent, Modifiers, MouseButtons, MouseEvent};

#[derive(Debug)]
pub enum UserEvent<U> {
    Exit,
    Tick,
    User(U),
}

/// An event that can be sent to a widget or handled by the global event handler.
#[derive(Debug)]
pub enum Event<U> {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize { rows: usize, cols: usize },
    Paste(String),
    User(UserEvent<U>),
}
