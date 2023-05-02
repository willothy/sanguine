use std::{sync::Arc, time::Duration};

use anyhow::Result;
use sanguine::{Direction, Layout, Leaf, Rect, Sanguine, Widget};
use termwiz::surface::{Change, Position};

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

struct Border {
    chars: BorderChars,
}

impl Border {
    pub fn new() -> Self {
        Self {
            chars: BorderVariant::Rounded.into(),
        }
    }

    #[allow(unused)]
    pub fn with_variant(variant: BorderVariant) -> Self {
        Self {
            chars: variant.into(),
        }
    }
}

impl Widget for Border {
    fn render(&self, rect: Rect, surface: &mut termwiz::surface::Surface) {
        let mut changes = vec![];
        changes.push(Change::CursorPosition {
            x: Position::Absolute(rect.x.floor() as usize),
            y: Position::Absolute(rect.y.floor() as usize),
        });
        changes.push(Change::Text(self.chars.top_left.to_string()));
        for _ in 0..(rect.width - 1.0) as usize {
            changes.push(Change::Text(self.chars.horizontal.to_string()));
        }
        changes.push(Change::CursorPosition {
            x: Position::Absolute((rect.x + rect.width - 1.0).floor() as usize),
            y: Position::Relative(0),
        });
        changes.push(Change::Text(self.chars.top_right.to_string()));
        for _ in 0..(rect.height - 1.0) as usize {
            changes.push(Change::CursorPosition {
                x: Position::Absolute(rect.x.floor() as usize),
                y: Position::Relative(1),
            });
            changes.push(Change::Text(self.chars.vertical.to_string()));
            changes.push(Change::CursorPosition {
                x: Position::Absolute((rect.x + rect.width - 1.0).floor() as usize),
                y: Position::Relative(0),
            });
            changes.push(Change::Text(self.chars.vertical.to_string()));
        }
        changes.push(Change::CursorPosition {
            x: Position::Absolute(rect.x.floor() as usize),
            y: Position::Absolute((rect.y + rect.height - 1.0).floor() as usize),
        });
        changes.push(Change::Text(self.chars.bottom_left.to_string()));
        for _ in 0..(rect.width - 1.0) as usize {
            changes.push(Change::Text(self.chars.horizontal.to_string()));
        }
        changes.push(Change::CursorPosition {
            x: Position::Absolute((rect.x + rect.width - 1.0).floor() as usize),
            y: Position::Relative(0),
        });
        changes.push(Change::Text(self.chars.bottom_right.to_string()));
        surface.add_changes(changes);
    }
}

pub fn main() -> Result<()> {
    let mut layout = Layout::new();

    let left = layout.add_leaf(Leaf::new(Arc::new(Border::new())));
    let right = layout.add_leaf(Leaf::new(Arc::new(Border::new())));
    // let top_right = layout.add_leaf(Leaf::new(Arc::new(Border::new())));
    // let bot_right = layout.add_leaf(Leaf::new(Arc::new(Border::new())));
    // let right = layout.add_with_children(
    //     Direction::Vertical,
    //     Some(SizeHint::fill()),
    //     [top_right, bot_right],
    // );

    let root = layout.root();
    layout.set_direction(root, Direction::Horizontal);

    layout.add_child(root, left);
    layout.add_child(root, right);

    // let bounds = {
    //     let (w, h) = term.dimensions();
    //     Rect::new(0., 0., w as f32, h as f32)
    // };
    // println!("bounds: {:?}", bounds);
    // layout.compute(&bounds);
    //
    // layout.print_recursive(root);

    let mut s = Sanguine::new(layout)?;
    s.render()?;
    std::thread::sleep(Duration::from_secs(3));
    s.update_layout(|l| {
        // todo
        l.split(
            right,
            Direction::Vertical,
            Leaf::new(Arc::new(Border::new())),
        );
    });
    s.render()?;
    std::thread::sleep(Duration::from_secs(3));

    Ok(())
}
