use std::{future::Future, task, time::Duration};

use anyhow::{anyhow, Result};
use dashmap::{DashMap, DashSet};
use geometry::{Layout, Point, Size};
use std::task::Poll;
use taffy::{
    prelude::Node,
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

pub mod geometry;

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

pub type BufferHandle = usize;

static mut NEXT_BUFFER_HANDLE: BufferHandle = 0;

fn next_buffer_handle() -> BufferHandle {
    unsafe {
        NEXT_BUFFER_HANDLE += 1;
        NEXT_BUFFER_HANDLE
    }
}

// TODO: Transition from buffers to a more general concept of a view that can be displayed in a
// window. This should provide interfaces for handling input and events, rendering, etc.
//
// Buffers should be program-specific - Sanguine should not care about the consumer's
// implementation of View internals.
//
// Notes:
// Should views be referenced externally, or should they be owned by the WindowManager? If they
// are owned by the WindowManager, then the WindowManager can handle the lifetime of the view and
// ensure that it is properly cleaned up when the window is closed. If they are referenced
// externally, then the WindowManager can be more lightweight, but views will likely need to be
// reference counted to ensure that they are not dropped while still in use.
//
// If views are owned by the WindowManager, the interface will likely be more complex since
// any view-specific methods will need to be exposed through the WindowManager via type-safe
// handles.
//
// In either scenario, the WindowManager needs to have some idea of the current contents of a view in a window,
// so that it can render the view to the window surface. This could be done by:
// - Re-rendering the entire view for each window directly on re-render, relying
//   on the view's implementation to preserve state across renders.
// - Maintaining a separate buffer for each window, and rendering the view to the window's buffer.
//   - Would require some way to synchronize the view's buffer with the window's buffer.
//   - Buffer representation TBD: Rope, Vec<String> etc.
pub struct Buffer {
    inner: crop::Rope,
    id: BufferHandle,
}

impl<T> From<T> for Buffer
where
    T: AsRef<str>,
{
    fn from(value: T) -> Self {
        let id = next_buffer_handle();
        Self {
            inner: crop::Rope::from(value.as_ref()),
            id,
        }
    }
}

impl Buffer {
    pub fn new() -> Self {
        let id = next_buffer_handle();
        Self {
            inner: crop::Rope::new(),
            id,
        }
    }

    pub fn test() -> Self {
        Self::from(
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
    pub floating: DashSet<Node>,
    pub terminal: BufferedTerminal<T>,
}

impl<T: Terminal> WindowManager<T> {
    pub fn new(terminal: T) -> Result<Self> {
        let mut layout = Taffy::new();
        let windows = DashMap::new();
        let buffer = Buffer::test();
        let node = layout.new_leaf(Style {
            size: taffy::prelude::Size::percent(1.),
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
                size: taffy::prelude::Size::percent(1.),
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
            floating: DashSet::new(),
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
            let size = taffy::prelude::Size {
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
                    size: taffy::prelude::Size::percent(1.),
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
            size: taffy::prelude::Size::percent(1.),
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

    pub fn close_window(&mut self, window: Node) -> Result<()> {
        if self.windows.contains_key(&window) {
            self.windows.remove(&window);
            if let None = self.floating.remove(&window) {
                let parent = self.layout.parent(window).expect("Window has no parent");
                self.layout.remove(window)?;
                self.layout.mark_dirty(parent)?;
            }
            return Ok(());
        }
        Err(anyhow!("Invalid window {window:?}"))
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

    pub fn open_float(
        &mut self,
        buffer: Option<BufferHandle>,
        position: Point,
        size: Size,
    ) -> Result<Node> {
        let buffer = match buffer {
            Some(buffer) => buffer,
            None => self.create_buffer(),
        };
        let node = self.create_window(buffer)?;
        self.update_style(node, |style| {
            style.position = Position::Absolute;
            style.size = size.into();
            style.margin = position.as_margin();
        })?;
        self.floating.insert(node);
        self.layout.add_child(self.root, node)?;
        Ok(node)
    }

    pub fn open_split(
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
            if self.non_floating_child_count(parent)? == 1 {
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

    fn draw_win_view(&self, win: Node, width: usize, height: usize) -> Result<()> {
        let mut win = self
            .windows
            .get_mut(&win)
            .ok_or_else(|| anyhow!("Invalid window"))?;
        if win.surface.dimensions() != (width, height) {
            win.surface.resize(width, height);
        }
        win.surface
            .add_change(Change::ClearScreen(termwiz::color::ColorAttribute::Default));
        win.surface.draw_border();
        let (width, height) = win.surface.dimensions();
        let buffer = self
            .buffers
            .get(&win.buffer)
            .ok_or_else(|| anyhow!("Invalid buffer {} for window", win.buffer))?;

        // FIXME: This likely will not work properly for wide characters or escape sequences.
        let view = &buffer.inner.line_slice(
            win.topline
                ..(win.topline + height)
                    .saturating_sub(2)
                    .max(win.topline)
                    .min(buffer.inner.line_len()),
        );

        for (i, line) in view.lines().enumerate() {
            win.surface.add_changes(vec![
                Change::CursorPosition {
                    x: termwiz::surface::Position::Absolute(2),
                    y: termwiz::surface::Position::Absolute(i + 1),
                },
                Change::Text(
                    line.chars()
                        .skip_while(|c| c.is_whitespace())
                        .take(width - 2)
                        .collect::<String>(),
                ),
            ]);
        }

        Ok(())
    }

    pub fn render(&mut self) -> Result<()> {
        self.recompute_layout()?;

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
            let dims = *self.layout.layout(node).map_err(|e| anyhow!("{e}"))?;

            let Layout {
                location:
                    Point {
                        x: local_x,
                        y: local_y,
                    },
                size: geometry::Size { width, height },
            } = dims.into();
            self.draw_win_view(node, width, height)?;

            // Dimensions are in the parent's local space, so we need to add the parent's location
            // to translate them to screen space.
            let geometry::Point {
                x: parent_x,
                y: parent_y,
            } = self
                .layout
                .parent(node)
                .and_then(|parent| Some(self.layout.layout(parent).ok()?.location.into()))
                .expect("Window has no parent");
            let translated_x = parent_x + local_x;
            let translated_y = parent_y + local_y;

            if let Some(win) = self.windows.get(&node) {
                self.terminal
                    .draw_from_screen(&win.surface, translated_x, translated_y);
            }
        }

        self.terminal.flush()?;

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

    pub fn non_floating_child_count(&self, frame: Node) -> Result<usize> {
        let mut count = 0;
        for child in self.layout.children(frame)? {
            if !self.floating.contains(&child) {
                count += 1;
            }
        }
        Ok(count)
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
    wm.open_split(
        wm.layout.child_at_index(wm.root, 0)?,
        SplitDirection::Right,
        Some(buf),
    )?;
    wm.render()?;
    std::thread::sleep(std::time::Duration::from_secs(1));
    let win3 = wm.open_split(
        wm.layout.child_at_index(wm.root, 1)?,
        SplitDirection::Above,
        Some(buf),
    )?;
    wm.render()?;
    std::thread::sleep(std::time::Duration::from_secs(1));
    wm.close_window(win3)?;
    wm.render()?;
    std::thread::sleep(std::time::Duration::from_secs(1));
    wm.open_float(
        Some(buf),
        Point { x: 10, y: 2 },
        Size {
            width: 25,
            height: 8,
        },
    )?;
    wm.render()?;
    let _ = stdin().read_u8().await?;
    println!("\n\r");
    wm.print_layout(None, &wm.windows);

    Ok(())
}
