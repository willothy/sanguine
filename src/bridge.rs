#![cfg(feature = "tui")]

use termwiz::{
    cell::{CellAttributes, Intensity, Underline},
    color::{AnsiColor, ColorAttribute, RgbColor},
    surface::{Change, Position, Surface},
};

/// Bridge for implementing backends for other TUI libraries
///
/// Required since `Surface` isn't implemented in this crate.
pub struct BridgeInner<'a>(&'a mut Surface);

pub trait Bridge {
    fn bridge<'a>(&'a mut self) -> ratatui::Terminal<BridgeInner<'a>>;
}

impl Bridge for &mut Surface {
    fn bridge<'a>(&'a mut self) -> ratatui::Terminal<BridgeInner<'a>> {
        ratatui::Terminal::new(BridgeInner(self)).expect("this should not fail")
    }
}

struct TuiColor(ratatui::style::Color);

impl Into<ColorAttribute> for TuiColor {
    fn into(self) -> ColorAttribute {
        use ratatui::style::Color::*;
        match self {
            TuiColor(Reset) => ColorAttribute::Default,
            TuiColor(Black) => AnsiColor::Black.into(),
            TuiColor(Red) => AnsiColor::Maroon.into(),
            TuiColor(Green) => AnsiColor::Green.into(),
            TuiColor(Yellow) => AnsiColor::Olive.into(),
            TuiColor(Blue) => AnsiColor::Navy.into(),
            TuiColor(Magenta) => AnsiColor::Purple.into(),
            TuiColor(Cyan) => AnsiColor::Teal.into(),
            TuiColor(Gray) => AnsiColor::Grey.into(),
            TuiColor(DarkGray) => AnsiColor::Grey.into(),
            TuiColor(LightRed) => AnsiColor::Red.into(),
            TuiColor(LightGreen) => AnsiColor::Lime.into(),
            TuiColor(LightYellow) => AnsiColor::Yellow.into(),
            TuiColor(LightBlue) => AnsiColor::Blue.into(),
            TuiColor(LightMagenta) => AnsiColor::Fuchsia.into(),
            TuiColor(LightCyan) => AnsiColor::Aqua.into(),
            TuiColor(White) => AnsiColor::White.into(),
            TuiColor(Rgb(r, g, b)) => {
                ColorAttribute::TrueColorWithDefaultFallback(RgbColor::new_8bpc(r, g, b).into())
            }
            TuiColor(Indexed(idx)) => ColorAttribute::PaletteIndex(idx),
        }
    }
}

impl<'surface> ratatui::backend::Backend for BridgeInner<'surface> {
    fn draw<'a, I>(&mut self, content: I) -> std::result::Result<(), std::io::Error>
    where
        I: Iterator<Item = (u16, u16, &'a ratatui::buffer::Cell)>,
    {
        for (x, y, cell) in content {
            let style = cell.style();
            let fg = style
                .fg
                .map(|v| TuiColor(v).into())
                .unwrap_or(ColorAttribute::Default);
            let bg = style
                .bg
                .map(|v| TuiColor(v).into())
                .unwrap_or(ColorAttribute::Default);
            let modifier = style.add_modifier;
            let bold = modifier.contains(ratatui::style::Modifier::BOLD);
            let italic = modifier.contains(ratatui::style::Modifier::ITALIC);
            let _dim = modifier.contains(ratatui::style::Modifier::DIM);
            let underline = modifier.contains(ratatui::style::Modifier::UNDERLINED);
            let reverse = modifier.contains(ratatui::style::Modifier::REVERSED);
            let hidden = modifier.contains(ratatui::style::Modifier::HIDDEN);
            let strikethrough = modifier.contains(ratatui::style::Modifier::CROSSED_OUT);
            let slow_blink = modifier.contains(ratatui::style::Modifier::SLOW_BLINK);
            let rapid_blink = modifier.contains(ratatui::style::Modifier::RAPID_BLINK);

            // add style
            let mut attr = CellAttributes::default();
            attr.set_foreground(fg);
            attr.set_background(bg);
            attr.set_intensity(if bold {
                Intensity::Bold
            } else {
                Intensity::Normal
            });
            attr.set_italic(italic);
            // todo: dim
            attr.set_underline(if underline {
                Underline::Single
            } else {
                Underline::None
            });
            attr.set_reverse(reverse);
            attr.set_invisible(hidden);
            attr.set_strikethrough(strikethrough);
            attr.set_blink(match (slow_blink, rapid_blink) {
                (_, true) => termwiz::cell::Blink::Rapid,
                (true, false) => termwiz::cell::Blink::Slow,
                (false, false) => termwiz::cell::Blink::None,
            });

            // set position
            self.0.add_changes(vec![
                // Set cursor position
                Change::CursorPosition {
                    x: Position::Absolute(x as usize),
                    y: Position::Absolute(y as usize),
                },
                // Set the style
                Change::AllAttributes(attr),
                // Write the text
                Change::Text(cell.symbol.clone()),
                // Reset attributes
                Change::AllAttributes(CellAttributes::default()),
            ]);
        }
        Ok(())
    }

    fn hide_cursor(&mut self) -> std::result::Result<(), std::io::Error> {
        Ok(())
    }

    fn show_cursor(&mut self) -> std::result::Result<(), std::io::Error> {
        Ok(())
    }

    fn get_cursor(&mut self) -> std::result::Result<(u16, u16), std::io::Error> {
        let pos = self.0.cursor_position();
        Ok((pos.0 as u16, pos.1 as u16))
    }

    fn set_cursor(&mut self, x: u16, y: u16) -> std::result::Result<(), std::io::Error> {
        self.0.add_change(Change::CursorPosition {
            x: Position::Absolute(x as usize),
            y: Position::Absolute(y as usize),
        });
        Ok(())
    }

    fn clear(&mut self) -> std::result::Result<(), std::io::Error> {
        self.0
            .add_change(Change::ClearScreen(ColorAttribute::default()));
        Ok(())
    }

    fn size(&self) -> std::result::Result<ratatui::layout::Rect, std::io::Error> {
        let dims = self.0.dimensions();
        Ok(ratatui::layout::Rect::new(
            0,
            0,
            dims.0 as u16,
            dims.1 as u16,
        ))
    }

    fn flush(&mut self) -> std::result::Result<(), std::io::Error> {
        Ok(())
    }
}
