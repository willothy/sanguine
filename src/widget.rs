use termwiz::{input::InputEvent, surface::Surface};

use crate::layout::{
    geometry::{Rect, SizeHint},
    tree::Layout,
};

pub trait Widget {
    fn render(&self, layout: &Layout, bounds: Rect, surface: &mut Surface);
    fn update(&mut self, _event: InputEvent) {}
    fn size_hint(&self) -> SizeHint {
        SizeHint::Fill
    }
}
