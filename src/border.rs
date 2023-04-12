use termwiz::{
    input::InputEvent,
    surface::{Change, Position},
    terminal::{buffered::BufferedTerminal, Terminal},
};

use crate::{layout::Rect, widget::Widget};

pub struct BorderChars {
    pub top_left: char,
    pub top_right: char,
    pub bottom_left: char,
    pub bottom_right: char,
    pub horizontal: char,
    pub vertical: char,
}

pub enum BorderVariant {
    Single,
    Double,
    Rounded,
    Custom(BorderChars),
    None,
}

impl From<BorderVariant> for BorderChars {
    fn from(border: BorderVariant) -> Self {
        match border {
            BorderVariant::Single => BorderChars {
                top_left: '┌',
                top_right: '┐',
                bottom_left: '└',
                bottom_right: '┘',
                horizontal: '─',
                vertical: '│',
            },
            BorderVariant::Double => BorderChars {
                top_left: '╔',
                top_right: '╗',
                bottom_left: '╚',
                bottom_right: '╝',
                horizontal: '═',
                vertical: '║',
            },
            BorderVariant::Rounded => BorderChars {
                top_left: '╭',
                top_right: '╮',
                bottom_left: '╰',
                bottom_right: '╯',
                horizontal: '─',
                vertical: '│',
            },
            BorderVariant::Custom(chars) => chars,
            BorderVariant::None => BorderChars {
                top_left: ' ',
                top_right: ' ',
                bottom_left: ' ',
                bottom_right: ' ',
                horizontal: ' ',
                vertical: ' ',
            },
        }
    }
}

pub struct Border<T: Terminal> {
    pub chars: BorderChars,
    pub inner: Box<dyn Widget<T>>,
}

impl<T: Terminal> Border<T> {
    pub fn new(border: BorderVariant, inner: Box<dyn Widget<T>>) -> Self {
        Self {
            chars: border.into(),
            inner,
        }
    }
}

impl<T: Terminal> Widget<T> for Border<T> {
    fn render(&self, rect: &Rect, term: &mut BufferedTerminal<T>) {
        let mut inner_rect = rect.clone();
        inner_rect.x += 1.0;
        inner_rect.y += 1.0;
        inner_rect.width -= 2.0;
        inner_rect.height -= 2.0;
        term.add_change(Change::CursorPosition {
            x: Position::Absolute(rect.x.floor() as usize),
            y: Position::Absolute(rect.y.floor() as usize),
        });
        term.add_change(Change::Text(self.chars.top_left.to_string()));
        for _ in 0..(rect.width - 2.0) as usize {
            term.add_change(Change::Text(self.chars.horizontal.to_string()));
        }
        term.add_change(Change::Text(self.chars.top_right.to_string()));
        for _ in 0..(rect.height - 2.0) as usize {
            term.add_change(Change::CursorPosition {
                x: Position::Absolute(rect.x.floor() as usize),
                y: Position::Relative(1),
            });
            term.add_change(Change::Text(self.chars.vertical.to_string()));
            term.add_change(Change::CursorPosition {
                x: Position::Absolute((rect.x + rect.width - 1.0).floor() as usize),
                y: Position::Relative(0),
            });
            term.add_change(Change::Text(self.chars.vertical.to_string()));
        }
        term.add_change(Change::CursorPosition {
            x: Position::Absolute(rect.x.floor() as usize),
            y: Position::Absolute((rect.y + rect.height - 1.0).floor() as usize),
        });
        term.add_change(Change::Text(self.chars.bottom_left.to_string()));
        for _ in 0..(rect.width - 1.0) as usize {
            term.add_change(Change::Text(self.chars.horizontal.to_string()));
        }
        term.add_change(Change::CursorPosition {
            x: Position::Absolute((rect.x + rect.width - 1.0).floor() as usize),
            y: Position::Relative(0),
        });
        term.add_change(Change::Text(self.chars.bottom_right.to_string()));
        self.inner.render(&inner_rect, term);
    }

    fn handle_event(&mut self, _event: &InputEvent) {
        // nothing
    }
}
