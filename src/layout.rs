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

impl SizeHint {
    pub fn fill() -> SizeHint {
        SizeHint::Percentage(1.0)
    }
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
        use LayoutAxis::*;

        let nwidgets = self.widgets.len();
        let mut available = match self.axis {
            Horizontal => rect.width,
            Vertical => rect.height,
        };

        available = available
            - self
                .widgets
                .iter()
                .filter_map(|w| match w.size_hint(&rect) {
                    SizeHint::Fixed(s) => Some(s as f64),
                    _ => None,
                })
                .sum::<f64>()
            + if nwidgets % 2 != 0 && nwidgets > 1 {
                1.
            } else {
                0.
            };

        self.widgets.iter().enumerate().for_each(|(i, widget)| {
            let (width, height) = match widget.size_hint(&rect) {
                SizeHint::Fixed(s) => match self.axis {
                    Horizontal => (s as f64, rect.height),
                    Vertical => (rect.width, s as f64),
                },
                SizeHint::Percentage(p) => {
                    let p = p * (1.0 / nwidgets as f64);
                    match self.axis {
                        Horizontal => ((available as f64 * p), rect.height),
                        Vertical => (rect.width, (available as f64 * p)),
                    }
                }
            };
            let widget_rect = Rect {
                x: if self.axis == Horizontal {
                    rect.x + width * i as f64
                } else {
                    rect.x
                },
                y: if self.axis == Vertical {
                    rect.y + height * i as f64
                } else {
                    rect.y
                },
                width: width as f64,
                height: height as f64,
            };
            widget.render(&widget_rect, term);
        });
    }

    fn size_hint(&self, parent: &Rect) -> SizeHint {
        SizeHint::fill()
    }

    // fn handle_event(&mut self, event: &InputEvent) {
    //     self.widgets.iter_mut().for_each(|w| w.handle_event(event));
    // }
}
