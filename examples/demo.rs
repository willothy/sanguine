use std::sync::Arc;

use anyhow::Result;
use rand::distributions::uniform::SampleRange;
use sanguine::prelude::{Axis, Layout, Leaf, Rect, Sanguine, SizeHint, Widget};
use termwiz::surface::{Change, Position};

pub struct BorderChars {
    pub top_left: char,
    pub top_right: char,
    pub bottom_left: char,
    pub bottom_right: char,
    pub top: char,
    pub bottom: char,
    pub left: char,
    pub right: char,
}

struct Border;
struct IndBorder;

const TEE_LEFT: char = '┤';
const TEE_RIGHT: char = '├';
const TEE_BOTTOM: char = '┬';
const TEE_TOP: char = '┴';
const CROSS: char = '┼';
const HORIZONTAL: char = '─';
const VERTICAL: char = '│';
const TOP_LEFT: char = '┌';
const TOP_RIGHT: char = '┐';
const BOTTOM_LEFT: char = '└';
const BOTTOM_RIGHT: char = '┘';

fn get_border_chars(child: &Rect, parent: &Rect) -> BorderChars {
    let left = child.x > 0.;
    let right = (child.x + child.width) != parent.width;
    let top = child.y > 0.;
    let bottom = (child.y + child.height) < parent.height;

    let top_left = if top && left {
        ' '
    } else if top {
        VERTICAL
    } else if left {
        HORIZONTAL
    } else {
        TOP_LEFT
    };
    let top_right = if top && right {
        VERTICAL
    } else if top {
        VERTICAL
    } else if right {
        TEE_BOTTOM
    } else {
        TOP_RIGHT
    };
    let bottom_left = if bottom && left {
        HORIZONTAL
    } else if bottom {
        TEE_RIGHT
    } else if left {
        HORIZONTAL
    } else {
        BOTTOM_LEFT
    };
    let bottom_right = if bottom && right {
        CROSS
    } else if bottom {
        TEE_LEFT
    } else if right {
        TEE_TOP
    } else {
        BOTTOM_RIGHT
    };
    let top = if top { ' ' } else { '─' };
    let bottom = '─';
    let left = if left { ' ' } else { VERTICAL };
    let right = '│';
    BorderChars {
        top_left,
        top_right,
        bottom_left,
        bottom_right,
        top,
        bottom,
        left,
        right,
    }
}

impl Widget for Border {
    fn render(&self, _layout: &Layout, rect: Rect, surface: &mut termwiz::surface::Surface) {
        let size = surface.dimensions();

        let corners = get_border_chars(
            &rect,
            &Rect {
                x: 0.,
                y: 0.,
                width: size.0 as f32,
                height: size.1 as f32,
            },
        );

        let mut changes = vec![];
        changes.push(Change::CursorPosition {
            x: Position::Absolute(rect.x.floor() as usize),
            y: Position::Absolute(rect.y.floor() as usize),
        });
        if corners.top_left != ' ' {
            changes.push(Change::Text(corners.top_left.to_string()));
        }
        for _ in 0..(rect.width.ceil() - 1.0) as usize {
            changes.push(Change::Text(corners.top.to_string()));
        }
        changes.push(Change::CursorPosition {
            x: Position::Absolute((rect.x + rect.width - 1.0).floor() as usize),
            y: Position::Relative(0),
        });
        if corners.top_right != ' ' {
            changes.push(Change::Text(corners.top_right.to_string()));
        }
        for _ in 0..(rect.height - 1.0) as usize {
            changes.push(Change::CursorPosition {
                x: Position::Absolute(rect.x.floor() as usize),
                y: Position::Relative(1),
            });
            changes.push(Change::Text(corners.left.to_string()));
            changes.push(Change::CursorPosition {
                x: Position::Absolute((rect.x + rect.width - 1.0).floor() as usize),
                y: Position::Relative(0),
            });
            changes.push(Change::Text(corners.right.to_string()));
        }
        changes.push(Change::CursorPosition {
            x: Position::Absolute(rect.x.floor() as usize),
            y: Position::Absolute((rect.y + rect.height - 1.0).floor() as usize),
        });
        if corners.bottom_left != ' ' {
            changes.push(Change::Text(corners.bottom_left.to_string()));
        } else {
            changes.push(Change::Text(corners.bottom.to_string()));
        }
        for _ in 0..(rect.width.ceil() - 1.0) as usize {
            changes.push(Change::Text(corners.bottom.to_string()));
        }
        changes.push(Change::CursorPosition {
            x: Position::Absolute((rect.x + rect.width - 1.0).floor() as usize),
            y: Position::Relative(0),
        });
        if corners.bottom_right != ' ' {
            changes.push(Change::Text(corners.bottom_right.to_string()));
        }
        surface.add_changes(changes);
    }
}

impl Widget for IndBorder {
    fn render(&self, _layout: &Layout, rect: Rect, surface: &mut termwiz::surface::Surface) {
        let mut changes = vec![];
        changes.push(Change::CursorPosition {
            x: Position::Absolute(rect.x.floor() as usize),
            y: Position::Absolute(rect.y.floor() as usize),
        });
        changes.push(Change::Text(TOP_LEFT.to_string()));
        for _ in 0..(rect.width - 1.0) as usize {
            changes.push(Change::Text(HORIZONTAL.to_string()));
        }
        changes.push(Change::CursorPosition {
            x: Position::Absolute((rect.x + rect.width - 1.0).floor() as usize),
            y: Position::Relative(0),
        });
        changes.push(Change::Text(TOP_RIGHT.to_string()));
        for _ in 0..(rect.height - 1.0) as usize {
            changes.push(Change::CursorPosition {
                x: Position::Absolute(rect.x.floor() as usize),
                y: Position::Relative(1),
            });
            changes.push(Change::Text(VERTICAL.to_string()));
            changes.push(Change::CursorPosition {
                x: Position::Absolute((rect.x + rect.width - 1.0).floor() as usize),
                y: Position::Relative(0),
            });
            changes.push(Change::Text(VERTICAL.to_string()));
        }
        changes.push(Change::CursorPosition {
            x: Position::Absolute(rect.x.floor() as usize),
            y: Position::Absolute((rect.y + rect.height - 1.0).floor() as usize),
        });
        changes.push(Change::Text(BOTTOM_LEFT.to_string()));
        for _ in 0..(rect.width - 1.0) as usize {
            changes.push(Change::Text(HORIZONTAL.to_string()));
        }
        changes.push(Change::CursorPosition {
            x: Position::Absolute((rect.x + rect.width - 1.0).floor() as usize),
            y: Position::Relative(0),
        });
        changes.push(Change::Text(BOTTOM_RIGHT.to_string()));
        surface.add_changes(changes);
    }
}

pub fn main() -> Result<()> {
    let mut layout = Layout::new();

    let left = layout.add_leaf(Leaf::new(Arc::new(IndBorder)));
    // let right = layout.add_leaf(Leaf::new(Arc::new(Border)));
    let top_right = layout.add_leaf(Leaf::new(Arc::new(IndBorder)));
    let bot_right = layout.add_leaf(Leaf::new(Arc::new(IndBorder)));
    let right = layout.add_with_children(
        Axis::Vertical,
        Some(SizeHint::fill()),
        [top_right, bot_right],
    );

    let root = layout.root();
    layout.set_direction(root, Axis::Horizontal);

    layout.add_child(root, left);
    layout.add_child(root, right);

    let mut buf = String::new();
    let mut s = Sanguine::new(layout)?;
    s.render()?;
    std::thread::sleep(std::time::Duration::from_millis(1000));

    for _ in 0..10 {
        // let parent = s.inspect_layout(|l| l.parent(new2)).unwrap();
        let leaves = s.inspect_layout(|l| l.leaves());
        let between = 0..leaves.len();
        let mut rng = rand::thread_rng();
        let idx = between.sample_single(&mut rng);
        let vertical = (0..2).sample_single(&mut rng) % 2 == 0;
        let _new3 = s.update_layout(|l| {
            l.split(
                leaves[idx],
                if vertical {
                    Axis::Vertical
                } else {
                    Axis::Horizontal
                },
                Leaf::new(Arc::new(IndBorder)),
            )
        });
        s.render()?;
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }

    std::io::stdin().read_line(&mut buf)?;

    Ok(())
}
