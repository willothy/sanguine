use std::sync::{Arc, RwLock};

use sanguine::{
    error::*,
    layout::{Axis, Rect},
    style::CellAttributes,
    surface::{Change, Surface},
    widgets::{Border, TextBox},
    App, Config, RenderCtx, Widget,
};
use termimad::MadSkin;

struct MarkdownPreview {
    buf: Arc<RwLock<Vec<String>>>,
}

impl MarkdownPreview {
    pub fn new(buf: Arc<RwLock<Vec<String>>>) -> Self {
        Self { buf }
    }
}

impl<U, S> Widget<U, S> for MarkdownPreview {
    fn render<'r>(
        &self,
        _: &'r RenderCtx<'r, U, S>,
        surface: &'r mut Surface,
    ) -> Option<Vec<(Rect, Arc<RwLock<dyn Widget<U, S>>>)>> {
        let skin = MadSkin::default_dark();
        let dims = surface.dimensions();
        let text = skin
            .text(&self.buf.read().unwrap().join("\n"), Some(dims.0))
            .to_string();

        sanguine::ansi::write_ansi(surface, text.as_str()).ok()?;
        surface.add_change(Change::AllAttributes(CellAttributes::default()));
        None
    }
}

fn main() -> Result<()> {
    let mut s = App::<()>::new(Config::default())?.with_layout(|layout| {
        let root = layout.root();
        layout.set_direction(root, Axis::Horizontal);

        let textbox = TextBox::new();
        let buf = textbox.buffer();
        let editor = layout.add_leaf(Border::new("Editor".to_owned(), textbox));
        layout.add_child(root, editor);
        let preview = layout.add_leaf(Border::new("Preview".to_owned(), MarkdownPreview::new(buf)));
        layout.add_child(root, preview);
        Some(editor)
    });

    while s.handle_events()? {
        s.render()?;
    }
    Ok(())
}
