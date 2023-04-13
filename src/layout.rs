use termwiz::{
    input::InputEvent,
    surface::Surface,
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

pub struct Layout {
    widgets: Vec<Box<dyn Widget>>,
    axis: LayoutAxis,
}

pub enum SizeHint {
    Fixed(usize),
    Percentage(f64),
}

impl Layout {
    pub fn h(widgets: Vec<Box<dyn Widget>>) -> Box<Self> {
        Box::new(Self {
            widgets,
            axis: LayoutAxis::Horizontal,
        })
    }

    pub fn v(widgets: Vec<Box<dyn Widget>>) -> Box<Self> {
        Box::new(Self {
            widgets,
            axis: LayoutAxis::Vertical,
        })
    }
}

impl Widget for Layout {
    fn render(&self, rect: &Rect, term: &mut Surface) {
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
            let widget_box = Rect {
                x: if self.axis == LayoutAxis::Horizontal {
                    rect.x + width * i as f64
                } else {
                    rect.x
                },
                y: if self.axis == LayoutAxis::Vertical {
                    rect.y + height * i as f64
                } else {
                    rect.y
                },
                width,
                height,
            };
            let rect = if let Some(constraint) = widget.constrain(&widget_box, &rect) {
                constraint
            } else {
                widget_box.clone()
            };
            widget.render(&rect, term)
        });
    }

    // fn handle_event(&mut self, event: &InputEvent) {
    //     self.widgets.iter_mut().for_each(|w| w.handle_event(event));
    // }
}
