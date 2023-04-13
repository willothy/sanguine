use termwiz::{
    input::InputEvent,
    surface::Surface,
    terminal::{buffered::BufferedTerminal, Terminal},
};

use crate::{layout::Rect, SizeHint};

pub trait Widget {
    /// Render the widget onto the terminal, within the specified rectangle.
    fn render(&self, rect: &Rect, term: &mut Surface);

    /// Constrain the widget, modifying the rectangle if necessary.
    /// Return the new rectangle, or None to apply no constraints.
    fn constrain(&self, rect: &Rect, parent: &Rect) -> Option<Rect> {
        None
    }

    fn size_hint(&self, parent: &Rect) -> SizeHint;

    /// Handle an input event.
    /// Return true if the event was handled, false to propagate to the event parent.
    fn handle_event(&mut self, event: &InputEvent) -> bool {
        false
    }
}
