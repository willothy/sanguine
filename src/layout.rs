use termwiz::{
    input::InputEvent,
    surface::Surface,
    terminal::{buffered::BufferedTerminal, Terminal},
};

use crate::widget::Widget;

#[derive(Debug, Clone)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        self.width = width as f64;
        self.height = height as f64;
    }

    pub fn center(&self) -> (f64, f64) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        self.contains(other.x, other.y)
            || self.contains(other.x + other.width, other.y)
            || self.contains(other.x, other.y + other.height)
            || self.contains(other.x + other.width, other.y + other.height)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum LayoutAxis {
    Horizontal,
    Vertical,
}

pub struct Layout {
    widgets: Vec<Box<dyn Widget>>,
    axis: LayoutAxis,
    size: Option<SizeHint>,
}

#[derive(Debug, Clone)]
pub enum SizeHint {
    Fixed(usize),
    Percentage(f64),
    Fill,
}

impl SizeHint {
    pub fn fill() -> SizeHint {
        SizeHint::Fill
    }
}

pub fn calc_sizes(container: &Rect, sizes: &Vec<SizeHint>, axis: &LayoutAxis) -> Vec<SizeHint> {
    let mut new_sizes = vec![SizeHint::Fixed(0); sizes.len()];
    let width = match axis {
        LayoutAxis::Horizontal => container.width,
        LayoutAxis::Vertical => container.height,
    };
    let mut remaining = width;

    let fixed = sizes
        .iter()
        .enumerate()
        .filter_map(|(i, size)| match size {
            SizeHint::Fixed(size) => {
                // new_sizes.insert(i, SizeHint::Fixed(*size));
                new_sizes[i] = SizeHint::Fixed(*size);
                Some(size)
            }
            _ => None,
        })
        .sum::<usize>();

    remaining -= fixed as f64;

    let mut percents = sizes
        .iter()
        .enumerate()
        .filter_map(|(i, size)| match size {
            SizeHint::Percentage(percent) => Some((i, *percent)),
            _ => None,
        })
        .collect::<Vec<_>>();
    let n_percent = percents.len();
    let percent = percents.iter().map(|(_, f)| f).sum::<f64>();

    if percent > 1.0 {
        let diff = percent - 1.0;
        let avg = diff / n_percent as f64;
        percents.iter_mut().for_each(|(_, f)| *f -= avg);
    }
    let mut pct_total = 0;
    percents.iter_mut().for_each(|(i, f)| {
        *f *= remaining as f64;
        let size = f.floor() as usize;
        pct_total += size;
        // new_sizes.insert(*i, SizeHint::Fixed(size));
        new_sizes[*i] = SizeHint::Fixed(size);
    });
    remaining -= pct_total as f64;

    let fill = sizes
        .iter()
        .enumerate()
        .filter_map(|(i, size)| match size {
            SizeHint::Fill => Some(i),
            _ => None,
        })
        .collect::<Vec<_>>();

    let nfill = fill.len();

    let fill_size = remaining / nfill as f64;
    fill.iter().for_each(|i| {
        // new_sizes.insert(*i, SizeHint::Fixed(fill_size.floor() as usize));
        new_sizes[*i] = SizeHint::Fixed(fill_size.ceil() as usize);
    });
    new_sizes
}

impl Layout {
    pub fn h(widgets: Vec<Box<dyn Widget>>, size: Option<SizeHint>) -> Box<Self> {
        Box::new(Self {
            widgets,
            axis: LayoutAxis::Horizontal,
            size,
        })
    }

    pub fn v(widgets: Vec<Box<dyn Widget>>, size: Option<SizeHint>) -> Box<Self> {
        Box::new(Self {
            widgets,
            axis: LayoutAxis::Vertical,
            size,
        })
    }
}

impl Widget for Layout {
    fn render(&self, rect: &Rect, term: &mut Surface) {
        use LayoutAxis::*;

        let nwidgets = self.widgets.len();

        let sizes = self
            .widgets
            .iter()
            .map(|w| w.size_hint(&rect))
            .collect::<Vec<_>>();

        let sizes = calc_sizes(&rect, &sizes, &self.axis);

        // println!("rect: {:?}", rect);
        // println!("sizes: {:?}", sizes);
        let mut current = match self.axis {
            Horizontal => rect.x,
            Vertical => rect.y,
        };
        self.widgets.iter().enumerate().for_each(|(i, w)| {
            let size = match sizes[i] {
                SizeHint::Fixed(s) => s as f64,
                _ => 0.,
            };
            let (width, height) = match self.axis {
                Horizontal => (size, rect.height),
                Vertical => (rect.width, size),
            };
            let (x, y) = (
                if self.axis == Horizontal {
                    // rect.x + width * i as f64
                    current
                } else {
                    rect.x
                },
                if self.axis == Vertical {
                    // rect.y + height * i as f64
                    current
                } else {
                    rect.y
                },
            );
            let widget_rect = Rect {
                x,
                y,
                width,
                height,
            };
            current += size;
            w.render(&widget_rect, term);
        });

        // let mut n_pct = 0;
        // let available = available
        //     - self
        //         .widgets
        //         .iter()
        //         .filter_map(|w| match w.size_hint(&rect) {
        //             SizeHint::Fixed(s) => Some(s as f64),
        //             SizeHint::Percentage(_) => {
        //                 n_pct += 1;
        //                 None
        //             }
        //             _ => {
        //                 n_pct += 1;
        //                 None
        //             }
        //         })
        //         .sum::<f64>()
        //     /* + if nwidgets % 2 != 0 && nwidgets > 1 {
        //         1.
        //     } else {
        //         0.
        //     } */;

        // let mut remaining = available;
        // self.widgets.iter().enumerate().for_each(|(i, widget)| {
        //     let size = widget.size_hint(&rect);
        //     let (width, height) = match &size {
        //         SizeHint::Fixed(s) => match self.axis {
        //             Horizontal => (*s as f64, rect.height),
        //             Vertical => (rect.width, *s as f64),
        //         },
        //         SizeHint::Fill => match self.axis {
        //             Horizontal => (available / n_pct as f64, rect.height),
        //             Vertical => (rect.width, available / n_pct as f64),
        //         },
        //         SizeHint::Percentage(p) => {
        //             // let p = p * (available / n_pct as f64);
        //             let p = available * p;
        //             match self.axis {
        //                 Horizontal => (p, rect.height),
        //                 Vertical => (rect.width, p),
        //             }
        //         }
        //     };
        //     let (x, y) = match widget.size_hint(&rect) {
        //         // SizeHint::Percentage(p) => (
        //         //     if self.axis == Horizontal {
        //         //         // rect.x + width * i as f64
        //         //         remaining as f64
        //         //     } else {
        //         //         rect.x
        //         //     },
        //         //     if self.axis == Vertical {
        //         //         // rect.y + height * i as f64
        //         //         remaining as f64
        //         //     } else {
        //         //         rect.y
        //         //     },
        //         // ),
        //         _ => (
        //             if self.axis == Horizontal {
        //                 rect.x + width * i as f64
        //                 // remaining as f64
        //             } else {
        //                 rect.x
        //             },
        //             if self.axis == Vertical {
        //                 rect.y + height * i as f64
        //                 // remaining as f64
        //             } else {
        //                 rect.y
        //             },
        //         ),
        //     };
        //
        //     let widget_rect = Rect {
        //         width: width as f64,
        //         height: height as f64,
        //         x,
        //         y,
        //     };
        //     widget.render(&widget_rect, term);
        // });
    }

    fn size_hint(&self, _parent: &Rect) -> SizeHint {
        self.size.clone().unwrap_or(SizeHint::fill())
    }

    // fn handle_event(&mut self, event: &InputEvent) {
    //     self.widgets.iter_mut().for_each(|w| w.handle_event(event));
    // }
}
