use termwiz::{
    input::InputEvent,
    terminal::{buffered::BufferedTerminal, Terminal},
};

use crate::layout::Rect;

pub trait Widget<T: Terminal> {
    fn render(&self, rect: &Rect, term: &mut BufferedTerminal<T>);
    fn handle_event(&mut self, event: &InputEvent);
}
