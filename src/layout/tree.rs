use std::{collections::HashMap, sync::Arc};

use slotmap::DefaultKey;

use super::geometry::{Axis, Direction, Rect, SizeHint};
use crate::widget::Widget;

pub type NodeId = DefaultKey;

pub struct Leaf {
    widget: Arc<dyn Widget>,
    parent: Option<NodeId>,
}

impl Leaf {
    pub fn new(widget: Arc<dyn Widget>) -> Self {
        Self {
            widget,
            parent: None,
        }
    }
}

#[derive(Debug)]
pub struct Container {
    direction: Axis,
    size: Option<SizeHint>,
    children: Vec<NodeId>,
    parent: Option<NodeId>,
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
    /// Whether the layout should be recomputed
    dirty: bool,
}

impl Layout {
    /// Initializes a new layout, and creates a root node
    pub fn new() -> Self {
        let mut nodes = slotmap::SlotMap::new();
        let root = nodes.insert(LayoutNode::Container(Container {
            direction: Axis::Vertical,
            size: None,
            children: vec![],
            parent: None,
        }));
        Self {
            nodes,
            root,
            layout: HashMap::from([(root, Rect::default())]),
            // True so that the first call to `compute` will always recompute the layout
            dirty: true,
        }
    }

    /// Returns nodes adjacent to the given node, along with the direction to get to them
    pub fn adjacent(&self, node: NodeId) -> HashMap<NodeId, Direction> {
        let mut map = HashMap::new();
        let parent = self.parent(node).unwrap();
        let direction = self.direction(parent).unwrap();
        let children = self.children(parent).unwrap();
        let index = children.iter().position(|id| *id == node).unwrap();
        if index > 0 {
            let node = children[index - 1];
            if self.is_leaf(node) {
                map.insert(
                    node,
                    match direction {
                        Axis::Vertical => Direction::Up,
                        Axis::Horizontal => Direction::Left,
                    },
                );
            } else {
                let direction = self.direction(node).unwrap();
                let children = self.children(node).unwrap();
                children.iter().for_each(|id| {
                    map.insert(
                        *id,
                        match direction {
                            Axis::Vertical => Direction::Up,
                            Axis::Horizontal => Direction::Left,
                        },
                    );
                });
            }
        }
        if index < children.len() - 1 {
            let node = children[index + 1];
            if self.is_leaf(node) {
                map.insert(
                    node,
                    match direction {
                        Axis::Vertical => Direction::Down,
                        Axis::Horizontal => Direction::Right,
                    },
                );
            } else {
                let direction = self.direction(node).unwrap();
                let children = self.children(node).unwrap();
                children.iter().for_each(|id| {
                    map.insert(
                        *id,
                        match direction {
                            Axis::Vertical => Direction::Right,
                            Axis::Horizontal => Direction::Down,
                        },
                    );
                });
            }
        }

        let parent_parent = self.parent(parent);
        if let Some(grandparent) = parent_parent {
            let direction = self.direction(grandparent).unwrap();
            let children = self.children(grandparent).unwrap();
            children.iter().for_each(|id| {
                if *id == parent {
                    return;
                }
                map.insert(
                    *id,
                    match direction {
                        Axis::Vertical => Direction::Down,
                        Axis::Horizontal => Direction::Left,
                    },
                );
            });
        }

        map
    }

    /// Returns nodes that are adjacent to the given node on the given side.
    pub fn adjacent_on_side(&self, node: NodeId, side: Direction) -> Vec<NodeId> {
        self.adjacent(node)
            .into_iter()
            .filter(|(_, d)| d == &side)
            .map(|(k, _)| k)
            .collect()
    }

    /// Returns x/y value of intersections between node and other nodes on the given side.
    pub fn side_intersections(&self, node: NodeId, side: Direction) -> Vec<f32> {
        let mut intersections = vec![];
        let Some(bounds) = self.layout(node) else {
            return intersections;
        };

        let adjacent = self.adjacent_on_side(node, side);
        adjacent.iter().for_each(|id| {
            let Some(layout) = self.layout(*id) else {
                return;
            };
            // check if top or bottom edge of adjacent node intersects with the side
            match side {
                Direction::Left => {
                    if layout.right() > bounds.left() && layout.left() < bounds.left() {
                        intersections.push(layout.y + layout.height);
                    }
                }
                Direction::Right => {
                    if layout.left() < bounds.right() && layout.right() > bounds.right() {
                        intersections.push(layout.y + layout.height);
                    }
                }
                Direction::Up => {
                    if layout.bottom() > bounds.top() && layout.top() < bounds.top() {
                        intersections.push(layout.x + layout.width);
                    }
                }
                Direction::Down => {
                    if layout.top() < bounds.bottom() && layout.bottom() > bounds.bottom() {
                        intersections.push(layout.x + layout.width);
                    }
                }
            }
        });

        intersections
    }

    /// Clears the layout and **drops** all nodes that are not part of the tree.
    pub fn clean(&mut self) {
        self.dirty = true;
        self.layout.clear();
        self.nodes.retain(|_, v| match v {
            LayoutNode::Container(c) => c.parent.is_some(),
            LayoutNode::Leaf(l) => l.parent.is_some(),
        });
    }

    /// Computes the layout of the tree for the given bounds. This must be called after each change to the tree.
    pub fn compute(&mut self, bounds: &Rect) {
        if self.dirty {
            self.compute_tree(None, bounds);
            self.dirty = false;
        }
    }

    /// Recursively computes the layout of the tree.
    fn compute_tree(&mut self, node: Option<NodeId>, bounds: &Rect) {
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

    /// Computes layout for an individual node
    fn compute_node(&mut self, node: NodeId, bounds: &Rect) {
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
                Axis::Horizontal => bounds.x,
                Axis::Vertical => bounds.y,
            };
            self.compute_sizes(&bounds, &sizes, &axis)
                .iter()
                .for_each(|(k, v)| {
                    let size = match v {
                        SizeHint::Fixed(size) => *size as f32,
                        _ => unreachable!(),
                    };
                    let (width, height) = match &axis {
                        Axis::Horizontal => (size, bounds.height),
                        Axis::Vertical => (bounds.width, size),
                    };
                    let (x, y) = (
                        if axis == Axis::Horizontal {
                            current
                        } else {
                            bounds.x
                        },
                        if axis == Axis::Vertical {
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
        axis: &Axis,
    ) -> Vec<(NodeId, SizeHint)> {
        let mut new_sizes = Vec::new();
        let width = match axis {
            Axis::Horizontal => bounds.width,
            Axis::Vertical => bounds.height,
        };
        let mut remaining = width;

        let fixed = sizes
            .iter()
            .filter_map(|(k, size)| match size {
                SizeHint::Fixed(size) => {
                    new_sizes.push((*k, SizeHint::Fixed(*size)));
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
            let size = f.round() as usize;
            pct_total += size;
            new_sizes.push((**k, SizeHint::Fixed(size)));
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
        let mut diff = remaining - (fill_size * nfill as f32);
        fill.iter()
            .map(|(k, _)| {
                if diff > 0. {
                    diff = (diff - 1.).max(0.);
                    (k, fill_size.floor() - 1.)
                } else if diff < 0. {
                    diff = (diff + 1.).min(0.);
                    (k, fill_size.floor() + 1.)
                } else {
                    (
                        k,
                        match &axis {
                            Axis::Horizontal => fill_size, /* .floor() */
                            Axis::Vertical => fill_size.ceil(),
                        },
                    )
                }
            })
            .for_each(|(k, v)| {
                new_sizes.push((**k, SizeHint::Fixed(v/* fill_size.floor() */ as usize)));
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

    fn leaves_inner(&self, node: NodeId, leaves: &mut Vec<NodeId>) {
        match self.children(node) {
            Some(children) => {
                for child in children {
                    self.leaves_inner(*child, leaves);
                }
            }
            None => leaves.push(node),
        }
    }

    pub fn leaves(&self) -> Vec<NodeId> {
        let mut leaves = vec![];

        self.leaves_inner(self.root, &mut leaves);

        leaves
    }

    pub fn traverse(&self, mut f: impl FnMut(NodeId, &LayoutNode)) {
        self.traverse_recursive(self.root, &mut f);
    }

    pub fn traverse_recursive(&self, node_id: NodeId, f: &mut impl FnMut(NodeId, &LayoutNode)) {
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
        self.dirty = true;
        self.nodes.remove(node);
        self.layout.remove(&node);
    }

    /// Adds a new container node to the layout.
    pub fn add_container(&mut self, direction: Axis, size: Option<SizeHint>) -> NodeId {
        let container = Container {
            children: vec![],
            direction,
            size,
            parent: None,
        };
        let node = LayoutNode::Container(container);
        let id = self.nodes.insert(node);
        self.layout.insert(id, Rect::default());
        id
    }

    pub fn set_size(&mut self, node: NodeId, size: SizeHint) {
        self.dirty = true;
        match self.nodes.get_mut(node) {
            Some(LayoutNode::Container(container)) => {
                container.size = Some(size);
            }
            _ => {}
        }
    }

    pub fn set_direction(&mut self, node: NodeId, axis: Axis) {
        self.dirty = true;
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
        direction: Axis,
        size: Option<SizeHint>,
        children: impl Into<Vec<NodeId>>,
    ) -> NodeId {
        self.dirty = true;
        let c = children.into();
        let container = Container {
            children: c.clone(),
            direction,
            size,
            parent: None,
        };
        let node = LayoutNode::Container(container);
        let id = self.nodes.insert(node);
        c.iter().for_each(|v| {
            // if let Some(LayoutNode::Container(container)) = self.nodes.get_mut(*v) {
            //     container.parent = Some(container.parent.unwrap_or(self.root));
            // }
            match self.nodes.get_mut(*v) {
                Some(LayoutNode::Container(container)) => {
                    container.parent = Some(id);
                }
                Some(LayoutNode::Leaf(leaf)) => {
                    leaf.parent = Some(id);
                }
                _ => {}
            }
        });
        self.layout.insert(id, Rect::default());
        id
    }

    /// Adds a new leaf node to the layout.
    pub fn add_leaf(&mut self, leaf: Leaf) -> NodeId {
        self.dirty = true;
        let node = LayoutNode::Leaf(leaf);
        let id = self.nodes.insert(node);
        self.layout.insert(id, Rect::default());
        id
    }

    /// Adds a new leaf node to the given container.
    pub fn add_child(&mut self, parent: NodeId, child: NodeId) {
        self.dirty = true;
        match self.nodes.get_mut(parent) {
            Some(LayoutNode::Container(container)) => {
                container.children.push(child);
            }
            _ => panic!("Parent is not a container"),
        }
        self.set_parent(child, Some(parent));
    }

    /// Removes a child from the given container. This does not drop the node.
    pub fn remove_child(&mut self, parent: NodeId, child: NodeId) {
        self.dirty = true;
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
        self.dirty = true;
        match self.nodes.get_mut(parent) {
            Some(LayoutNode::Container(container)) => {
                container.children.remove(index);
            }
            _ => panic!("Parent is not a container"),
        }
    }

    pub fn replace_child(&mut self, parent: NodeId, child: NodeId, new: NodeId) {
        self.dirty = true;
        let old;
        match self.nodes.get_mut(parent) {
            Some(LayoutNode::Container(container)) => {
                let index = container.children.iter().position(|&x| x == child).unwrap();
                old = Some(container.children[index]);

                container.children[index] = new;
            }
            _ => panic!("Parent is not a container"),
        }
        if let Some(old) = old {
            self.set_parent(old, None);
        }
        self.set_parent(new, Some(parent));
    }

    pub fn node(&self, node: NodeId) -> Option<&LayoutNode> {
        self.nodes.get(node)
    }

    fn set_parent(&mut self, node: NodeId, parent: Option<NodeId>) {
        self.dirty = true;
        match self.nodes.get_mut(node) {
            Some(LayoutNode::Container(container)) => {
                container.parent = parent;
            }
            Some(LayoutNode::Leaf(leaf)) => {
                leaf.parent = parent;
            }
            _ => {}
        }
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

    pub fn direction(&self, node: NodeId) -> Option<Axis> {
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

    pub fn parent(&self, node: NodeId) -> Option<NodeId> {
        match self.nodes.get(node) {
            Some(LayoutNode::Container(container)) => container.parent,
            Some(LayoutNode::Leaf(leaf)) => leaf.parent,
            _ => None,
        }
    }

    pub fn insert_child_at(&mut self, parent: NodeId, child: NodeId, index: usize) {
        self.dirty = true;
        match self.nodes.get_mut(parent) {
            Some(LayoutNode::Container(container)) => {
                container.children.insert(index, child);
            }
            _ => panic!("Parent is not a container"),
        }
        self.set_parent(child, Some(parent));
    }

    pub fn split(&mut self, node: NodeId, direction: Axis, leaf: Leaf) -> Option<NodeId> {
        self.dirty = true;
        if self.is_leaf(node) {
            let new = self.add_container(direction, None);
            let new_leaf = self.add_leaf(leaf);
            let parent = self.parent(node).unwrap();
            let index = self.child_index(parent, node).unwrap();
            self.remove_child_by_index(parent, index);
            self.add_child(new, node);
            self.add_child(new, new_leaf);
            self.insert_child_at(parent, new, index);
            return Some(new_leaf);
        } else {
            let self_dir = self.direction(node).unwrap();
            let new_leaf = self.add_leaf(leaf);
            if self_dir == direction {
                self.add_child(node, new_leaf)
            } else {
                let new = self.add_container(direction, None);
                let parent = self.parent(node).unwrap();
                let index = self.child_index(parent, node).unwrap();
                self.remove_child_by_index(parent, index);
                self.add_child(new, node);
                self.add_child(new, new_leaf);
                self.insert_child_at(parent, new, index);
                return Some(node);
            }
            None
        }
    }
}
