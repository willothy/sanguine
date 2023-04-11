use std::collections::{HashMap, VecDeque};

use termwiz::{
    caps::Capabilities,
    input::{InputEvent, KeyCode, KeyEvent, Modifiers, MouseEvent},
    surface::{Change, Position},
    terminal::{buffered::BufferedTerminal, new_terminal, Terminal},
    Result,
};

#[derive(Clone)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

pub trait Widget<T: Terminal> {
    fn render(&self, rect: &Rect, term: &mut BufferedTerminal<T>);
    fn handle_event(&mut self, event: &InputEvent);
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

#[derive(Debug, Clone)]
pub enum Alignment {
    Start,
    Middle,
    End,
}

impl Default for Alignment {
    fn default() -> Self {
        Alignment::Start
    }
}

#[derive(Debug, Clone)]
pub struct Label {
    text: String,
    h_align: Alignment,
    v_align: Alignment,
}

impl Default for Label {
    fn default() -> Self {
        Self {
            text: String::new(),
            h_align: Default::default(),
            v_align: Default::default(),
        }
    }
}

impl Label {
    pub fn new(text: &str) -> Box<Self> {
        Box::new(Self {
            text: text.to_string(),
            ..Default::default()
        })
    }
}

impl Align for Label {
    fn align(mut self, h_align: Alignment, v_align: Alignment) -> Box<Self> {
        self.h_align = h_align;
        self.v_align = v_align;
        Box::new(self)
    }

    fn align_h(mut self, h_align: Alignment) -> Box<Self> {
        self.h_align = h_align;
        Box::new(self)
    }

    fn align_v(mut self, v_align: Alignment) -> Box<Self> {
        self.v_align = v_align;
        Box::new(self)
    }
}

pub trait Align
where
    Self: Sized,
{
    fn align(self, h_align: Alignment, v_align: Alignment) -> Box<Self>;
    fn align_h(self, h_align: Alignment) -> Box<Self>;
    fn align_v(self, v_align: Alignment) -> Box<Self>;
    fn topleft(self) -> Box<Self> {
        self.align_h(Alignment::Start).align_v(Alignment::Start)
    }
    fn topcenter(self) -> Box<Self> {
        self.align_h(Alignment::Middle).align_v(Alignment::Start)
    }
    fn topright(self) -> Box<Self> {
        self.align_h(Alignment::End).align_v(Alignment::Start)
    }
    fn centerleft(self) -> Box<Self> {
        self.align_h(Alignment::Start).align_v(Alignment::Middle)
    }
    fn center(self) -> Box<Self> {
        self.align_h(Alignment::Middle).align_v(Alignment::Middle)
    }
    fn centerright(self) -> Box<Self> {
        self.align_h(Alignment::End).align_v(Alignment::Middle)
    }
    fn bottomleft(self) -> Box<Self> {
        self.align_h(Alignment::Start).align_v(Alignment::End)
    }
    fn bottomcenter(self) -> Box<Self> {
        self.align_h(Alignment::Middle).align_v(Alignment::End)
    }
    fn bottomright(self) -> Box<Self> {
        self.align_h(Alignment::End).align_v(Alignment::End)
    }
}

impl<T: Terminal> Widget<T> for Label {
    fn render(&self, rect: &Rect, term: &mut BufferedTerminal<T>) {
        term.add_change(Change::CursorPosition {
            x: Position::Absolute(rect.x.floor() as usize),
            y: Position::Absolute(rect.y.floor() as usize),
        });
        let mut text = self.text.clone();
        if text.len() > rect.width as usize {
            text.truncate(rect.width as usize);
        } else {
            match self.h_align {
                Alignment::Start => {}
                Alignment::Middle => {
                    let pad = (rect.width as usize - text.len()) / 2;
                    text = format!("{}{}", " ".repeat(pad), text);
                }
                Alignment::End => {
                    let pad = rect.width as usize - text.len();
                    text = format!("{}{}", " ".repeat(pad), text);
                }
            }
        }
        if text.lines().count() > rect.height as usize {
            text = text
                .lines()
                .take(rect.height as usize)
                .map(|l| l.to_string())
                .collect::<Vec<_>>()
                .join("\n");
        } else {
            match self.v_align {
                Alignment::Start => {}
                Alignment::Middle => {
                    let pad = (rect.height as usize - text.lines().count()) / 2;
                    let mut lines = vec![];
                    for _ in 0..pad {
                        lines.push(" ".repeat(rect.width.floor() as usize));
                    }
                    text = format!("{}\n{}", lines.join("\n"), text);
                }
                Alignment::End => {
                    let pad = rect.height as usize - text.lines().count();
                    let mut lines = vec![];
                    for _ in 0..pad {
                        lines.push(" ".repeat(rect.width.floor() as usize));
                    }
                    text = format!("{}\n{}", lines.join("\n"), text);
                }
            }
        }
        for line in text.lines() {
            term.add_change(Change::Text(line.to_string()));
            term.add_change(Change::CursorPosition {
                x: Position::Absolute(rect.x.floor() as usize),
                y: Position::Relative(1),
            });
        }
    }

    fn handle_event(&mut self, _event: &InputEvent) {}
}

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

pub struct BorderChars {
    pub top_left: char,
    pub top_right: char,
    pub bottom_left: char,
    pub bottom_right: char,
    pub horizontal: char,
    pub vertical: char,
}

pub enum BorderVariant {
    Single,
    Double,
    Rounded,
    Custom(BorderChars),
    None,
}

impl From<BorderVariant> for BorderChars {
    fn from(border: BorderVariant) -> Self {
        match border {
            BorderVariant::Single => BorderChars {
                top_left: '┌',
                top_right: '┐',
                bottom_left: '└',
                bottom_right: '┘',
                horizontal: '─',
                vertical: '│',
            },
            BorderVariant::Double => BorderChars {
                top_left: '╔',
                top_right: '╗',
                bottom_left: '╚',
                bottom_right: '╝',
                horizontal: '═',
                vertical: '║',
            },
            BorderVariant::Rounded => BorderChars {
                top_left: '╭',
                top_right: '╮',
                bottom_left: '╰',
                bottom_right: '╯',
                horizontal: '─',
                vertical: '│',
            },
            BorderVariant::Custom(chars) => chars,
            BorderVariant::None => BorderChars {
                top_left: ' ',
                top_right: ' ',
                bottom_left: ' ',
                bottom_right: ' ',
                horizontal: ' ',
                vertical: ' ',
            },
        }
    }
}

pub struct Border<T: Terminal> {
    pub chars: BorderChars,
    pub inner: Box<dyn Widget<T>>,
}

impl<T: Terminal> Border<T> {
    pub fn new(border: BorderVariant, inner: Box<dyn Widget<T>>) -> Self {
        Self {
            chars: border.into(),
            inner,
        }
    }
}

impl<T: Terminal> Widget<T> for Border<T> {
    fn render(&self, rect: &Rect, term: &mut BufferedTerminal<T>) {
        let mut inner_rect = rect.clone();
        inner_rect.x += 1.0;
        inner_rect.y += 1.0;
        inner_rect.width -= 2.0;
        inner_rect.height -= 2.0;
        term.add_change(Change::CursorPosition {
            x: Position::Absolute(rect.x.floor() as usize),
            y: Position::Absolute(rect.y.floor() as usize),
        });
        term.add_change(Change::Text(self.chars.top_left.to_string()));
        for _ in 0..(rect.width - 2.0) as usize {
            term.add_change(Change::Text(self.chars.horizontal.to_string()));
        }
        term.add_change(Change::Text(self.chars.top_right.to_string()));
        for _ in 0..(rect.height - 2.0) as usize {
            term.add_change(Change::CursorPosition {
                x: Position::Absolute(rect.x.floor() as usize),
                y: Position::Relative(1),
            });
            term.add_change(Change::Text(self.chars.vertical.to_string()));
            term.add_change(Change::CursorPosition {
                x: Position::Absolute((rect.x + rect.width - 1.0).floor() as usize),
                y: Position::Relative(0),
            });
            term.add_change(Change::Text(self.chars.vertical.to_string()));
        }
        term.add_change(Change::CursorPosition {
            x: Position::Absolute(rect.x.floor() as usize),
            y: Position::Absolute((rect.y + rect.height - 1.0).floor() as usize),
        });
        term.add_change(Change::Text(self.chars.bottom_left.to_string()));
        for _ in 0..(rect.width - 1.0) as usize {
            term.add_change(Change::Text(self.chars.horizontal.to_string()));
        }
        term.add_change(Change::CursorPosition {
            x: Position::Absolute((rect.x + rect.width - 1.0).floor() as usize),
            y: Position::Relative(0),
        });
        term.add_change(Change::Text(self.chars.bottom_right.to_string()));
        self.inner.render(&inner_rect, term);
    }

    fn handle_event(&mut self, _event: &InputEvent) {
        // nothing
    }
}

pub struct Ui<T: Terminal> {
    pub root: Box<dyn Widget<T>>,
    pub terminal: BufferedTerminal<T>,
    pub queue: VecDeque<InputEvent>,
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
