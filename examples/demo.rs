use termwiz::{
    caps::Capabilities,
    surface::Change,
    terminal::{buffered::BufferedTerminal, new_terminal, Terminal},
    Result,
};

use sanguine::{
    align::Align,
    bordered,
    float::Float,
    horizontal, label,
    layout::{Rect, SizeHint},
    vertical, Sanguine,
};

fn main() -> Result<()> {
    let caps = Capabilities::new_from_env()?;
    let mut term = new_terminal(caps)?;
    term.set_raw_mode()?;

    let mut buf = BufferedTerminal::new(term)?;
    // Hide the cursor before initializing
    buf.add_change(Change::CursorVisibility(
        termwiz::surface::CursorVisibility::Hidden,
    ));

    buf.flush()?;

    let mut ui = Sanguine::new(
        horizontal![
            // Bordered window, 40% of parent size
            bordered![label!["Window 1!"].center() => Some(SizeHint::Percentage(0.4))],
            vertical![
                bordered![label!["Window 2!"].center() => Some(SizeHint::Percentage(0.4))],
                bordered![label!["Window 3!"].center() => Some(SizeHint::Percentage(0.6))],
                // No sizing defaults to Fill, making this automatically take 60% of parent size
                => None
            ],
            // No sizing for the root element, it will fill the full screen
            => None
        ],
        buf,
    )?;

    ui.add_float(Float {
        contents: bordered![label!["Float 1"] => None],
        rect: Rect {
            x: 10.,
            y: 10.,
            width: 15.,
            height: 10.,
        },
        z_index: 1,
    });
    ui.add_float(Float {
        contents: bordered![label!["Float 2"] => None],
        rect: Rect {
            x: 60.,
            y: 20.,
            width: 20.,
            height: 10.,
        },
        z_index: 0,
    });

    ui.exec()?;

    Ok(())
}
