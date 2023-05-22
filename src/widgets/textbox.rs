use std::sync::{Arc, RwLock};

use crate::{
    error::Error,
    error::Result,
    event::{Event, KeyCode, KeyEvent, Modifiers, MouseButtons, MouseEvent},
    layout::Rect,
    surface::{Change, Position, Surface},
    widget::{RenderCtx, UpdateCtx, Widget},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    x: usize,
    y: usize,
}

/// A simple editable textbox widget
pub struct TextBox {
    buf: Arc<RwLock<Vec<String>>>,
    cursor: Cursor,
}

impl Default for TextBox {
    fn default() -> Self {
        Self::new()
    }
}

impl TextBox {
    pub fn new() -> Self {
        Self {
            buf: Arc::new(RwLock::new(vec![String::new()])),
            cursor: Cursor { x: 0, y: 0 },
        }
    }

    pub fn from_str(s: impl Into<String>) -> Self {
        Self {
            buf: Arc::new(RwLock::new(
                s.into().lines().map(|s| s.to_owned()).collect(),
            )),
            cursor: Cursor { x: 0, y: 0 },
        }
    }

    pub fn buffer(&self) -> Arc<RwLock<Vec<String>>> {
        self.buf.clone()
    }

    fn write_char(&mut self, c: char) -> Result<()> {
        let mut writer = self.buf.write().unwrap();
        let line = writer
            .get(self.cursor.y)
            .ok_or(crate::error::Error::TerminalError)?;
        if self.cursor.x >= line.len() {
            writer
                .get_mut(self.cursor.y)
                .ok_or(crate::error::Error::TerminalError)?
                .push(c);
        } else {
            let mut new_line = String::new();
            new_line.push_str(&line[0..self.cursor.x]);
            new_line.push(c);
            new_line.push_str(&line[self.cursor.x..]);
            *writer
                .get_mut(self.cursor.y)
                .ok_or(crate::error::Error::TerminalError)? = new_line;
        }
        self.cursor.x += 1;
        Ok(())
    }

    fn delete(&mut self) -> Result<()> {
        // backspace
        if self.cursor.x == 0 && self.cursor.y == 0 {
            return Ok(());
        }

        if self.cursor.x == 0 {
            let mut writer = self.buf.write().unwrap();
            let line = writer.remove(self.cursor.y);
            let prev_line = writer
                .get_mut(self.cursor.y - 1)
                .ok_or(crate::error::Error::TerminalError)?;
            let old_len = prev_line.len();
            prev_line.push_str(&line);
            self.cursor.y -= 1;
            self.cursor.x = old_len;
        } else {
            let mut writer = self.buf.write().unwrap();
            let line = writer
                .get_mut(self.cursor.y)
                .ok_or(crate::error::Error::TerminalError)?;
            let mut new_line = String::new();
            new_line.push_str(&line[0..self.cursor.x - 1]);
            new_line.push_str(&line[self.cursor.x..]);
            *writer
                .get_mut(self.cursor.y)
                .ok_or(crate::error::Error::TerminalError)? = new_line;
            self.cursor.x -= 1;
        }
        Ok(())
    }

    fn set_cursor_x(&mut self, x: usize) {
        let line = self
            .buf
            .read()
            .unwrap()
            .get(self.cursor.y)
            .map(|l| l.len())
            .unwrap_or(0);
        if x >= line {
            self.cursor.x = line;
        } else {
            self.cursor.x = x;
        }
    }

    fn set_cursor_y(&mut self, y: usize) {
        let nlines = self.buf.read().unwrap().len();
        if y >= nlines {
            self.cursor.y = nlines - 1;
        } else {
            self.cursor.y = y;
        }
        let len = self
            .buf
            .read()
            .unwrap()
            .get(self.cursor.y)
            .map(|l| l.len())
            .unwrap_or(0);
        if self.cursor.x > len {
            self.cursor.x = len;
        }
    }

    fn set_cursor(&mut self, x: usize, y: usize) {
        self.set_cursor_y(y);
        self.set_cursor_x(x);
    }

    fn validate_cursor(&mut self) {
        let nlines = self.buf.read().unwrap().len();
        if self.cursor.y >= nlines {
            self.cursor.y = nlines - 1;
        }
        let len = self
            .buf
            .read()
            .unwrap()
            .get(self.cursor.y)
            .map(|l| l.len())
            .unwrap_or(0);
        if self.cursor.x > len {
            self.cursor.x = len;
        }
    }
}

impl<U, S> Widget<U, S> for TextBox {
    fn render<'r>(
        &self,
        _cx: &RenderCtx<'r, U, S>,
        surface: &mut Surface,
    ) -> Option<Vec<(Rect, Arc<RwLock<dyn Widget<U, S>>>)>> {
        let (width, height) = surface.dimensions();
        self.buf
            .read()
            .unwrap()
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
                surface.add_change(Change::Text(format!("{}", l)));
            });
        None
    }

    fn cursor(&self) -> Option<(Option<usize>, usize, usize)> {
        Some((None, self.cursor.x, self.cursor.y))
    }

    fn update<'u>(
        &mut self,
        _cx: &mut UpdateCtx<'u, U, S>,
        event: Event<U>,
    ) -> crate::error::Result<()> {
        self.validate_cursor();
        match event {
            Event::Key(KeyEvent { key, modifiers }) => {
                if modifiers == Modifiers::NONE || modifiers == Modifiers::SHIFT {
                    match key {
                        KeyCode::Char(c) => self.write_char(c)?,
                        KeyCode::Enter => {
                            if self.cursor.x
                                == self
                                    .buf
                                    .write()
                                    .unwrap()
                                    .get(self.cursor.y)
                                    .ok_or(Error::TerminalError)?
                                    .len()
                            {
                                self.buf
                                    .write()
                                    .unwrap()
                                    .insert(self.cursor.y + 1, String::new());
                            } else {
                                let mut writer = self.buf.write().unwrap();
                                let line =
                                    writer.get_mut(self.cursor.y).ok_or(Error::TerminalError)?;
                                let new_line = line.drain(self.cursor.x..).collect::<String>();

                                if self.cursor.y == writer.len() {
                                    writer.push(new_line);
                                } else {
                                    writer.insert(self.cursor.y + 1, new_line);
                                }
                            }
                            self.set_cursor(0, self.cursor.y + 1);
                        }
                        KeyCode::Tab => {
                            self.write_char(' ')?;
                            self.write_char(' ')?;
                        }
                        KeyCode::UpArrow => {
                            self.set_cursor_y(self.cursor.y.saturating_sub(1));
                        }
                        KeyCode::DownArrow => {
                            let lines = self.buf.read().unwrap().len();
                            self.set_cursor_y(self.cursor.y.saturating_add(1).min(lines));
                        }
                        KeyCode::LeftArrow => {
                            self.set_cursor_x(self.cursor.x.saturating_sub(1));
                        }
                        KeyCode::RightArrow => {
                            self.set_cursor_x(self.cursor.x.saturating_add(1));
                        }
                        KeyCode::Backspace => {
                            self.delete()?;
                        }
                        _ => {}
                    }
                }
                Ok(())
            }
            Event::Mouse(MouseEvent {
                x,
                y,
                mouse_buttons,
                modifiers: _,
            }) => {
                if mouse_buttons == MouseButtons::LEFT {
                    self.set_cursor(x as usize, y as usize);
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}
