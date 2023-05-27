use std::sync::{Arc, RwLock};

use sanguine::{
    error::*,
    layout::{Axis, Rect, WidgetId},
    style::CellAttributes,
    surface::{Change, Surface},
    widgets::{Border, TextBox},
    App, RenderCtx, Widget,
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
    ) -> Option<Vec<(Rect, WidgetId)>> {
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
    App::<()>::default()
        .with_layout(|layout, widgets| {
            let root = layout.root();
            layout.set_direction(root, Axis::Horizontal);

            let textbox = TextBox::new();
            let buf = textbox.buffer();

            let textbox_id = widgets.register(textbox);
            let textbox_widget = widgets.register(Border::new("Editor".to_owned(), textbox_id));
            let editor = layout.add_leaf(textbox_widget);
            layout.add_child(root, editor);

            let preview = widgets.register(MarkdownPreview::new(buf));
            let preview =
                layout.add_leaf(widgets.register(Border::new("Preview".to_owned(), preview)));
            layout.add_child(root, preview);

            Some(editor)
        })
        .exec()
}
