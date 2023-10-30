use std::{
    future::Future,
    os::fd::{AsRawFd, FromRawFd, RawFd},
    task,
    time::Duration,
};

use anyhow::{anyhow, Result};
use dashmap::DashMap;
use slotmap::{new_key_type, SlotMap};
use std::task::Poll;
use stretch::{
    geometry::Size,
    node::{MeasureFunc, Node},
    number::Number,
    style::{Dimension, Direction, FlexDirection, JustifyContent, Style},
    Stretch,
};
use termwiz::{
    input::InputEvent,
    surface::{Change, Position, Surface},
    terminal::{buffered::BufferedTerminal, new_terminal, Terminal},
};
use tokio::{
    io::{stdin, AsyncReadExt},
    net::unix::OwnedReadHalf,
    time::interval,
};

pub enum LayoutAxis {
    Row,
    Col,
}

pub enum Frame {
    Leaf {
        node: Node,
    },
    Container {
        layout: LayoutAxis,
        children: Vec<Frame>,
        node: Node,
    },
}

impl Frame {
    pub fn node(&self) -> Node {
        match *self {
            Frame::Leaf { node, .. } => node,
            Frame::Container { node, .. } => node,
        }
    }
}

new_key_type! {
    pub struct WindowId;
}

trait ToAnyhow<T, E> {
    fn to_anyhow(self) -> Result<T, anyhow::Error>;
}

impl<T> ToAnyhow<T, stretch::Error> for Result<T, stretch::Error> {
    fn to_anyhow(self) -> Result<T, anyhow::Error> {
        self.map_err(|e| anyhow!("{e}"))
    }
}

pub struct Window {
    pub handle: Node,
    surface: Surface,
}

pub struct WindowManager {
    pub root: Frame,
    pub layout: Stretch,
    pub windows: DashMap<Node, Window>,
    win_parents: DashMap<Node, Node>,
    pub floating: Vec<WindowId>,
}

impl WindowManager {
    pub fn new() -> Self {
        let mut layout = Stretch::new();
        let mut windows = DashMap::new();
        let node = layout
            .new_leaf(
                Style {
                    size: Size {
                        width: Dimension::Percent(1.0),
                        height: Dimension::Percent(1.0),
                    },
                    ..Default::default()
                },
                Box::new(|s| {
                    let width = match s.width {
                        Number::Defined(w) => w,
                        Number::Undefined => 0.0,
                    };
                    let height = match s.height {
                        Number::Defined(h) => h,
                        Number::Undefined => 0.0,
                    };
                    Ok(Size { width, height })
                }),
            )
            .unwrap();
        let first_win = windows.insert(
            node,
            Window {
                surface: Surface::new(1, 1),
                handle: node,
            },
        );
        let root = Frame::Leaf { node };
        Self {
            root,
            windows,
            layout,
            floating: Vec::new(),
        }
    }

    pub fn recompute_layout(&mut self) -> Result<()> {
        if self.layout.dirty(self.root.node()).to_anyhow()? {
            self.layout
                .compute_layout(self.root.node(), Size::undefined())
                .to_anyhow()?;
        }
        todo!()
    }

    pub fn split(&mut self, handle: Node) -> Result<()> {
        self.recompute_layout()?;
        let window = self
            .windows
            .get(&handle)
            .ok_or_else(|| anyhow!("Invalid window {handle:?}"))?;
        // let layout = self.layout.layout(handle).to_anyhow()?;
        // let parent = handle.

        // let new_node =self.layout.new_lea

        Ok(())
    }

    pub fn render(&mut self, surface: &mut Surface) -> Result<()> {
        let (width, height) = surface.dimensions();
        let size = Size {
            width: Number::Defined(width as f32),
            height: Number::Defined(height as f32),
        };

        self.layout
            .compute_layout(self.root.node(), size)
            .map_err(|e| anyhow!("{e}"))?;

        let mut stack = vec![&self.root];
        loop {
            let node = match stack.pop() {
                Some(Frame::Leaf { node }) => *node,
                Some(Frame::Container { children, .. }) => {
                    stack.extend(children.iter().rev());
                    continue;
                }
                None => break,
            };
            let dims = self.layout.layout(node).map_err(|e| anyhow!("{e}"))?;
            let Some(mut win) = self.windows.get_mut(&node) else {
                continue;
            };
            let win_dims = (
                dims.size.width.round() as usize,
                dims.size.height.round() as usize,
            );
            win.surface.resize(win_dims.0, win_dims.1);
            win.surface.draw_border();
            surface.draw_from_screen(
                &win.surface,
                dims.location.x.round() as usize,
                dims.location.y.round() as usize,
            );
        }

        Ok(())
    }
}

pub trait DrawBorder {
    const HORIZONTAL: char = '─';
    const VERTICAL: char = '│';
    const TOP_LEFT: char = '┌';
    const TOP_RIGHT: char = '┐';
    const BOTTOM_LEFT: char = '└';
    const BOTTOM_RIGHT: char = '┘';

    fn draw_border(&mut self);
}

impl DrawBorder for Surface {
    fn draw_border(&mut self) {
        let (w, h) = self.dimensions();

        let horz = std::iter::repeat(Self::HORIZONTAL)
            .take(w.saturating_sub(2))
            .collect::<String>();

        let (x, y) = self.cursor_position();

        self.add_changes(vec![
            Change::CursorPosition {
                x: Position::Absolute(0),
                y: Position::Absolute(0),
            },
            Change::Text(format!("{}{}{}", Self::TOP_LEFT, horz, Self::TOP_RIGHT)),
        ]);

        for y in 1..h - 1 {
            self.add_changes(vec![
                Change::CursorPosition {
                    x: Position::Absolute(0),
                    y: Position::Absolute(y),
                },
                Change::Text(format!("{}", Self::VERTICAL)),
                Change::CursorPosition {
                    x: Position::Absolute(w.saturating_sub(1)),
                    y: Position::Absolute(y),
                },
                Change::Text(format!("{}", Self::VERTICAL)),
            ]);
        }

        self.add_changes(vec![
            Change::CursorPosition {
                x: Position::Absolute(0),
                y: Position::Absolute(h.saturating_sub(1)),
            },
            Change::Text(format!(
                "{}{}{}",
                Self::BOTTOM_LEFT,
                horz,
                Self::BOTTOM_RIGHT
            )),
            Change::CursorPosition {
                x: Position::Absolute(x),
                y: Position::Absolute(y),
            },
        ]);
    }
}

struct PollInput<'a, T>(&'a mut T, tokio::time::Interval)
where
    T: termwiz::terminal::Terminal + Send;

impl<'a, T> Future for PollInput<'a, T>
where
    T: termwiz::terminal::Terminal + Send,
{
    type Output = Result<InputEvent>;

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Self::Output> {
        match self.0.poll_input(Some(Duration::ZERO)) {
            Ok(Some(v)) => Poll::Ready(Ok(v)),
            Ok(None) => {
                if self.1.poll_tick(cx).is_ready() {
                    cx.waker().wake_by_ref();
                }
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e.into())),
        }
    }
}

#[async_trait::async_trait]
pub trait PollInputAsync<T>
where
    T: termwiz::terminal::Terminal + Send,
{
    async fn poll_input_async(&mut self) -> Result<InputEvent>;
}

#[async_trait::async_trait]
impl<T> PollInputAsync<T> for T
where
    T: termwiz::terminal::Terminal + Send,
{
    async fn poll_input_async(&mut self) -> Result<InputEvent> {
        PollInput(self, interval(Duration::from_millis(1))).await
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let caps = termwiz::caps::Capabilities::new_from_env()?;
    let mut term = new_terminal(caps)?;
    term.set_raw_mode()?;
    let mut buffered = BufferedTerminal::new(term)?;
    let mut wm = WindowManager::new();
    wm.render(&mut buffered)?;
    buffered.flush()?;
    let input = stdin().read_u8().await?;
    // let input = buffered.terminal().poll_input_async().await?;
    println!("Got input {:?}", input);
    Ok(())
}
