use termwiz::{
    input::InputEvent,
    terminal::{buffered::BufferedTerminal, Terminal},
};

use crate::widget::Widget;

#[derive(Clone)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, PartialEq, Clone)]
pub enum LayoutAxis {
    Horizontal,
    Vertical,
}

pub struct Layout<T: Terminal> {
    widgets: Vec<Box<dyn Widget<T>>>,
    axis: LayoutAxis,
}

impl<T: Terminal> Layout<T> {
    pub fn h(widgets: Vec<Box<dyn Widget<T>>>) -> Box<Self> {
        Box::new(Self {
            widgets,
            axis: LayoutAxis::Horizontal,
        })
    }

    pub fn v(widgets: Vec<Box<dyn Widget<T>>>) -> Box<Self> {
        Box::new(Self {
            widgets,
            axis: LayoutAxis::Vertical,
        })
    }
}

impl<T: Terminal> Widget<T> for Layout<T> {
    fn render(&self, rect: &Rect, term: &mut BufferedTerminal<T>) {
        let n = self.widgets.len() as f64;
        let width = match self.axis {
            LayoutAxis::Horizontal => rect.width as f64 / n,
            LayoutAxis::Vertical => rect.width,
        };
        let height = match self.axis {
            LayoutAxis::Horizontal => rect.height,
            LayoutAxis::Vertical => rect.height as f64 / n,
        };
        self.widgets.iter().enumerate().for_each(|(i, widget)| {
            widget.render(
                &Rect {
                    x: if self.axis == LayoutAxis::Horizontal {
                        rect.x + width * i as f64
                    } else {
                        rect.x
                    },
                    y: if self.axis == LayoutAxis::Horizontal {
                        rect.y
                    } else {
                        rect.y + height * i as f64
                    },
                    width,
                    height,
                },
                term,
            )
        });
    }

    fn handle_event(&mut self, event: &InputEvent) {
        self.widgets.iter_mut().for_each(|w| w.handle_event(event));
    }
}
