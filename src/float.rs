use std::collections::BTreeMap;

use termwiz::surface::Surface;

use crate::{Rect, Widget};

pub struct Float {
    pub contents: Box<dyn Widget>,
    pub rect: Rect,
    pub z_index: usize,
}

pub struct FloatStack {
    pub floats: BTreeMap<usize, Float>,
}

impl FloatStack {
    pub fn new() -> Self {
        Self {
            floats: BTreeMap::new(),
        }
    }

    pub fn add(&mut self, mut float: Float) -> usize {
        while self.floats.contains_key(&float.z_index) {
            float.z_index += 1;
        }
        let z = float.z_index;
        self.floats.insert(float.z_index, float);
        z
    }

    pub fn get_rect(&self, z_index: usize) -> Option<Rect> {
        self.floats.get(&z_index).map(|f| f.rect.clone())
    }

    pub fn update(&mut self, z_index: usize, rect: Rect) {
        if let Some(float) = self.floats.get_mut(&z_index) {
            float.rect = rect;
        }
    }

    pub fn focus(&mut self, z_index: usize) {
        if let Some(float) = self.floats.remove(&z_index) {
            let last = self
                .floats
                .last_entry()
                .map(|e| e.get().z_index + 1)
                .unwrap_or(0);
            self.floats.insert(last, float);
        }
    }

    pub fn remove(&mut self, z_index: usize) {
        self.floats.retain(|_, f| f.z_index != z_index);
    }

    pub fn render(&mut self, rect: &Rect, surface: &mut Surface) {
        self.floats.iter_mut().for_each(|(_, f)| {
            let mut float = Surface::new(f.rect.width as usize, f.rect.height as usize);
            if f.rect.x + f.rect.width > rect.width {
                f.rect.width = rect.width - f.rect.x;
            }
            if f.rect.y + f.rect.height > rect.height {
                f.rect.height = rect.height - f.rect.y;
            }
            f.contents.render(
                &Rect {
                    x: 0.,
                    y: 0.,
                    width: f.rect.width,
                    height: f.rect.height,
                },
                &mut float,
            );
            surface.draw_from_screen(&float, f.rect.x as usize, f.rect.y as usize);
        });
    }
}
