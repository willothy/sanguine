use std::collections::HashMap;

use termwiz::{
    input::InputEvent,
    terminal::{buffered::BufferedTerminal, Terminal},
};

use crate::{layout::Rect, widget::Widget};

pub struct Stack<T: Terminal> {
    layers: HashMap<usize, Vec<Box<dyn Widget<T>>>>,
}

impl<T: Terminal> Stack<T> {
    pub fn new() -> Self {
        Self {
            layers: HashMap::new(),
        }
    }

    pub fn add_or_extend_layer(&mut self, layer: usize, widgets: Vec<Box<dyn Widget<T>>>) {
        self.layers.entry(layer).or_insert(vec![]).extend(widgets);
    }

    pub fn add_widget(&mut self, layer: usize, widget: Box<dyn Widget<T>>) {
        self.layers.entry(layer).or_insert(vec![]).push(widget);
    }

    pub fn with_layer(mut self, z_index: usize, widget: Box<dyn Widget<T>>) -> Box<Self> {
        self.add_widget(z_index, widget);
        Box::new(self)
    }
}

impl<T: Terminal> Widget<T> for Stack<T> {
    fn render(&self, rect: &Rect, term: &mut BufferedTerminal<T>) {
        use itertools::sorted;

        for layer in sorted(self.layers.keys()) {
            for widget in self.layers.get(layer).unwrap() {
                widget.render(rect, term);
            }
        }
    }

    fn handle_event(&mut self, event: &InputEvent) {
        use itertools::sorted;
        // should be fine to clone just the keys
        // reverse so that the top layer is handled first
        for layer in sorted(self.layers.keys().cloned()).rev() {
            for widget in self.layers.get_mut(&layer).unwrap() {
                widget.handle_event(event);
            }
        }
    }
}
