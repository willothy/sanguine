use termwiz::{
    input::{InputEvent, KeyCode, KeyEvent, Modifiers},
    surface::{Change, Position},
};

use crate::{widget::Widget, Event};

pub struct TextBox {
    buf: String,
}

impl TextBox {
    pub fn new() -> Self {
        Self { buf: String::new() }
    }
}

impl Widget for TextBox {
    fn render(
        &self,
        _layout: &crate::layout::Layout,
        surface: &mut termwiz::surface::Surface,
        _focused: bool,
    ) {
        let (width, height) = surface.dimensions();
        self.buf
            .lines()
            .into_iter()
            .map(|l| &l[0..width.min(l.len())])
            .enumerate()
            .take(height)
            .for_each(|(i, l)| {
                if i > 0 {
                    surface.add_change(Change::CursorPosition {
                        x: Position::Absolute(0),
                        y: Position::Relative(1),
                    });
                }
                surface.add_change(Change::Text(l.to_string()));
            });
    }

    fn update(&mut self, event: Event, _: std::sync::Arc<std::sync::mpsc::Sender<()>>) {
        match event {
            Event::Input(InputEvent::Key(KeyEvent { key, modifiers })) => {
                if modifiers == Modifiers::NONE || modifiers == Modifiers::SHIFT {
                    match key {
                        KeyCode::Char(c) => self.buf.push(c),
                        KeyCode::Enter => self.buf.push('\n'),
                        KeyCode::Tab => self.buf.push_str("  "),
                        KeyCode::Backspace => {
                            self.buf.pop();
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}
