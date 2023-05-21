use std::sync::{Arc, RwLock};

use crate::{
    allocator::{NodeId, Slab},
    Widget,
};

use super::{Direction, LayoutNode, Rect};

pub struct Floating<U> {
    /// The widget to be rendered
    widget: Arc<RwLock<dyn Widget<U>>>,
    /// Position and size of the floating window
    pos: Rect,
    /// Z-index of the window (only applies when not focused)
    z_index: usize,
    /// Whether the window can be focused
    #[allow(unused)]
    focusable: bool,
}

impl<U> Floating<U> {
    pub fn new(widget: impl Widget<U> + 'static, pos: Rect) -> Self {
        Self {
            widget: Arc::new(RwLock::new(widget)),
            pos,
            z_index: 1,
            focusable: true,
        }
    }

    pub fn from_widget(widget: Arc<RwLock<dyn Widget<U>>>, pos: Rect) -> Self {
        Self {
            widget,
            pos,
            z_index: 1,
            focusable: true,
        }
    }

    pub fn with_z_index(self, z_index: usize) -> Self {
        Self { z_index, ..self }
    }

    pub fn z_index(&self) -> usize {
        self.z_index
    }

    pub fn focusable(self) -> Self {
        Self {
            focusable: true,
            ..self
        }
    }

    pub fn widget(&self) -> Arc<RwLock<dyn Widget<U>>> {
        self.widget.clone()
    }

    pub fn set_focusable(&mut self, focusable: bool) {
        self.focusable = focusable;
    }

    pub fn move_to(&mut self, pos: (usize, usize)) {
        self.pos.x = pos.0 as f32;
        self.pos.y = pos.1 as f32;
    }

    pub fn no_focus(self) -> Self {
        Self {
            focusable: false,
            ..self
        }
    }

    pub fn move_dir(&mut self, direction: Direction) {
        match direction {
            Direction::Up => self.pos.y -= 1.,
            Direction::Down => self.pos.y += 1.,
            Direction::Left => self.pos.x -= 1.,
            Direction::Right => self.pos.x += 1.,
        }
    }
}

pub struct FloatStack<U> {
    inner: Vec<NodeId>,
    marker: std::marker::PhantomData<U>,
}

impl<U> FloatStack<U> {
    pub fn new() -> Self {
        Self {
            inner: Vec::new(),
            marker: std::marker::PhantomData,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &NodeId> {
        self.inner.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut NodeId> {
        self.inner.iter_mut()
    }

    pub fn remove(&mut self, node: NodeId) {
        self.inner.retain(|v| *v != node);
    }

    pub fn sort(&mut self, nodes: &Slab<LayoutNode<U>>) {
        self.inner.sort_by(|a, b| {
            nodes
                .get(*b)
                .map(|v| v.floating().unwrap().z_index)
                .unwrap_or(1)
                .cmp(
                    &nodes
                        .get(*a)
                        .map(|v| v.floating().unwrap().z_index)
                        .unwrap_or(1),
                )
        })
    }

    pub fn push(&mut self, node: NodeId, nodes: &Slab<LayoutNode<U>>) {
        self.inner.push(node);
        self.sort(nodes);
    }

    pub fn pop(&mut self, nodes: &Slab<LayoutNode<U>>) -> Option<NodeId> {
        if let Some(res) = self.inner.pop() {
            self.sort(nodes);
            return Some(res);
        }
        None
    }

    pub fn peek(&self) -> Option<NodeId> {
        self.inner.last().copied()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}
