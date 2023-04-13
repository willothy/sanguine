use float::{Float, FloatStack};
use std::collections::VecDeque;
use termwiz::{
    caps::{Capabilities, ProbeHints},
    input::{InputEvent, KeyCode, KeyEvent, Modifiers, MouseEvent},
    surface::{Change, Surface},
    terminal::{buffered::BufferedTerminal, new_terminal, Terminal},
};

pub mod align;
pub mod border;
pub mod float;
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

pub use anyhow;
use anyhow::Result;

pub struct Ui<T: Terminal> {
    pub root: Box<dyn Widget>,
    pub buffer: BufferedTerminal<T>,
    pub floats: FloatStack,
    pub queue: VecDeque<InputEvent>,
    pub rect: Rect,
    pub current_float: Option<(usize, Rect)>,
}

impl<T: Terminal> Ui<T> {
    pub fn new(root: Box<dyn Widget>, surface: BufferedTerminal<T>) -> Result<Self> {
        Ok(Self {
            root,
            buffer: surface,
            floats: FloatStack::new(),
            queue: VecDeque::new(),
            rect: Rect::new(0., 0., 0., 0.),
            current_float: None,
        })
    }

    pub fn init(&mut self) -> Result<()> {
        self.buffer.terminal().set_raw_mode()?;
        self.buffer
            .add_change(Change::ClearScreen(Default::default()));
        let (w, h) = self.buffer.dimensions();
        self.rect = Rect::new(0., 0., w as f64, h as f64);
        Ok(())
    }

    pub fn add_float(&mut self, float: Float) {
        let rect = float.rect.clone();
        self.current_float = Some((self.floats.add(float), rect));
    }

    pub fn cycle_float(&mut self) {
        let count = self.floats.floats.len();
        if count == 0 {
            self.current_float = None;
        } else if let Some((id, rect)) = &self.current_float {
            if *id == count - 1 {
                let first = self.floats.floats.first_key_value();
                if let Some(f) = first {
                    self.current_float = Some((*f.0, f.1.rect.clone()));
                } else {
                    self.current_float = None;
                }
            } else {
                let next = self.floats.floats.get(&(id + 1));
                self.current_float = next.map(|f| (*id + 1, f.rect.clone()));
            }
        } else {
            let first = self.floats.floats.first_key_value();
            if let Some(f) = first {
                self.current_float = Some((*f.0, f.1.rect.clone()));
            } else {
                self.current_float = None;
            }
        }
    }

    pub fn resize(&mut self, rows: usize, cols: usize) {
        self.buffer
            .add_change(Change::ClearScreen(Default::default()));
        self.buffer.resize(cols, rows);
        self.rect.resize(cols, rows);
        // TODO: update float positions to keep them on screen
    }

    pub fn render(&mut self) -> Result<bool> {
        // TODO: Only clear old float locations, not whole screen
        self.buffer
            .add_change(Change::ClearScreen(Default::default()));
        if let Some(curr) = &self.current_float {
            self.floats.update(curr.0, curr.1.clone());
        }
        self.root.render(&self.rect, &mut self.buffer);
        self.floats.render(&self.rect, &mut self.buffer);
        self.buffer.flush()?;

        match self.buffer.terminal().poll_input(None) {
            Ok(Some(InputEvent::Resized { rows, cols })) => self.resize(rows, cols),
            Ok(Some(input)) => match input {
                InputEvent::Key(KeyEvent {
                    key: KeyCode::Char(c),
                    modifiers,
                }) => {
                    if c == 'q' && modifiers == Modifiers::CTRL {
                        // Quit the app when q is pressed
                        self.buffer
                            .add_change(Change::ClearScreen(Default::default()));
                        self.buffer.add_change(Change::CursorVisibility(
                            termwiz::surface::CursorVisibility::Visible,
                        ));
                        self.buffer.flush()?;
                        // break;
                        return Ok(false);
                    }
                }
                InputEvent::Key(KeyEvent {
                    key: KeyCode::UpArrow,
                    modifiers,
                }) => {
                    if let Some((id, rect)) = &mut self.current_float {
                        if modifiers == Modifiers::SHIFT {
                            rect.height -= 1.;
                        } else {
                            rect.y -= 1.;
                        }
                    }
                }
                InputEvent::Key(KeyEvent {
                    key: KeyCode::DownArrow,
                    modifiers,
                }) => {
                    if let Some((id, rect)) = &mut self.current_float {
                        if modifiers == Modifiers::SHIFT {
                            rect.height += 1.;
                        } else {
                            rect.y += 1.;
                        }
                    }
                }
                InputEvent::Key(KeyEvent {
                    key: KeyCode::LeftArrow,
                    modifiers,
                }) => {
                    if let Some((id, rect)) = &mut self.current_float {
                        if modifiers == Modifiers::SHIFT {
                            rect.width -= 1.;
                        } else {
                            rect.x -= 1.;
                        }
                    }
                }
                InputEvent::Key(KeyEvent {
                    key: KeyCode::RightArrow,
                    modifiers,
                }) => {
                    if let Some((id, rect)) = &mut self.current_float {
                        if modifiers == Modifiers::SHIFT {
                            rect.width += 1.;
                        } else {
                            rect.x += 1.;
                        }
                    }
                }
                InputEvent::Key(KeyEvent {
                    key: KeyCode::Tab,
                    modifiers,
                }) => {
                    if modifiers == Modifiers::SHIFT {
                        // let t = cycle_float(main_float, floats.floats.len());
                        // let rect = &floats.floats[&t].rect;
                        // float_x = rect.x as usize;
                        // float_y = rect.y as usize;
                        // float_width = rect.width as usize;
                        // float_height = rect.height as usize;
                        // // floats.update_z_index(main_float, );
                        // main_float = t;
                        self.cycle_float();
                    } else {
                        // ui = cycle_layout();
                    }
                }
                #[allow(unused_variables)]
                InputEvent::Mouse(MouseEvent {
                    x,
                    y,
                    mouse_buttons,
                    modifiers,
                }) => {
                    // Hacky fix for mouse events registering one row too low
                    let y = y - 1;
                    // TODO: Feed input into the Ui
                    // Get widget under mouse
                    // Send input to widget
                }
                _input @ _ => {
                    // TODO: Feed input into the Ui
                    // Get focused widget
                    // Send input to widget
                }
            },
            Ok(None) => {}
            Err(e) => {
                print!("{:?}\r\n", e);
                // break;
                // return Err(e);
            }
        }
        Ok(true)
    }
}
