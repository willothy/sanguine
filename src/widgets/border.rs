//! Displays a border around a widget, with a title and a `*` when the widget is focused.

use std::sync::{mpsc::Sender, Arc, RwLock};

use crate::{
    error::Error,
    event::{Event, UserEvent},
    layout::Rect,
    surface::*,
    Widget,
};

/// Displays a border around a widget, with a title and a `*` when the widget is focused.
pub struct Border<U> {
    title: String,
    inner: Arc<RwLock<dyn Widget<U>>>,
}

impl<U> Border<U> {
    pub fn new(title: impl Into<String>, inner: impl Widget<U> + 'static) -> Self {
        Self {
            title: title.into(),
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

impl<U> Widget<U> for Border<U> {
    fn render(
        &self,
        _layout: &crate::layout::Layout<U>,
        surface: &mut Surface,
        focused: bool,
    ) -> Option<Vec<(Rect, Arc<RwLock<dyn Widget<U>>>)>> {
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
        let inner_rect = Rect {
            x: 1.,
            y: 1.,
            width: (width - 2) as f32,
            height: (height - 2) as f32,
        };
        Some(vec![(inner_rect, self.inner.clone())])
    }

    fn cursor(&self) -> Option<(Option<usize>, usize, usize)> {
        self.inner
            .read()
            .unwrap()
            .cursor()
            .map(|(_, x, y)| (Some(0), x, y))
    }

    fn update(
        &mut self,
        rect: &Rect,
        event: Event<U>,
        exit_tx: Arc<Sender<UserEvent<U>>>,
    ) -> crate::error::Result<()> {
        let rect = Rect {
            x: rect.x + 1.,
            y: rect.y + 1.,
            width: rect.width - 2.,
            height: rect.height - 2.,
        };

        self.inner
            .write()
            .map_err(|_| Error::external("could not lock widget"))?
            .update(&rect, event, exit_tx)?;
        Ok(())
    }
}
