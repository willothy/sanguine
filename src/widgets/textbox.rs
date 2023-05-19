use termwiz::{
    input::{InputEvent, KeyCode, KeyEvent, Modifiers, MouseButtons, MouseEvent},
    surface::{Change, Position},
};

use crate::{layout::Rect, widget::Widget, Event};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    x: usize,
    y: usize,
}

pub struct TextBox {
    buf: Vec<String>,
    cursor: Cursor,
}

impl TextBox {
    pub fn new() -> Self {
        Self {
            buf: vec![String::new()],
            cursor: Cursor { x: 0, y: 0 },
        }
    }

    fn write_char(&mut self, c: char) {
        let line = self.buf.get(self.cursor.y).unwrap();
        if self.cursor.x >= line.len() {
            self.buf.get_mut(self.cursor.y).unwrap().push(c);
        } else {
            let mut new_line = String::new();
            new_line.push_str(&line[0..self.cursor.x]);
            new_line.push(c);
            new_line.push_str(&line[self.cursor.x..]);
            *self.buf.get_mut(self.cursor.y).unwrap() = new_line;
        }
        self.cursor.x += 1;
    }

    fn delete(&mut self) {
        // backspace
        if self.cursor.x == 0 && self.cursor.y == 0 {
            return;
        }

        if self.cursor.x == 0 {
            let line = self.buf.remove(self.cursor.y);
            let prev_line = self.buf.get_mut(self.cursor.y - 1).unwrap();
            let old_len = prev_line.len();
            prev_line.push_str(&line);
            self.cursor.y -= 1;
            self.cursor.x = old_len;
        } else {
            let line = self.buf.get_mut(self.cursor.y).unwrap();
            let mut new_line = String::new();
            new_line.push_str(&line[0..self.cursor.x - 1]);
            new_line.push_str(&line[self.cursor.x..]);
            *self.buf.get_mut(self.cursor.y).unwrap() = new_line;
            self.cursor.x -= 1;
        }
    }

    fn set_cursor_x(&mut self, x: usize) {
        let line = self.buf.get(self.cursor.y).map(|l| l.len()).unwrap_or(0);
        if x >= line {
            self.cursor.x = line;
        } else {
            self.cursor.x = x;
        }
    }

    fn set_cursor_y(&mut self, y: usize) {
        let nlines = self.buf.len();
        if y >= nlines {
            self.cursor.y = nlines - 1;
        } else {
            self.cursor.y = y;
        }
        let len = self.buf.get(self.cursor.y).map(|l| l.len()).unwrap_or(0);
        if self.cursor.x > len {
            self.cursor.x = len;
        }
    }

    fn set_cursor(&mut self, x: usize, y: usize) {
        self.set_cursor_y(y);
        self.set_cursor_x(x);
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
            .iter()
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

    fn cursor(&self) -> Option<(usize, usize)> {
        Some((self.cursor.x, self.cursor.y))
    }

    fn update(
        &mut self,
        layout: &Rect,
        event: Event,
        _: std::sync::Arc<std::sync::mpsc::Sender<()>>,
    ) {
        match event {
            Event::Input(InputEvent::Key(KeyEvent { key, modifiers })) => {
                if modifiers == Modifiers::NONE || modifiers == Modifiers::SHIFT {
                    match key {
                        KeyCode::Char(c) => self.write_char(c),
                        KeyCode::Enter => {
                            if self.cursor.x == self.buf.get(self.cursor.y).unwrap().len() {
                                self.buf.insert(self.cursor.y + 1, String::new());
                            } else {
                                let line = self.buf.get_mut(self.cursor.y).unwrap();
                                let new_line = line.drain(self.cursor.x..).collect::<String>();

                                if self.cursor.y == self.buf.len() {
                                    self.buf.push(new_line);
                                } else {
                                    self.buf.insert(self.cursor.y + 1, new_line);
                                }
                            }
                            self.set_cursor(0, self.cursor.y + 1);
                        }
                        KeyCode::Tab => {
                            self.write_char(' ');
                            self.write_char(' ');
                        }
                        KeyCode::UpArrow => {
                            self.set_cursor_y(self.cursor.y.saturating_sub(1));
                        }
                        KeyCode::DownArrow => {
                            self.set_cursor_y(
                                self.cursor.y.saturating_add(1).min(layout.height as usize),
                            );
                        }
                        KeyCode::LeftArrow => {
                            self.set_cursor_x(self.cursor.x.saturating_sub(1));
                        }
                        KeyCode::RightArrow => {
                            self.set_cursor_x(
                                self.cursor.x.saturating_add(1).min(layout.width as usize),
                            );
                        }
                        KeyCode::Backspace => {
                            self.delete();
                        }
                        _ => {}
                    }
                }
            }
            Event::Input(InputEvent::Mouse(MouseEvent {
                x,
                y,
                mouse_buttons,
                modifiers: _,
            })) => {
                if mouse_buttons == MouseButtons::LEFT {
                    self.set_cursor(x as usize, y as usize);
                }
            }
            _ => {}
        }
    }
}
