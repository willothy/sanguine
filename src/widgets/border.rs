//! Displays a border around a widget, with a title and a `*` when the widget is focused.

use std::sync::{Arc, RwLock};

use crate::{
    error::Error,
    event::Event,
    layout::{Rect, WidgetId},
    surface::*,
    widget::{RenderCtx, UpdateCtx},
    Layout, Widget,
};

/// Displays a border around a widget, with a title and a `*` when the widget is focused.
pub struct Border<U, S> {
    title: String,
    inner: WidgetId,
    #[doc(hidden)]
    __phantom: std::marker::PhantomData<(U, S)>,
}

impl<U, S> Border<U, S> {
    pub fn from_inner(
        title: impl Into<String>,
        inner: impl Widget<U, S> + 'static,
        layout: &mut Layout<U, S>,
    ) -> Self {
        let inner = layout.register_widget(inner);
        Self {
            title: title.into(),
            inner,
            __phantom: std::marker::PhantomData,
        }
    }

    pub fn new(title: impl Into<String>, inner: WidgetId) -> Self {
        Self {
            title: title.into(),
            inner,
            __phantom: std::marker::PhantomData,
        }
    }
}

const HORIZONTAL: char = '─';
const VERTICAL: char = '│';
const TOP_LEFT: char = '┌';
const TOP_RIGHT: char = '┐';
const BOTTOM_LEFT: char = '└';
const BOTTOM_RIGHT: char = '┘';

impl<U, S> Widget<U, S> for Border<U, S> {
    fn render<'r>(
        &self,
        cx: &RenderCtx<'r, U, S>,
        surface: &mut Surface,
    ) -> Option<Vec<(Rect, WidgetId)>> {
        let (width, height) = surface.dimensions();
        let mut changes = vec![];
        changes.push(Change::Text(TOP_LEFT.to_string()));
        let title = if cx.focused {
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

    fn cursor(&self, layout: &Layout<U, S>) -> Option<(usize, usize)> {
        self.inner
            .get(layout)
            .unwrap()
            .read()
            .unwrap()
            .cursor(layout)
            .map(|(x, y)| (x, y))
    }

    fn update<'u>(
        &mut self,
        cx: &mut UpdateCtx<'u, U, S>,
        event: Event<U>,
    ) -> crate::error::Result<()> {
        let rect = Rect {
            x: cx.bounds.x + 1.,
            y: cx.bounds.y + 1.,
            width: cx.bounds.width - 2.,
            height: cx.bounds.height - 2.,
        };

        self.inner
            .get(&cx.layout)
            .unwrap()
            .write()
            .map_err(|_| Error::external("could not lock widget"))?
            .update(&mut cx.with_rect(rect), event)?;
        Ok(())
    }
}
