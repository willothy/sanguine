//! Types relating to input and event handling

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
