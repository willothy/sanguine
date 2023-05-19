use std::sync::{mpsc::Sender, Arc, RwLock};

use termwiz::{
    input::MouseEvent,
    surface::{Change, Position, Surface},
};

use crate::{layout::Rect, widget::Widget, Event};

pub struct Border {
    title: String,
    inner: Arc<RwLock<dyn Widget>>,
}

impl Border {
    pub fn new(title: String, inner: impl Widget + 'static) -> Self {
        Self {
            title,
            inner: Arc::new(RwLock::new(inner)),
        }
    }
}

const HORIZONTAL: char = '─';
const VERTICAL: char = '│';
const TOP_LEFT: char = '┌';
const TOP_RIGHT: char = '┐';
const BOTTOM_LEFT: char = '└';
const BOTTOM_RIGHT: char = '┘';

impl Widget for Border {
    fn render(
        &self,
        layout: &crate::layout::Layout,
        surface: &mut termwiz::surface::Surface,
        focused: bool,
    ) {
        let (width, height) = surface.dimensions();
        let mut changes = vec![];
        changes.push(Change::Text(TOP_LEFT.to_string()));
        let title = if focused {
            self.title.clone() + "*"
        } else {
            self.title.clone()
        };
        changes.push(Change::Text(title.to_owned()));
        for _ in 0..(width - 1 - title.len()) {
            changes.push(Change::Text(HORIZONTAL.to_string()));
        }
        changes.push(Change::CursorPosition {
            x: Position::Absolute(width - 1),
            y: Position::Relative(0),
        });
        changes.push(Change::Text(TOP_RIGHT.to_string()));
        for _ in 0..(height - 1) {
            changes.push(Change::CursorPosition {
                x: Position::Absolute(0),
                y: Position::Relative(1),
            });
            changes.push(Change::Text(VERTICAL.to_string()));
            changes.push(Change::CursorPosition {
                x: Position::Absolute(width - 1),
                y: Position::Relative(0),
            });
            changes.push(Change::Text(VERTICAL.to_string()));
        }
        changes.push(Change::CursorPosition {
            x: Position::Absolute(0),
            y: Position::Absolute(height - 1),
        });
        changes.push(Change::Text(BOTTOM_LEFT.to_string()));
        for _ in 0..(width - 1) {
            changes.push(Change::Text(HORIZONTAL.to_string()));
        }
        changes.push(Change::CursorPosition {
            x: Position::Absolute(width - 1),
            y: Position::Relative(0),
        });
        changes.push(Change::Text(BOTTOM_RIGHT.to_string()));

        surface.add_changes(changes);

        // Draw inner widget
        let mut inner_screen = Surface::new(width - 2, height - 2);
        self.inner
            .read()
            .unwrap()
            .render(layout, &mut inner_screen, focused);
        surface.draw_from_screen(&inner_screen, 1, 1);
    }

    fn cursor(&self) -> Option<(usize, usize)> {
        self.inner
            .read()
            .unwrap()
            .cursor()
            .map(|(x, y)| (x + 1, y + 1))
    }

    fn update(&mut self, rect: &Rect, mut event: Event, exit_tx: Arc<Sender<()>>) {
        let rect = Rect {
            x: rect.x + 1.,
            y: rect.y + 1.,
            width: rect.width - 2.,
            height: rect.height - 2.,
        };

        match &mut event {
            Event::Input(evt) => match evt {
                termwiz::input::InputEvent::Mouse(MouseEvent {
                    x,
                    y,
                    mouse_buttons: _,
                    modifiers: _,
                }) => {
                    *x -= 2;
                    *y -= 2;
                }
                _ => {}
            },
            _ => {}
        }

        self.inner.write().unwrap().update(&rect, event, exit_tx);
    }
}
