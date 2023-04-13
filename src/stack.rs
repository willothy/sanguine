use std::collections::HashMap;

use termwiz::{
    input::InputEvent,
    surface::{Change, Position, Surface},
    terminal::{buffered::BufferedTerminal, Terminal},
};

use crate::{layout::Rect, widget::Widget};

pub struct Stack {
    layers: HashMap<usize, Vec<Box<dyn Widget>>>,
}

impl Stack {
    pub fn new() -> Self {
        Self {
            layers: HashMap::new(),
        }
    }

    pub fn add_or_extend_layer(&mut self, layer: usize, widgets: Vec<Box<dyn Widget>>) {
        self.layers.entry(layer).or_insert(vec![]).extend(widgets);
    }

    pub fn add_widget(&mut self, layer: usize, widget: Box<dyn Widget>) {
        self.layers.entry(layer).or_insert(vec![]).push(widget);
    }

    pub fn with_layer(mut self, z_index: usize, widget: Box<dyn Widget>) -> Box<Self> {
        self.add_widget(z_index, widget);
        Box::new(self)
    }
}

pub trait Layer {
    fn overlay(&mut self, other: &mut Self);
}

impl Layer for Surface {
    /// Overlay other onto self
    fn overlay(&mut self, other: &mut Self) {
        // let self_s = self.screen_lines();
        // let other_s = other.screen_lines();
        //
        // self_s
        //     .iter()
        //     .zip(other_s.iter())
        //     .for_each(|(self_line, other_line)| {
        //         let mut linenr = 0;
        //         self_line
        //             .visible_cells()
        //             .enumerate()
        //             .zip(other_line.visible_cells())
        //             .for_each(|((idx, self_cell), other_cell)| {
        //                 let blank = self_cell.str().chars().all(|c| c.is_whitespace());
        //                 if !blank {
        //                     // *self_cell = *other_cell;
        //                     let other_cell = other_cell.as_cell();
        //                     self_cell
        //                 }
        //                 linenr += 1;
        //             })
        //     });

        self.add_changes(other.get_changes(other.current_seqno()).1.to_vec());
    }
}

impl Widget for Stack {
    fn render(&self, rect: &Rect, mut surface: &mut Surface) {
        use itertools::sorted;

        // let mut layers = vec![];
        for (i, layer) in sorted(self.layers.keys()).enumerate() {
            // let mut layer_surface = Surface::new(rect.width as usize, rect.height as usize);
            // surface.add_change(Change::ClearScreen(termwiz::color::ColorAttribute::Default));
            for widget in self.layers.get(layer).unwrap() {
                widget.render(rect, &mut surface);
            }
            // surface.draw_from_screen(
            //     &layer_surface,
            //     rect.x.floor() as usize,
            //     rect.y.floor() as usize,
            // );
            // if i == 0 {
            //     surface.draw_from_screen(&layer_surface, 0, 0);
            // } else {
            //     surface.overlay(&mut layer_surface);
            // }
            // surface.overlay(&mut layer_surface);
            // layers.push(layer_surface);
        }
    }

    // fn handle_event(&mut self, event: &InputEvent) {
    //     use itertools::sorted;
    //     // should be fine to clone just the keys
    //     // reverse so that the top layer is handled first
    //     for layer in sorted(self.layers.keys().cloned()).rev() {
    //         for widget in self.layers.get_mut(&layer).unwrap() {
    //             widget.handle_event(event);
    //         }
    //     }
    // }
}
