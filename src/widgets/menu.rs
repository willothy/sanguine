use std::sync::RwLock;
use std::sync::{mpsc::Sender, Arc};

use termwiz::surface::Surface;

use crate::prelude::*;
use crate::{event::UserEvent, layout::Layout, Widget};
use termwiz::{
    cell::AttributeChange,
    color::{AnsiColor, ColorAttribute},
};

pub trait MenuAction<U>: Fn(usize, &mut Menu<U>, Arc<Sender<UserEvent<U>>>) {}

impl<C, U> MenuAction<U> for C where C: Fn(usize, &mut Menu<U>, Arc<Sender<UserEvent<U>>>) {}

pub struct Menu<U> {
    title: String,
    items: Vec<(String, String, Box<dyn MenuAction<U>>)>,
    active: usize,
}

impl<U> Menu<U> {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            items: vec![],
            active: 0,
        }
    }

    pub fn add_item(
        &mut self,
        title: impl Into<String>,
        tag: impl Into<String>,
        action: impl MenuAction<U> + 'static,
    ) {
        self.items
            .push((title.into(), tag.into(), Box::new(action)));
    }

    pub fn next(&mut self) {
        self.active = (self.active + 1) % self.items.len().max(1);
    }

    pub fn prev(&mut self) {
        self.active = (self.active + self.items.len() - 1) % self.items.len().max(1);
    }

    pub fn select(&mut self, event_tx: Arc<Sender<UserEvent<U>>>) {
        if let Some((_, _, action)) = self.items.get(self.active) {
            let func = action as *const dyn MenuAction<U>;
            unsafe { (*func)(self.active, self, event_tx.clone()) };
        }
    }

    pub fn item(&self, index: usize) -> Option<&(String, String, Box<dyn MenuAction<U>>)> {
        self.items.get(index)
    }

    pub fn tag(&self, index: usize) -> Option<&str> {
        self.items.get(index).map(|(_, tag, _)| tag.as_str())
    }

    pub fn update_tag(&mut self, index: usize, f: impl Fn(&str) -> String) {
        if let Some((_, t, _)) = self.items.get_mut(index) {
            *t = f(t);
        }
    }

    pub fn entry(&self, index: usize) -> Option<&str> {
        self.items.get(index).map(|(title, _, _)| title.as_str())
    }

    pub fn update_entry(&mut self, index: usize, f: impl Fn(&str) -> String) {
        if let Some((t, _, _)) = self.items.get_mut(index) {
            *t = f(t);
        }
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn remove(&mut self, index: usize) {
        self.items.remove(index);
    }

    pub fn update_menu_title(&mut self, f: impl Fn(&str) -> String) {
        self.title = f(&self.title);
    }
}

impl<U> Widget<U> for Menu<U> {
    fn render(
        &self,
        _layout: &Layout<U>,
        surface: &mut Surface,
        _focused: bool,
    ) -> Option<Vec<(Rect, Arc<RwLock<dyn Widget<U>>>)>> {
        let dims = surface.dimensions();
        surface.add_changes(vec![Change::CursorPosition {
            x: Position::Absolute(0),
            y: Position::Relative(0),
        }]);
        let line = format!("{:^width$}", self.title, width = dims.0);
        surface.add_changes(vec![
            Change::Attribute(AttributeChange::Foreground(AnsiColor::Black.into())),
            Change::Attribute(AttributeChange::Background(AnsiColor::White.into())),
            Change::Text(line),
            Change::Attribute(AttributeChange::Foreground(Default::default())),
            Change::Attribute(AttributeChange::Background(Default::default())),
            Change::CursorPosition {
                x: Position::Absolute(0),
                y: Position::Relative(2),
            },
        ]);
        surface.add_changes(vec![]);
        for (i, (item, tag, _)) in self.items.iter().enumerate() {
            if i == self.active {
                surface.add_changes(vec![
                    Change::Attribute(AttributeChange::Foreground(AnsiColor::Black.into())),
                    Change::Attribute(AttributeChange::Background(AnsiColor::White.into())),
                ]);
            }
            let line = format!("{item} {tag}");
            surface.add_changes(vec![
                Change::Text(format!("{:^width$}", line, width = dims.0)),
                Change::CursorPosition {
                    x: Position::Relative(dims.0 as isize),
                    y: Position::Relative(0),
                },
                Change::Attribute(AttributeChange::Foreground(ColorAttribute::Default)),
                Change::Attribute(AttributeChange::Background(ColorAttribute::Default)),
                Change::CursorPosition {
                    x: Position::Absolute(0),
                    y: Position::Relative(1),
                },
            ]);
        }
        None
    }

    fn update(
        &mut self,
        _owner: NodeId,
        _bounds: &Rect,
        _layout: &mut Layout<U>,
        event: Event<U>,
        event_tx: std::sync::Arc<std::sync::mpsc::Sender<UserEvent<U>>>,
    ) -> crate::error::Result<()> {
        match event {
            Event::Key(KeyEvent { key, .. }) => match key {
                KeyCode::UpArrow => self.prev(),
                KeyCode::DownArrow => self.next(),
                KeyCode::Enter => self.select(event_tx),
                _ => {}
            },
            Event::Mouse(MouseEvent {
                y, mouse_buttons, ..
            }) => {
                if mouse_buttons == MouseButtons::LEFT {
                    if (y as usize) <= self.items.len() + 1 && y >= 2 {
                        self.active = y as usize - 2;
                        self.select(event_tx);
                    }
                } else if mouse_buttons == MouseButtons::NONE {
                    if (y as usize) <= self.items.len() + 1 && y >= 2 {
                        self.active = y as usize - 2;
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }
}
