use sanguine::{prelude::*, widgets::Menu};

pub fn main() -> Result<()> {
    // Create the layout struct
    let mut layout = Layout::new();

    // Create a TextBox widget, wrapped by a Border widget
    let textbox = TextBox::new();
    // Get a copy of the textbox buffer
    let textbox_buffer = textbox.buffer();
    let editor_1 = Border::new("Shared TextBox".to_owned(), textbox);

    // create a menu widget, and add some items to it
    let mut menu = Menu::new("Demo menu");
    menu.add_item("Quit", "", |_, _, event_tx| {
        // exit button using the event sender
        event_tx.send(UserEvent::Exit).ok();
    });
    menu.add_item("Delete", "", {
        // use a shared copy of the textbox buffer, and delete the last character of the buffer
        let textbox_buffer = textbox_buffer.clone();
        move |_, _, _| {
            let mut w = textbox_buffer.write().unwrap();
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
        menu.update_tag(this, |_| textbox_buffer.read().unwrap().len().to_string())
    });

    // Add the first editor to the layout
    let left = layout.add_leaf(editor_1);

    // Add the menu widget
    let top_right = layout.add_leaf(Border::new("Menu".to_owned(), menu));

    // Add a floating window
    layout.add_floating(
        // The window will contain a text box
        Border::new("Example Float", TextBox::new()),
        Rect {
            x: 1.,
            y: 1.,
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
        |state: &mut App, event: &Event<_>, _| {
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
