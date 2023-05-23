use std::sync::{mpsc::Sender, Arc, RwLock};

use sanguine::{
    error::*,
    event::{Event, UserEvent},
    layout::{Axis, Constraint, Direction, NodeId, Rect},
    widgets::{Border, Menu, TextBox},
    App, Config, Layout,
};
use termwiz::input::{KeyCode, KeyEvent, Modifiers};

fn menu(buf: Arc<RwLock<Vec<String>>>) -> Menu<()> {
    // create a menu widget, and add some items to it
    let mut menu = Menu::new("Demo menu");
    menu.add_item("Quit", "", |_, _, event_tx| {
        // exit button using the event sender
        event_tx.send(UserEvent::Exit).ok();
    });
    menu.add_item("Delete", "", {
        // use a shared copy of the textbox buffer, and delete the last character of the buffer
        let buf = buf.clone();
        move |_, _, _| {
            let mut w = buf.write().unwrap();
            let len = w.len();
            let last = w.last_mut().unwrap();
            if last.is_empty() && len > 1 {
                w.pop();
            } else if !last.is_empty() {
                last.pop();
            }
        }
    });
    menu.add_item("Get line count: ", "<unknown>", move |this, menu, _| {
        // count buffer lines, and update the menu item
        menu.update_tag(this, |_| buf.read().unwrap().len().to_string())
    });
    menu
}

fn app(layout: &mut Layout) -> Option<NodeId> {
    // Create a TextBox widget, wrapped by a Border widget
    let textbox = TextBox::new();
    // Get a copy of the textbox buffer
    let textbox_buffer = textbox.buffer();

    // Add the menu widget
    let menu = layout.add_leaf(Border::new("Menu".to_owned(), menu(textbox_buffer)));

    // Add the first editor to the layout
    let editor = Border::new("Shared TextBox".to_owned(), textbox);
    let left = layout.add_leaf(editor);

    // Add a floating window
    layout.add_floating(
        // The window will contain a text box
        Border::new("Example Float", TextBox::new()),
        Rect {
            x: 10.,
            y: 10.,
            width: 25.,
            height: 5.,
        },
    );

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
        Some(Constraint::fill()),
        // The container will contain the cloned first editor, and the second editor
        [menu, bot_right],
    );

    // Get the root node of the layout
    let root = layout.root();
    // Ensure that the root container is laid out horizontally
    layout.set_direction(root, Axis::Horizontal);

    // Add the left window (leaf) and the right container to the root
    layout.add_child(root, left);
    layout.add_child(root, right);

    Some(left)
}

fn handle_event(state: &mut App, event: &Event<()>, _: Arc<Sender<UserEvent<()>>>) -> Result<bool> {
    match event {
        Event::Key(KeyEvent {
            key: KeyCode::Tab,
            modifiers: Modifiers::SHIFT,
        }) => {
            state.cycle_focus()?;
            Ok(true)
        }
        Event::Key(KeyEvent {
            key:
                k @ (KeyCode::UpArrow | KeyCode::DownArrow | KeyCode::LeftArrow | KeyCode::RightArrow),
            modifiers: Modifiers::SHIFT,
        }) => {
            let dir = match k {
                KeyCode::UpArrow => Direction::Up,
                KeyCode::DownArrow => Direction::Down,
                KeyCode::LeftArrow => Direction::Left,
                KeyCode::RightArrow => Direction::Right,
                _ => unreachable!(),
            };
            state.focus_direction(dir)?;
            Ok(true)
        }
        // If the event wasn't matched, return false to allow it to propagate
        _ => Ok(false),
    }
}

pub fn main() -> Result<()> {
    // Create the sanguine app, providing a handler for *global* input events.
    // In this case, we only handle occurrences of Shift+Tab, which we use to cycle focus.
    // If Shift+Tab is pressed, we return true to signal that the event should not be
    // propagated.
    let mut demo = App::new(
        // The default config is fine for this example
        Config::default(),
    )?
    // The with_layout function can be used to setup the layout and set the initially focused
    // window at the same time
    .with_layout(app)
    // Setup the handler for global input events
    .with_handler(handle_event);

    // The main render loop, which will run until the user closes the application (defaults to
    // Ctrl-q).
    while demo.handle_events()? {
        demo.render()?;
    }

    Ok(())
}
