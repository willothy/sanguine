use sanguine::prelude::*;

pub fn main() -> Result<()> {
    // Create the layout struct
    let mut layout = Layout::new();

    // Create two TextBox widgets, wrapped by Border widgets
    let editor_1 = Border::new("textbox 1".to_owned(), TextBox::new());
    let editor_2 = Border::new("textbox 2".to_owned(), TextBox::new());

    // Add the first editor to the layout
    let left = layout.add_leaf(editor_1);

    // Clone the first editor to add it to the layout again
    // This widget will be *shared* between the two windows, meaning that changes to the underlying
    // buffer will be shown in both windows and focusing on either window will allow you to edit
    // the same buffer.
    let top_right = layout.clone_leaf(left);

    // Add the second editor to the layout
    let bot_right = layout.add_leaf(editor_2);

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
    // If the Shift+Tab is pressed, we return true to signal that the event should not be
    // propagated.
    let mut app = Sanguine::with_global_handler(
        layout,
        Box::new(move |state: &mut Sanguine, event: &InputEvent, _| {
            if let InputEvent::Key(KeyEvent {
                key: KeyCode::Tab,
                modifiers: Modifiers::SHIFT,
            }) = event
            {
                state.cycle_focus()?;
                return Ok(true);
            }
            Ok(false)
        }),
    )?;
    // Set the initial focus to the left node.
    // Only windows can be focused, attempting to focus a container will throw an error.
    app.set_focus(left)?;

    while app.handle_events()? {
        app.render()?;
    }

    Ok(())
}
