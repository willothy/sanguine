use std::{future::Future, task, time::Duration};

use anyhow::{anyhow, Result};
use dashmap::DashMap;
use std::task::Poll;
use taffy::{
    prelude::{Node, Size},
    style::{AvailableSpace, FlexDirection, Style},
    tree::LayoutTree,
    Taffy,
};
use termwiz::{
    input::InputEvent,
    surface::{Change, Position, Surface},
    terminal::{buffered::BufferedTerminal, new_terminal, Terminal},
};
use tokio::{
    io::{stdin, AsyncReadExt},
    time::interval,
};

static mut NEXT_BUFFER_HANDLE: usize = 0;

fn next_buffer_handle() -> BufferHandle {
    unsafe {
        let handle = NEXT_BUFFER_HANDLE;
        NEXT_BUFFER_HANDLE += 1;
        handle
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

pub type BufferHandle = usize;

pub struct Buffer {
    // TODO: How should buffers be represented? A rope is likely not needed as most
    // terminals will be fed line-by-line (negating most of the Rope's edit performance benefits).
    // But maybe raw terminals would benefit from a rope if we're able to render individual changes
    // to the buffer.
    inner: crop::Rope,
    id: BufferHandle,
}

impl Buffer {
    pub fn new() -> Self {
        let id = next_buffer_handle();
        Self {
            inner: crop::Rope::new(),
            id,
        }
    }

    pub fn from_str(s: &str) -> Self {
        let id = next_buffer_handle();
        Self {
            inner: crop::Rope::from(s),
            id,
        }
    }

    pub fn test() -> Self {
        Self::from_str(
            r#"Hello, world!
            Hello, world1!
            Hello, world2!
            Hello, world!
            Hello, world!
            Hello, world!
            Hello, world!
            Hello, world5!
            Hello, world6!
            Hello, world!
            Hello, world!
            Hello, world7!
            Hello, world8!
            Hello, world!
            Hello, world!
            Hello, world!"#,
        )
    }
}

pub struct Window {
    pub handle: Node,
    surface: Surface,
    buffer: BufferHandle,
    topline: usize,
}

pub struct WindowManager<T: Terminal> {
    pub root: Node,
    pub layout: Taffy,
    pub windows: DashMap<Node, Window>,
    pub buffers: DashMap<BufferHandle, Buffer>,
    pub floating: Vec<Node>,
    pub terminal: BufferedTerminal<T>,
}

pub enum SplitDirection {
    Left,
    Right,
    Up,
    Down,
}

impl SplitDirection {
    pub fn axis(&self) -> LayoutAxis {
        match self {
            SplitDirection::Left | SplitDirection::Right => LayoutAxis::Row,
            SplitDirection::Up | SplitDirection::Down => LayoutAxis::Col,
        }
    }
}

impl<T: Terminal> WindowManager<T> {
    pub fn new(terminal: T) -> Result<Self> {
        let mut layout = Taffy::new();
        let windows = DashMap::new();
        let buffer = Buffer::test();
        let node = layout.new_leaf(Style {
            size: Size::from_percent(1.0, 1.0),
            ..Default::default()
        })?;
        windows.insert(
            node,
            Window {
                surface: Surface::new(1, 1),
                handle: node,
                topline: 0,
                buffer: buffer.id,
            },
        );

        let root = layout.new_with_children(
            Style {
                size: Size::from_percent(1.0, 1.0),
                ..Default::default()
            },
            &[node],
        )?;

        let buffers = DashMap::new();
        buffers.insert(buffer.id, buffer);

        Ok(Self {
            root,
            layout,
            windows,
            buffers,
            floating: Vec::new(),
            terminal: BufferedTerminal::new(terminal)?,
        })
    }

    pub fn create_buffer(&mut self) -> BufferHandle {
        let buffer = Buffer::new();
        let id = buffer.id;
        self.buffers.insert(id, buffer);
        id
    }

    pub fn recompute_layout(&mut self) -> Result<()> {
        if self.layout.dirty(self.root)? {
            let (width, height) = self.terminal.dimensions();
            let size = Size {
                width: AvailableSpace::Definite(width as f32),
                height: AvailableSpace::Definite(height as f32),
            };
            self.layout.compute_layout(self.root, size)?;
        }
        Ok(())
    }

    pub fn frame_axis(&self, node: Node) -> Option<LayoutAxis> {
        use taffy::style::FlexDirection::*;
        if self.layout.is_childless(node) {
            return None;
        }
        match self.layout.style(node).ok()?.flex_direction {
            Row | RowReverse => Some(LayoutAxis::Row),
            Column | ColumnReverse => Some(LayoutAxis::Col),
        }
    }

    pub fn split(&mut self, handle: Node, direction: SplitDirection) -> Result<Node> {
        let buffer = self.create_buffer();
        let window = self
            .windows
            .get(&handle)
            .ok_or_else(|| anyhow!("Invalid window {handle:?}"))?;
        let parent = self.layout.parent(handle).unwrap();
        let handle_idx = self
            .layout
            .children(parent)?
            .iter()
            .position(|n| *n == handle)
            .unwrap();
        let parent_axis = self.frame_axis(parent).unwrap();
        let (axis, before) = match direction {
            SplitDirection::Left => (FlexDirection::Row, true),
            SplitDirection::Right => (FlexDirection::Row, false),
            SplitDirection::Up => (FlexDirection::Column, true),
            SplitDirection::Down => (FlexDirection::Column, false),
        };
        if parent_axis != direction.axis() {
            if self.layout.child_count(parent)? == 1 {
                let mut style = self.layout.style(parent).unwrap().to_owned();
                style.flex_direction = match style.flex_direction {
                    FlexDirection::Row => FlexDirection::Column,
                    FlexDirection::Column => FlexDirection::Row,
                    FlexDirection::RowReverse => FlexDirection::ColumnReverse,
                    FlexDirection::ColumnReverse => FlexDirection::RowReverse,
                };
                self.layout.set_style(parent, style)?;
            } else {
                let new_leaf = self.layout.new_leaf(Style {
                    size: Size::from_percent(1., 1.),
                    ..Default::default()
                })?;
                let new_win = Window {
                    surface: Surface::new(1, 1),
                    handle: new_leaf,
                    topline: 0,
                    buffer,
                };
                self.windows.insert(new_leaf, new_win);
                let new_frame = self.layout.new_with_children(
                    Style {
                        size: Size::from_percent(1., 1.),
                        flex_direction: axis,
                        ..Default::default()
                    },
                    &if before {
                        [new_leaf, handle]
                    } else {
                        [handle, new_leaf]
                    },
                )?;
                self.layout
                    .replace_child_at_index(parent, handle_idx, new_frame)?;
                return Ok(new_leaf);
            }
        }

        let new_node = self.layout.new_leaf(Style {
            size: Size::from_percent(1., 1.),
            ..Default::default()
        })?;
        let new_win = Window {
            surface: Surface::new(1, 1),
            handle: new_node,
            topline: 0,
            buffer,
        };
        self.windows.insert(new_node, new_win);

        let mut children = self.layout.children(parent)?;
        if before {
            children.insert(handle_idx, new_node);
        } else {
            if handle_idx == children.len() - 1 {
                children.push(new_node);
            } else {
                children.insert(handle_idx + 1, new_node);
            }
        }
        self.layout.set_children(parent, &children)?;

        Ok(new_node)
    }

    pub fn render(&mut self) -> Result<()> {
        self.recompute_layout()?;

        let surface = &mut self.terminal;

        let mut stack = vec![self.root];
        loop {
            let node = match stack.pop() {
                Some(node) if self.layout.is_childless(node) => node,
                Some(node) => {
                    stack.extend(self.layout.children(node)?.iter().rev());
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
            let buffer = self
                .buffers
                .get(&win.buffer)
                .ok_or_else(|| anyhow!("Invalid buffer handle {}", win.buffer))?;

            let buffer = &buffer.inner.line_slice(
                win.topline
                    ..(win.topline + win_dims.1)
                        .saturating_sub(2)
                        .max(win.topline)
                        .min(buffer.inner.line_len()),
            );

            for (i, line) in buffer.lines().enumerate() {
                win.surface.add_changes(vec![
                    Change::CursorPosition {
                        x: Position::Absolute(2),
                        y: Position::Absolute(i + 1),
                    },
                    Change::Text(
                        line.chars()
                            .skip_while(|c| c.is_whitespace())
                            .take(win_dims.0 - 2)
                            .collect::<String>(),
                    ),
                ]);
            }

            surface.draw_from_screen(
                &win.surface,
                dims.location.x.round() as usize,
                dims.location.y.round() as usize,
            );
        }

        surface.flush()?;

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
    let mut wm = WindowManager::new(term)?;
    wm.render()?;
    wm.split(wm.layout.child_at_index(wm.root, 0)?, SplitDirection::Right)?;
    std::thread::sleep(std::time::Duration::from_secs(1));
    wm.render()?;
    let input = stdin().read_u8().await?;
    // let input = buffered.terminal().poll_input_async().await?;
    println!("Got input {:?}", input);
    Ok(())
}
