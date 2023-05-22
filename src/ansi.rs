#![cfg(feature = "ansi")]
//! Utility function for parsing ansi escape sequences and writing the result to a [`Surface`]

use ansi_to_tui::IntoText;
use termwiz::{
    cell::CellAttributes,
    surface::{Change, Position, Surface},
};

use crate::{
    bridge::TuiStyle,
    error::{Error, Result},
};

/// Parse ansi text from the provided string using [`ansi_to_tui`], and write the result onto the
/// specified surface
pub fn write_ansi(screen: &mut Surface, bytes: &str) -> Result<()> {
    let text = bytes.into_text().map_err(Error::external)?;
    text.lines.into_iter().for_each(|l| {
        l.0.into_iter().for_each(|span| {
            let content = span.content;
            let style = span.style;
            let style: CellAttributes = TuiStyle(style).into();
            screen.add_changes(vec![
                Change::AllAttributes(style),
                Change::Text(content.to_string()),
                Change::AllAttributes(CellAttributes::default()),
            ]);
        });
        screen.add_changes(vec![
            Change::CursorPosition {
                x: Position::Absolute(0),
                y: Position::Relative(1),
            },
            Change::AllAttributes(CellAttributes::default()),
        ]);
    });
    Ok(())
}
