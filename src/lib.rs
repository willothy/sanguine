use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use anyhow::Result;
use slotmap::DefaultKey;
use termwiz::{
    caps::Capabilities,
    input::InputEvent,
    surface::Surface,
    terminal::{buffered::BufferedTerminal, UnixTerminal},
};

#[derive(Debug, Clone)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        self.width = width as f32;
        self.height = height as f32;
    }

    pub fn center(&self) -> (f32, f32) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        self.contains(other.x, other.y)
            || self.contains(other.x + other.width, other.y)
            || self.contains(other.x, other.y + other.height)
            || self.contains(other.x + other.width, other.y + other.height)
    }
}

impl Default for Rect {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction {
    Horizontal,
    Vertical,
}

pub type NodeId = DefaultKey;

pub trait Widget {
    fn render(&self, bounds: Rect, surface: &mut Surface);
    fn update(&mut self, _event: InputEvent) {}
    fn size_hint(&self) -> SizeHint {
        SizeHint::Fill
    }
}

pub struct Leaf {
    widget: Arc<dyn Widget>,
}

impl Leaf {
    pub fn new(widget: Arc<dyn Widget>) -> Self {
        Self { widget }
    }
}

#[derive(Debug, Clone)]
pub enum SizeHint {
    Fixed(usize),
    Percentage(f32),
    Fill,
}

impl SizeHint {
    pub fn fill() -> SizeHint {
        SizeHint::Fill
    }
}

#[derive(Debug)]
pub struct Container {
    direction: Direction,
    size: Option<SizeHint>,
    children: Vec<NodeId>,
}

pub enum LayoutNode {
    Container(Container),
    Leaf(Leaf),
}

pub struct Layout {
    /// The arena containing all nodes, keyed by unique id.
    nodes: slotmap::SlotMap<NodeId, LayoutNode>,
    /// Render results. Will be stale or zeroed if `Layout::compute()` isn't called after each
    /// change.
    layout: HashMap<NodeId, Rect>,
    /// The root node of the layout.
    root: NodeId,
}

impl Layout {
    pub fn new() -> Self {
        let mut nodes = slotmap::SlotMap::new();
        let root = nodes.insert(LayoutNode::Container(Container {
            direction: Direction::Vertical,
            size: None,
            children: vec![],
        }));
        Self {
            nodes,
            root,
            layout: HashMap::from([(root, Rect::default())]),
        }
    }

    pub fn compute(&mut self, bounds: &Rect) {
        self.compute_tree(None, bounds);
    }

    pub fn compute_tree(&mut self, node: Option<NodeId>, bounds: &Rect) {
        let node = node.unwrap_or(self.root());
        self.compute_node(node, bounds);
        if self.is_leaf(node) {
            return;
        } else {
            let children = self.children(node).unwrap().clone();
            children.iter().for_each(|id| {
                let bounds = self.layout(*id).unwrap().clone();
                self.compute_tree(Some(*id), &bounds);
            })
        }
    }

    /// Computes the layout of the tree. This must be called after each change to the tree.
    pub fn compute_node(&mut self, node: NodeId, bounds: &Rect) {
        self.layout.insert(node, bounds.clone());
        if self.is_leaf(node) {
            return;
        } else {
            // TODO: Handle size hints
            let children = self.children(node).unwrap();
            let axis = self.direction(node).unwrap();
            let sizes = children
                .iter()
                .map(|id| (*id, self.size(*id)))
                .collect::<Vec<_>>();

            let mut current = match &axis {
                Direction::Horizontal => bounds.x,
                Direction::Vertical => bounds.y,
            };
            self.compute_sizes(&bounds, &sizes, &axis)
                .iter()
                .for_each(|(k, v)| {
                    let size = match v {
                        SizeHint::Fixed(size) => *size as f32,
                        _ => unreachable!(),
                    };
                    let (width, height) = match &axis {
                        Direction::Horizontal => (size, bounds.height),
                        Direction::Vertical => (bounds.width, size),
                    };
                    let (x, y) = (
                        if axis == Direction::Horizontal {
                            // rect.x + width * i as f64
                            current
                        } else {
                            bounds.x
                        },
                        if axis == Direction::Vertical {
                            // rect.y + height * i as f64
                            current
                        } else {
                            bounds.y
                        },
                    );
                    let widget_rect = Rect {
                        x,
                        y,
                        width,
                        height,
                    };
                    current += size;
                    self.layout.insert(*k, widget_rect);
                });
        }
    }

    fn compute_sizes(
        &mut self,
        bounds: &Rect,
        sizes: &Vec<(NodeId, SizeHint)>,
        axis: &Direction,
    ) -> HashMap<NodeId, SizeHint> {
        let mut new_sizes = HashMap::new();
        let width = match axis {
            Direction::Horizontal => bounds.width,
            Direction::Vertical => bounds.height,
        };
        let mut remaining = width;

        let fixed = sizes
            .iter()
            .filter_map(|(k, size)| match size {
                SizeHint::Fixed(size) => {
                    // new_sizes.insert(i, SizeHint::Fixed(*size));
                    // new_sizes[&i] = (*k, SizeHint::Fixed(*size));
                    new_sizes.insert(*k, SizeHint::Fixed(*size));
                    Some(size)
                }
                _ => None,
            })
            .sum::<usize>();

        remaining -= fixed as f32;

        let mut percents = sizes
            .iter()
            .filter_map(|(k, size)| match size {
                SizeHint::Percentage(percent) => Some((k, *percent)),
                _ => None,
            })
            .collect::<Vec<_>>();
        let n_percent = percents.len();
        let percent = percents.iter().map(|(_, f)| f).sum::<f32>();

        if percent > 1.0 {
            let diff = percent - 1.0;
            let avg = diff / n_percent as f32;
            percents.iter_mut().for_each(|(_, f)| *f -= avg);
        }
        let mut pct_total = 0;
        percents.iter_mut().for_each(|(k, f)| {
            *f *= remaining as f32;
            let size = f.floor() as usize;
            pct_total += size;
            // new_sizes.insert(*i, SizeHint::Fixed(size));
            // new_sizes[*i] = (**k, SizeHint::Fixed(size));
            new_sizes.insert(**k, SizeHint::Fixed(size));
        });
        remaining -= pct_total as f32;

        let fill = sizes
            .iter()
            .enumerate()
            .filter_map(|(i, (k, size))| match size {
                SizeHint::Fill => Some((k, i)),
                _ => None,
            })
            .collect::<Vec<_>>();

        let nfill = fill.len();

        let fill_size = remaining / nfill as f32;
        fill.iter().for_each(|(k, _)| {
            // new_sizes.insert(*i, SizeHint::Fixed(fill_size.floor() as usize));
            // new_sizes[*i] = (**k, SizeHint::Fixed(fill_size.ceil() as usize));
            new_sizes.insert(**k, SizeHint::Fixed(fill_size.ceil() as usize));
        });
        new_sizes
    }

    pub fn size(&self, node: NodeId) -> SizeHint {
        match self.nodes.get(node) {
            Some(LayoutNode::Container(container)) => {
                container.size.clone().unwrap_or(SizeHint::Fill)
            }
            Some(LayoutNode::Leaf(leaf)) => leaf.widget.size_hint(),
            None => SizeHint::Fill,
        }
    }

    pub fn root(&self) -> NodeId {
        self.root
    }

    pub fn layout(&self, node: NodeId) -> Option<&Rect> {
        self.layout.get(&node)
    }

    pub fn leaves(&self) -> Vec<NodeId> {
        self.nodes
            .iter()
            .filter_map(|(id, node)| match node {
                LayoutNode::Leaf(_) => Some(id),
                _ => None,
            })
            .collect()
    }

    pub fn traverse(&self, f: impl Fn(NodeId, &LayoutNode)) {
        self.traverse_recursive(self.root, &f);
    }

    pub fn traverse_recursive(&self, node_id: NodeId, f: &impl Fn(NodeId, &LayoutNode)) {
        let node = self.nodes.get(node_id).unwrap();
        f(node_id, node);
        match node {
            LayoutNode::Container(container) => {
                for child in &container.children {
                    self.traverse_recursive(*child, f);
                }
            }
            LayoutNode::Leaf(_) => {}
        }
    }

    pub fn print_recursive(&self, node_id: NodeId) {
        let node = self.nodes.get(node_id).unwrap();
        match node {
            LayoutNode::Container(container) => {
                println!("Container: {:?}", self.layout(node_id));
                for child in &container.children {
                    self.print_recursive(*child);
                }
            }
            LayoutNode::Leaf(_) => println!("Leaf: {:?}", self.layout(node_id)),
        }
    }

    /// Drops a node from the layout. This will not drop children of the node.
    /// Use of the provided NodeId after calling this is invalid.
    pub fn remove_node(&mut self, node: NodeId) {
        self.nodes.remove(node);
        self.layout.remove(&node);
    }

    /// Adds a new container node to the layout.
    pub fn add_container(&mut self, direction: Direction, size: Option<SizeHint>) -> NodeId {
        let container = Container {
            children: vec![],
            direction,
            size,
        };
        let node = LayoutNode::Container(container);
        let id = self.nodes.insert(node);
        self.layout.insert(id, Rect::default());
        id
    }

    pub fn set_size(&mut self, node: NodeId, size: SizeHint) {
        match self.nodes.get_mut(node) {
            Some(LayoutNode::Container(container)) => {
                container.size = Some(size);
            }
            _ => {}
        }
    }

    pub fn set_direction(&mut self, node: NodeId, axis: Direction) {
        match self.nodes.get_mut(node) {
            Some(LayoutNode::Container(container)) => {
                container.direction = axis;
            }
            _ => {}
        }
    }

    /// Adds a new container node to the layout with the given children.
    pub fn add_with_children(
        &mut self,
        direction: Direction,
        size: Option<SizeHint>,
        children: impl Into<Vec<NodeId>>,
    ) -> NodeId {
        let container = Container {
            children: children.into(),
            direction,
            size,
        };
        let node = LayoutNode::Container(container);
        let id = self.nodes.insert(node);
        self.layout.insert(id, Rect::default());
        id
    }

    /// Adds a new leaf node to the layout.
    pub fn add_leaf(&mut self, leaf: Leaf) -> NodeId {
        let node = LayoutNode::Leaf(leaf);
        let id = self.nodes.insert(node);
        self.layout.insert(id, Rect::default());
        id
    }

    /// Adds a new leaf node to the given container.
    pub fn add_child(&mut self, parent: NodeId, child: NodeId) {
        match self.nodes.get_mut(parent) {
            Some(LayoutNode::Container(container)) => {
                container.children.push(child);
            }
            _ => panic!("Parent is not a container"),
        }
    }

    /// Removes a child from the given container. This does not drop the node.
    pub fn remove_child(&mut self, parent: NodeId, child: NodeId) {
        match self.nodes.get_mut(parent) {
            Some(LayoutNode::Container(container)) => {
                container.children.retain(|&x| x != child);
            }
            _ => panic!("Parent is not a container"),
        }
    }

    pub fn child_index(&self, parent: NodeId, child: NodeId) -> Option<usize> {
        match self.nodes.get(parent) {
            Some(LayoutNode::Container(container)) => {
                container.children.iter().position(|&x| x == child)
            }
            _ => None,
        }
    }

    pub fn remove_child_by_index(&mut self, parent: NodeId, index: usize) {
        match self.nodes.get_mut(parent) {
            Some(LayoutNode::Container(container)) => {
                container.children.remove(index);
            }
            _ => panic!("Parent is not a container"),
        }
    }

    pub fn replace_child(&mut self, parent: NodeId, index: usize, child: NodeId) {
        match self.nodes.get_mut(parent) {
            Some(LayoutNode::Container(container)) => {
                container.children[index] = child;
            }
            _ => panic!("Parent is not a container"),
        }
    }

    pub fn node(&self, node: NodeId) -> Option<&LayoutNode> {
        self.nodes.get(node)
    }

    pub fn is_leaf(&self, node: NodeId) -> bool {
        match self.nodes.get(node) {
            Some(LayoutNode::Leaf(_)) => true,
            _ => false,
        }
    }

    pub fn children(&self, node: NodeId) -> Option<&Vec<NodeId>> {
        match self.nodes.get(node) {
            Some(LayoutNode::Container(container)) => Some(&container.children),
            _ => None,
        }
    }

    pub fn direction(&self, node: NodeId) -> Option<Direction> {
        match self.nodes.get(node) {
            Some(LayoutNode::Container(container)) => Some(container.direction),
            _ => None,
        }
    }

    pub fn child_count(&self, node: NodeId) -> Option<usize> {
        match self.nodes.get(node) {
            Some(LayoutNode::Container(container)) => Some(container.children.len()),
            _ => None,
        }
    }

    pub fn widget(&self, node: NodeId) -> Option<Arc<dyn Widget>> {
        match self.nodes.get(node) {
            Some(LayoutNode::Leaf(leaf)) => Some(leaf.widget.clone()),
            _ => None,
        }
    }
}

impl From<(usize, usize)> for Rect {
    fn from((w, h): (usize, usize)) -> Self {
        Rect {
            x: 0.,
            y: 0.,
            width: w as f32,
            height: h as f32,
        }
    }
}

pub enum Event {
    Input(InputEvent),
    User(String),
}

pub struct Sanguine {
    layout: Layout,
    #[allow(unused)]
    event_queue: VecDeque<Event>,
    term: BufferedTerminal<UnixTerminal>,
    size: Rect,
}

impl Sanguine {
    pub fn new(layout: Layout) -> Result<Self> {
        let caps = Capabilities::new_from_env()?;
        let term = BufferedTerminal::new(UnixTerminal::new(caps)?)?;
        Ok(Sanguine {
            event_queue: VecDeque::new(),
            size: term.dimensions().into(),
            layout,
            term,
        })
    }

    pub fn update_layout(&mut self, f: impl FnOnce(&mut Layout)) {
        f(&mut self.layout);
    }

    pub fn render(&mut self) -> Result<()> {
        self.layout.compute(&self.size);

        self.layout.leaves().iter().for_each(|id| {
            let layout = self.layout.layout(*id).unwrap();
            let leaf = self.layout.widget(*id).unwrap();
            leaf.render(layout.clone(), &mut self.term);
        });

        self.term.flush()?;

        Ok(())
    }
}
