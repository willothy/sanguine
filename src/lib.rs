use std::collections::VecDeque;
use termwiz::{
    input::InputEvent,
    terminal::{buffered::BufferedTerminal, Terminal},
};

pub mod align;
pub mod border;
pub mod label;
pub mod layout;
pub mod stack;
pub mod widget;

pub use align::*;
pub use border::*;
pub use label::*;
pub use layout::*;
pub use stack::*;
pub use widget::*;

pub struct Ui<T: Terminal> {
    pub root: Box<dyn Widget>,
    pub terminal: BufferedTerminal<T>,
    pub queue: VecDeque<InputEvent>,
}
