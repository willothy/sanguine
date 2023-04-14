use termwiz::surface::{Change, Position, Surface};

use crate::{
    align::{Align, Alignment},
    layout::{Rect, SizeHint},
    widget::Widget,
};

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

impl Widget for Label {
    fn render(&self, rect: &Rect, term: &mut Surface) {
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

    fn size_hint(&self, _parent: &Rect) -> SizeHint {
        SizeHint::fill()
    }

    // fn handle_event(&mut self, _event: &InputEvent) {}
}
