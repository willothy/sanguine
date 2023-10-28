use std::{future::Future, task, time::Duration};

use anyhow::Result;
use slotmap::{new_key_type, SlotMap};
use std::task::Poll;
use stretch::{
    geometry::Size,
    node::Node,
    number::Number,
    style::{Dimension, JustifyContent, Style},
    Stretch,
};
use termwiz::{
    input::InputEvent,
    surface::Surface,
    terminal::{buffered::BufferedTerminal, new_terminal, Terminal},
};

pub enum LayoutAxis {
    Row,
    Col,
}

pub enum Frame {
    Leaf {
        win: WindowId,
        node: Node,
    },
    Container {
        layout: LayoutAxis,
        children: Vec<Frame>,
        node: Node,
    },
}

new_key_type! {
    pub struct WindowId;
}

pub struct Window {
    pub handle: WindowId,
    surface: Surface,
}

pub struct WindowManager {
    pub root: Frame,
    pub layout: Stretch,
    pub windows: SlotMap<WindowId, Window>,
    pub floating: Vec<WindowId>,
}

impl WindowManager {
    pub fn new() -> Self {
        let mut layout = Stretch::new();
        let mut windows = SlotMap::<WindowId, _>::with_key();
        let first_win = windows.insert_with_key(|k| Window {
            handle: k,
            surface: Surface::new(1, 1),
        });
        let node = layout
            .new_node(
                Style {
                    size: Size {
                        width: Dimension::Auto,
                        height: Dimension::Auto,
                    },
                    ..Default::default()
                },
                vec![],
            )
            .unwrap();
        let root = Frame::Leaf {
            win: first_win,
            node,
        };
        Self {
            root,
            windows,
            layout,
            floating: Vec::new(),
        }
    }
}

struct PollInput<'a, T>(&'a mut T)
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
                cx.waker().wake_by_ref();
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
        PollInput(self).await
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let caps = termwiz::caps::Capabilities::new_from_env()?;
    let mut term = new_terminal(caps)?;
    term.set_raw_mode()?;
    let mut buffered = BufferedTerminal::new(term)?;
    let input = buffered.terminal().poll_input_async().await?;
    println!("Got input {:?}", input);
    Ok(())
}
