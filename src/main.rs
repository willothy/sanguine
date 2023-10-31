use std::{future::Future, task, time::Duration};

use anyhow::{anyhow, Result};
use dashmap::DashMap;
use std::task::Poll;
use taffy::{
    prelude::{Node, Size},
    style::{AvailableSpace, Display, FlexDirection, Position, Style},
    tree::LayoutTree,
    Taffy,
};
use termwiz::{
    input::InputEvent,
    surface::{Change, Surface},
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

impl From<FlexDirection> for LayoutAxis {
    fn from(value: FlexDirection) -> Self {
        match value {
            FlexDirection::Row => Self::Row,
            FlexDirection::Column => Self::Col,
            FlexDirection::RowReverse => Self::Row,
            FlexDirection::ColumnReverse => Self::Col,
        }
    }
}

impl std::fmt::Display for LayoutAxis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LayoutAxis::Row => write!(f, "row"),
            LayoutAxis::Col => write!(f, "col"),
        }
    }
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
    Above,
    Below,
}

impl SplitDirection {
    pub fn axis(&self) -> LayoutAxis {
        match self {
            SplitDirection::Left | SplitDirection::Right => LayoutAxis::Row,
            SplitDirection::Above | SplitDirection::Below => LayoutAxis::Col,
        }
    }

    pub fn is_before(&self) -> bool {
        match self {
            SplitDirection::Left | SplitDirection::Above => true,
            SplitDirection::Right | SplitDirection::Below => false,
        }
    }
}

impl Into<FlexDirection> for SplitDirection {
    fn into(self) -> FlexDirection {
        use SplitDirection::*;
        match self {
            Left | Right => FlexDirection::Row,
            Above | Below => FlexDirection::Column,
        }
    }
}

impl Into<FlexDirection> for LayoutAxis {
    fn into(self) -> FlexDirection {
        match self {
            LayoutAxis::Row => FlexDirection::Row,
            LayoutAxis::Col => FlexDirection::Column,
        }
    }
}

impl<T: Terminal> WindowManager<T> {
    pub fn new(terminal: T) -> Result<Self> {
        let mut layout = Taffy::new();
        let windows = DashMap::new();
        let buffer = Buffer::test();
        let node = layout.new_leaf(Style {
            size: Size::percent(1.),
            position: Position::Relative,
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
                size: Size::percent(1.),
                position: Position::Relative,
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
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

    pub fn create_buffer(&self) -> BufferHandle {
        let buffer = Buffer::new();
        let id = buffer.id;
        self.buffers.insert(id, buffer);
        id
    }

    pub fn delete_buffer(&mut self, buffer: BufferHandle) -> Result<()> {
        if self.buffers.contains_key(&buffer) {
            let tmp_buf = self
                .buffers
                .iter()
                .find(|e| e.id != buffer)
                .map(|e| e.id)
                .unwrap_or_else(|| self.create_buffer());
            let mut used_tmp = false;
            self.windows.iter_mut().for_each(|mut win| {
                if win.buffer == buffer {
                    win.buffer = tmp_buf;
                    used_tmp = true;
                }
            });
            self.buffers.remove(&buffer);
            return Ok(());
        }
        Err(anyhow!("Invalid buffer handle {}", buffer))
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

    pub fn frame_axis(&self, frame: Node) -> Option<LayoutAxis> {
        use taffy::style::FlexDirection::*;
        if self.layout.is_childless(frame) {
            return None;
        }
        match self.layout.style(frame).ok()?.flex_direction {
            Row | RowReverse => Some(LayoutAxis::Row),
            Column | ColumnReverse => Some(LayoutAxis::Col),
        }
    }

    fn create_frame(
        &mut self,
        direction: FlexDirection,
        children: impl AsRef<[Node]>,
    ) -> Result<Node> {
        self.layout
            .new_with_children(
                Style {
                    size: Size::percent(1.),
                    position: Position::Relative,
                    display: Display::Flex,
                    flex_direction: direction,
                    ..Default::default()
                },
                children.as_ref(),
            )
            .map_err(|e| anyhow!("{e}"))
    }

    fn create_window(&mut self, buffer: BufferHandle) -> Result<Node> {
        let node = self.layout.new_leaf(Style {
            size: Size::percent(1.),
            position: Position::Relative,
            ..Default::default()
        })?;
        self.windows.insert(
            node,
            Window {
                surface: Surface::new(1, 1),
                handle: node,
                topline: 0,
                buffer,
            },
        );
        Ok(node)
    }

    fn update_style(&mut self, node: Node, f: impl FnOnce(&mut Style)) -> Result<()> {
        let mut style = self.layout.style(node)?.clone();
        f(&mut style);
        self.layout.set_style(node, style)?;
        Ok(())
    }

    pub fn win_get_buffer(&self, window: Node) -> Result<BufferHandle> {
        if let Some(win) = self.windows.get(&window) {
            return Ok(win.buffer);
        }
        return Err(anyhow!("Invalid window {window:?}"));
    }

    pub fn win_set_buffer(&mut self, window: Node, buffer: BufferHandle) -> Result<()> {
        if !self.buffers.contains_key(&buffer) {
            return Err(anyhow!("Invalid buffer handle {}", buffer));
        }
        if let Some(mut win) = self.windows.get_mut(&window) {
            win.buffer = buffer;
            return Ok(());
        }
        return Err(anyhow!("Invalid window {window:?}"));
    }

    pub fn split(
        &mut self,
        handle: Node,
        direction: SplitDirection,
        buffer: Option<BufferHandle>,
    ) -> Result<Node> {
        let parent = self.layout.parent(handle).unwrap();
        let handle_idx = self.layout.child_index(handle).unwrap();
        let parent_axis = self.frame_axis(parent).unwrap();
        let axis = direction.axis();
        if parent_axis != axis {
            if self.layout.child_count(parent)? == 1 {
                self.update_style(parent, |style| {
                    style.flex_direction = match style.flex_direction {
                        FlexDirection::Row => FlexDirection::Column,
                        FlexDirection::Column => FlexDirection::Row,
                        FlexDirection::RowReverse => FlexDirection::ColumnReverse,
                        FlexDirection::ColumnReverse => FlexDirection::RowReverse,
                    };
                })?;
            } else {
                let new_leaf =
                    self.create_window(buffer.unwrap_or(self.win_get_buffer(handle)?))?;
                let new_frame = self.create_frame(axis.into(), [])?;
                self.layout
                    .replace_child_at_index(parent, handle_idx, new_frame)?;
                self.layout.set_children(
                    new_frame,
                    &if direction.is_before() {
                        [new_leaf, handle]
                    } else {
                        [handle, new_leaf]
                    },
                )?;
                return Ok(new_leaf);
            }
        }

        let new_win =
            self.create_window(buffer.unwrap_or_else(|| self.win_get_buffer(handle).unwrap()))?;

        let mut children = self.layout.children(parent)?;
        if direction.is_before() {
            children.insert(handle_idx, new_win);
        } else {
            if handle_idx == children.len() - 1 {
                children.push(new_win);
            } else {
                children.insert(handle_idx + 1, new_win);
            }
        }
        self.layout.set_children(parent, &children)?;

        Ok(new_win)
    }

    pub fn render(&mut self) -> Result<()> {
        use termwiz::surface::Position;
        self.recompute_layout()?;

        let surface = &mut self.terminal;
        surface.add_change(Change::ClearScreen(termwiz::color::ColorAttribute::Default));

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
            let (width, height) = (
                dims.size.width.round() as usize,
                dims.size.height.round() as usize,
            );
            win.surface.resize(width, height);
            win.surface.draw_border();
            let buffer = self
                .buffers
                .get(&win.buffer)
                .ok_or_else(|| anyhow!("Invalid buffer handle {}", win.buffer))?;

            let buffer = &buffer.inner.line_slice(
                win.topline
                    ..(win.topline + height)
                        .saturating_sub(2)
                        .max(win.topline)
                        .min(buffer.inner.line_len()),
            );

            for (i, line) in buffer.lines().enumerate() {
                win.surface.add_changes(vec![
                    Change::CursorPosition {
                        x: termwiz::surface::Position::Absolute(2),
                        y: Position::Absolute(i + 1),
                    },
                    Change::Text(
                        line.chars()
                            .skip_while(|c| c.is_whitespace())
                            .take(width - 2)
                            .collect::<String>(),
                    ),
                ]);
            }

            // Dimensions are in the parent's local space, so we need to add the parent's location
            // to translate them to screen space.
            // If there's no parent, we're a root window and can draw directly to the screen
            // without translation.
            let parent_dims = self
                .layout
                .parent(node)
                .and_then(|p| {
                    self.layout
                        .layout(p)
                        .map(|l| (l.location.x.round() as usize, l.location.y.round() as usize))
                        .ok()
                })
                .unwrap_or_else(|| (0, 0));

            surface.draw_from_screen(
                &win.surface,
                parent_dims.0 + dims.location.x.round() as usize,
                parent_dims.1 + dims.location.y.round() as usize,
            );
        }

        surface.flush()?;

        Ok(())
    }

    pub fn depth(&self, node: Node) -> usize {
        let mut depth = 0;
        let mut current = node;
        while let Some(parent) = self.layout.parent(current) {
            current = parent;
            depth += 1;
        }
        depth
    }

    pub fn windows(&self) -> impl Iterator<Item = Node> + '_ {
        struct Windows<'b> {
            inner: std::collections::VecDeque<Node>,
            layout: &'b Taffy,
        }
        impl<'b> Iterator for Windows<'b> {
            type Item = Node;

            fn next(&mut self) -> Option<Self::Item> {
                match self.inner.pop_front() {
                    Some(node) => {
                        if self.layout.is_childless(node) {
                            return Some(node);
                        }
                        for child in self.layout.children(node).unwrap() {
                            self.inner.push_back(child);
                        }
                        self.next()
                    }
                    None => None,
                }
            }
        }
        Windows {
            inner: std::collections::VecDeque::from([self.root]),
            layout: &self.layout,
        }
    }

    pub fn print_layout(&self, root: Option<Node>, wins: &DashMap<Node, Window>) {
        // print a tree structure of nodes

        let current = root.unwrap_or(self.root);
        if self.layout.is_childless(current) {
            println!(
                "{:indent$}{:?} : {:?} {}\r",
                "",
                current,
                self.layout.layout(current).unwrap(),
                self.windows.contains_key(&current),
                indent = self.depth(current) * 2
            );
            return;
        }

        println!(
            "{:indent$}{:?} : {} : {:?}\r",
            "",
            current,
            LayoutAxis::from(self.layout.style(current).unwrap().flex_direction),
            self.layout.layout(current).unwrap(),
            indent = self.depth(current) * 2,
        );

        let children = self.layout.children(current).unwrap();

        for child in children {
            self.print_layout(Some(child), wins);
        }
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
        use termwiz::surface::Position;
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

pub trait ChildIndex {
    fn child_index(&self, child: Node) -> Option<usize>;
}

impl ChildIndex for Taffy {
    fn child_index(&self, child: Node) -> Option<usize> {
        self.children(self.parent(child)?)
            .ok()?
            .iter()
            .position(|n| *n == child)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let caps = termwiz::caps::Capabilities::new_from_env()?;
    let mut term = new_terminal(caps)?;
    term.set_raw_mode()?;
    let mut wm = WindowManager::new(term)?;
    wm.render()?;
    let buf = wm.create_buffer();
    std::thread::sleep(std::time::Duration::from_secs(1));
    wm.split(
        wm.layout.child_at_index(wm.root, 0)?,
        SplitDirection::Right,
        Some(buf),
    )?;
    wm.render()?;
    std::thread::sleep(std::time::Duration::from_secs(1));
    let _win = wm.split(
        wm.layout.child_at_index(wm.root, 1)?,
        SplitDirection::Above,
        Some(buf),
    )?;
    wm.render()?;
    let _ = stdin().read_u8().await?;
    println!("\n\r");
    wm.print_layout(None, &wm.windows);

    Ok(())
}
