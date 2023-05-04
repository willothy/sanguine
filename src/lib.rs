use std::collections::VecDeque;

use anyhow::Result;
use layout::{geometry::Rect, tree::Layout};
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

    pub fn render(&mut self) -> Result<()> {
        self.layout.compute(&self.size);

        // Create temporary background screen
        let mut screen = Surface::new(self.size.width as usize, self.size.height as usize);

        // Retrieve leaves (windows) from layout
        let leaves = self.layout.leaves();

        leaves.iter().for_each(|id| {
            // Retrieve computed layout for window
            let layout = self.layout.layout(*id).unwrap();
            // Retrieve widget trait object from node
            let widget = self.layout.widget(*id).unwrap();
            // Draw onto temporary background screen
            widget.render(&self.layout, layout.clone(), &mut screen);
        });

        // Draw contents of background screen to terminal
        self.term.draw_from_screen(&screen, 0, 0);

        // Compute optimized diff and flush
        self.term.flush()?;

        Ok(())
    }
}
