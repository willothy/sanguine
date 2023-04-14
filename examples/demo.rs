use red_tui::{Align, Border, BorderVariant, Label, Layout, Rect, Ui};
use termwiz::{
    caps::Capabilities,
    surface::Change,
    terminal::{buffered::BufferedTerminal, new_terminal, Terminal},
    Result,
};

fn main() -> Result<()> {
    let caps = Capabilities::new_from_env()?;
    let mut term = new_terminal(caps)?;
    // term.enter_alternate_screen()?;
    term.set_raw_mode()?;

    let mut buf = BufferedTerminal::new(term)?;
    buf.add_change(Change::CursorVisibility(
        termwiz::surface::CursorVisibility::Hidden,
    ));

    buf.flush()?;

    macro_rules! horizontal {
        [$($v:expr),*$(,)? => $s:expr] => {
            Layout::h(vec![
                $($v),*
            ], $s)
        };
    }

    macro_rules! vertical {
        [$($v:expr),*$(,)?=> $s:expr] => {
            Layout::v(vec![
                $($v),*
            ], $s)
        };
    }

    macro_rules! label {
        [$($v:expr),+$(,)?] => {
            Label::new(&format!($($v),+))
        };
    }

    macro_rules! bordered {
        ($v:expr => $s:expr) => {
            Box::new(Border::new(BorderVariant::Rounded, $v, $s))
        };
    }

    let _layouts = [
        horizontal![
            bordered![label!["Window 1!"].center() => Some(red_tui::SizeHint::Percentage(0.3))],
            bordered![label!["Window 2!"].center() => Some(red_tui::SizeHint::Percentage(0.7))],
            => None
        ],
        vertical![
            bordered![label!["Window 1!"].center() => None],
            bordered![label!["Window 2!"].center() => None],
            => None
        ],
        horizontal![
            vertical![
                bordered![label!["Window 1!"].center() => None],
                => None
            ],
            vertical![
                bordered![label!["Window 2!"].center() => None],
                bordered![label!["Window 3!"].center() => None],
                bordered![label!["Window 4!"].center() => None],
                => None
            ],
            => None
        ],
        horizontal![
            vertical![
                bordered![label!["Window 1!"].center() => None],
                bordered![label!["Window 2!"].center() => None],
                => None
            ],
            vertical![
                bordered![label!["Window 3!"].center() => None],
                => None
            ],
            => None
        ],
    ];

    let mut ui = Ui::new(
        horizontal![
            bordered![label!["Window 1!"].center() => Some(red_tui::SizeHint::Percentage(0.4))],
            vertical![
                bordered![label!["Window 2!"].center() => Some(red_tui::SizeHint::Percentage(0.4))],
                bordered![label!["Window 3!"].center() => Some(red_tui::SizeHint::Percentage(0.6))],
                => None
            ],
            => None
        ],
        buf,
    )?;

    ui.init()?;
    ui.add_float(red_tui::float::Float {
        contents: bordered![label!["Float 1"] => None],
        rect: Rect {
            x: 10.,
            y: 10.,
            width: 15.,
            height: 10.,
        },
        z_index: 1,
    });
    ui.add_float(red_tui::float::Float {
        contents: bordered![label!["Float 2"] => None],
        rect: Rect {
            x: 60.,
            y: 20.,
            width: 20.,
            height: 10.,
        },
        z_index: 0,
    });

    while ui.render()? {}

    Ok(())
}
