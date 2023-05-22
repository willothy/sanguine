#![doc = include_str!("../README.md")]
//!
//!```
#![doc = include_str!("../examples/demo.rs")]
//!```

/// Re-exports relating to [`termwiz::surface::Surface`]
pub mod surface {
    pub use termwiz::surface::{Change, CursorShape, CursorVisibility, Position, Surface};
    pub use termwiz::terminal::Terminal;

    pub(crate) mod term {
        pub use termwiz::caps::Capabilities;
        pub use termwiz::terminal::{buffered::BufferedTerminal, UnixTerminal};
    }
}

/// Re-exports from [`termwiz`] relating to text style
pub mod style {
    pub use termwiz::{
        cell::{CellAttributes, Intensity, Underline},
        color::{AnsiColor, ColorAttribute, RgbColor},
    };
}

pub use app::*;
pub use layout::Layout;
pub use widget::Widget;

pub mod ansi;
mod app;
pub mod bridge;
pub mod error;
pub mod event;
pub mod layout;
mod widget;
pub mod widgets;
