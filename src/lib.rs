use std::{
    collections::VecDeque,
    sync::{Arc, RwLock},
};

use anyhow::{anyhow, Result};
use layout::{
    geometry::Rect,
    tree::{Layout, NodeId},
};
use prelude::Widget;
use termwiz::{
    caps::Capabilities,
    input::InputEvent,
    surface::{Change, Surface},
    terminal::{buffered::BufferedTerminal, UnixTerminal},
};

pub mod layout;
pub mod widget;

pub mod prelude {
    pub use crate::layout::geometry::{Axis, Rect, SizeHint};
    pub use crate::layout::tree::{Container, Layout, LayoutNode, Leaf};
    pub use crate::widget::Widget;
    pub use crate::Sanguine;
}

pub enum Event {
    Input(InputEvent),
    User(String),
}

pub struct Sanguine {
    layout: Layout,
    #[allow(unused)]
    event_queue: VecDeque<Event>,
    term: BufferedTerminal<UnixTerminal>,
    size: Rect,
    focus: Option<NodeId>,
}

impl Drop for Sanguine {
    fn drop(&mut self) {
        self.term.add_change(Change::CursorVisibility(
            termwiz::surface::CursorVisibility::Visible,
        ));
    }
}

impl Sanguine {
    pub fn new(layout: Layout) -> Result<Self> {
        let caps = Capabilities::new_from_env()?;
        let mut term = BufferedTerminal::new(UnixTerminal::new(caps)?)?;
        term.add_change(Change::CursorVisibility(
            termwiz::surface::CursorVisibility::Hidden,
        ));
        Ok(Sanguine {
            event_queue: VecDeque::new(),
            size: {
                let t = term.dimensions();
                Rect {
                    x: 0.,
                    y: 0.,
                    width: t.0 as f32,
                    height: t.1 as f32,
                }
            },
            layout,
            term,
            focus: None,
        })
    }

    pub fn update_layout<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut Layout) -> R,
        R: Sized,
    {
        f(&mut self.layout)
    }

    pub fn inspect_layout<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&Layout) -> R,
        R: Sized,
    {
        f(&self.layout)
    }

    fn render_ctx(&self, node: NodeId) -> Result<(Arc<RwLock<dyn Widget>>, &Rect)> {
        Ok((
            // Retrieve widget trait object from node
            self.layout
                .widget(node)
                .ok_or(anyhow!("Could not find widget"))?,
            // Retrieve computed layout for window
            self.layout
                .layout(node)
                .ok_or(anyhow!("Could not find layout"))?,
        ))
    }

    pub fn process_event(&mut self, event: InputEvent) -> Result<()> {
        match event {
            InputEvent::Resized { cols, rows } => {
                self.size.width = cols as f32;
                self.size.height = rows as f32;
            }
            InputEvent::Mouse(_event) => {}
            InputEvent::Wake => {}
            InputEvent::PixelMouse(_event) => {}
            event => {
                let widget = self
                    .focus
                    .and_then(|node| self.layout.widget(node))
                    .ok_or(anyhow!("Could not find widget"))?;
                widget.write().unwrap().update(event);
            }
        }
        Ok(())
    }

    pub fn set_focus(&mut self, node: NodeId) -> Result<()> {
        if !self.layout.is_leaf(node) {
            return Err(anyhow!("Node is not a leaf"));
        }
        self.focus = Some(node);
        Ok(())
    }

    pub fn get_focus(&self) -> Option<NodeId> {
        self.focus
    }

    pub fn render(&mut self) -> Result<()> {
        self.layout.compute(&self.size);

        // Create temporary background screen
        let mut screen = Surface::new(self.size.width as usize, self.size.height as usize);

        // Retrieve leaves (windows) from layout
        self.layout.leaves().for_each(|node| {
            let Ok((widget, layout)) = self.render_ctx(node) else {
                // Do nothing if widget or layout is missing
                // TODO: Log error
                return;
            };

            // Draw onto widget screen for composition
            let mut widget_screen = Surface::new(layout.width as usize, layout.height as usize);

            // Remove x/y offset for widget-local layout
            let widget_layout = Rect::from_size(layout.width as usize, layout.height as usize);

            // Render widget onto widget screen
            let Ok(widget) = widget.read() else {
                return
            };
            widget.render(&self.layout, widget_layout, &mut widget_screen);

            // Draw widget onto background screen
            screen.draw_from_screen(&widget_screen, layout.x as usize, layout.y as usize);
        });

        // Draw contents of background screen to terminal
        self.term.draw_from_screen(&screen, 0, 0);

        // Compute optimized diff and flush
        self.term.flush()?;

        Ok(())
    }
}
