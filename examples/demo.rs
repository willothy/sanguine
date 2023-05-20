use std::{
    sync::{mpsc::Sender, Arc},
    unreachable,
};

use sanguine::{prelude::*, Widget};
use termwiz::{
    cell::AttributeChange,
    color::{AnsiColor, ColorAttribute},
};

pub struct Menu {
    items: Vec<(String, String, Box<dyn Fn(&mut Self, Arc<Sender<()>>)>)>,
    active: usize,
}

impl Widget for Menu {
    fn render(&self, _layout: &Layout, surface: &mut Surface, _focused: bool) {
        let dims = surface.dimensions();
        surface.add_changes(vec![Change::CursorPosition {
            x: Position::Absolute(0),
            y: Position::Relative(0),
        }]);
        for (i, (item, tag, _)) in self.items.iter().enumerate() {
            if i == self.active {
                surface.add_changes(vec![
                    Change::Attribute(AttributeChange::Foreground(AnsiColor::Black.into())),
                    Change::Attribute(AttributeChange::Background(AnsiColor::White.into())),
                ]);
            }
            let line = format!("{item} ({tag})");
            surface.add_changes(vec![
                Change::Text(format!("{:^width$}", line, width = dims.0)),
                Change::CursorPosition {
                    x: Position::Relative(dims.0 as isize),
                    y: Position::Relative(0),
                },
                Change::Attribute(AttributeChange::Foreground(ColorAttribute::Default)),
                Change::Attribute(AttributeChange::Background(ColorAttribute::Default)),
                Change::CursorPosition {
                    x: Position::Absolute(0),
                    y: Position::Relative(1),
                },
            ]);
        }
    }

    fn update(
        &mut self,
        _: &Rect,
        event: Event,
        exit_tx: std::sync::Arc<std::sync::mpsc::Sender<()>>,
    ) -> sanguine::error::Result<()> {
        match event {
            Event::Input(InputEvent::Key(KeyEvent {
                key: KeyCode::UpArrow,
                modifiers: _,
            })) => {
                if self.active > 0 {
                    self.active -= 1;
                }
            }
            Event::Input(InputEvent::Key(KeyEvent {
                key: KeyCode::DownArrow,
                modifiers: _,
            })) => {
                if self.active < self.items.len() - 1 {
                    self.active += 1;
                }
            }
            Event::Input(InputEvent::Key(KeyEvent {
                key: KeyCode::Enter,
                ..
            })) => {
                let func = &self.items[self.active].2 as *const dyn Fn(&mut Self, Arc<Sender<()>>);
                unsafe { (*func)(self, exit_tx.clone()) };
            }
            Event::Input(InputEvent::Mouse(MouseEvent {
                y, mouse_buttons, ..
            })) => {
                if mouse_buttons == MouseButtons::LEFT {
                    if y as usize <= self.items.len() {
                        self.active = y as usize;
                        let func =
                            &self.items[self.active].2 as *const dyn Fn(&mut Self, Arc<Sender<()>>);
                        unsafe { (*func)(self, exit_tx.clone()) };
                    }
                } else if mouse_buttons == MouseButtons::NONE {
                    if (y as usize) < self.items.len() {
                        self.active = y as usize;
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }
}

pub fn main() -> Result<()> {
    // Create the layout struct
    let mut layout = Layout::new();

    // Create a TextBox widget, wrapped by a Border widget
    let editor_1 = Border::new("textbox 1".to_owned(), TextBox::new());

    let menu = Border::new(
        "menu".to_owned(),
        Menu {
            items: vec![
                (
                    "item 1".to_owned(),
                    "quit".to_owned(),
                    Box::new(|_, tx: Arc<Sender<()>>| {
                        tx.send(()).ok();
                    }),
                ),
                (
                    "+".to_owned(),
                    "0".to_owned(),
                    Box::new(|s, _| {
                        s.items[2].1 = format!("{}", s.items[1].1.parse::<u16>().unwrap() + 1);
                        s.items[1].1 = format!("{}", s.items[1].1.parse::<u16>().unwrap() + 1);
                    }),
                ),
                (
                    "-".to_owned(),
                    "0".to_owned(),
                    Box::new(|s, _| {
                        s.items[2].1 = format!("{}", s.items[1].1.parse::<u16>().unwrap() - 1);
                        s.items[1].1 = format!("{}", s.items[1].1.parse::<u16>().unwrap() - 1);
                    }),
                ),
            ],
            active: 0,
        },
    );

    // Add the first editor to the layout
    let left = layout.add_leaf(editor_1);

    // Add the menu widget
    let top_right = layout.add_leaf(menu);

    // Clone the first editor to add it to the layout again
    // This widget will be *shared* between the two windows, meaning that changes to the underlying
    // buffer will be shown in both windows and focusing on either window will allow you to edit
    // the same buffer.
    let bot_right = layout.clone_leaf(left);

    // Add the second editor to the layout
    // let bot_right = layout.add_leaf(editor_2);

    // Create a container to hold the two right hand side editors
    let right = layout.add_with_children(
        // The container will be a vertical layout
        Axis::Vertical,
        // The container will take up all available space
        Some(SizeHint::fill()),
        // The container will contain the cloned first editor, and the second editor
        [top_right, bot_right],
    );

    // Get the root node of the layout
    let root = layout.root();
    // Ensure that the root container is laid out horizontally
    layout.set_direction(root, Axis::Horizontal);

    // Add the left window (leaf) and the right container to the root
    layout.add_child(root, left);
    layout.add_child(root, right);

    // Create the sanguine app, providing a handler for *global* input events.
    // In this case, we only handle occurrences of Shift+Tab, which we use to cycle focus.
    // If Shift+Tab is pressed, we return true to signal that the event should not be
    // propagated.
    let mut app = App::with_global_handler(
        layout,
        // The default config is fine for this example
        Config::default(),
        |state: &mut App, event: &Event, _| {
            match event {
                Event::Input(InputEvent::Key(KeyEvent {
                    key: KeyCode::Tab,
                    modifiers: Modifiers::SHIFT,
                })) => {
                    state.cycle_focus()?;
                    return Ok(true);
                }
                Event::Input(InputEvent::Key(KeyEvent {
                    key:
                        k @ (KeyCode::UpArrow
                        | KeyCode::DownArrow
                        | KeyCode::LeftArrow
                        | KeyCode::RightArrow),
                    modifiers: Modifiers::SHIFT,
                })) => {
                    let dir = match k {
                        KeyCode::UpArrow => Direction::Up,
                        KeyCode::DownArrow => Direction::Down,
                        KeyCode::LeftArrow => Direction::Left,
                        KeyCode::RightArrow => Direction::Right,
                        _ => unreachable!(),
                    };
                    state.focus_direction(dir)?;
                    return Ok(true);
                }
                _ => (),
            }
            Ok(false)
        },
    )?;
    // Set the initial focus to the left node.
    // Only windows can be focused, attempting to focus a container will throw an error.
    app.set_focus(left)?;

    // The main render loop, which will run until the user closes the application (defaults to
    // Ctrl-q).
    while app.handle_events()? {
        app.render()?;
    }

    Ok(())
}
