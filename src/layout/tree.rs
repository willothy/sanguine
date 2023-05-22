use std::sync::{Arc, RwLock};

use slotmap::{new_key_type, SecondaryMap, SlotMap};

use super::{
    floating::{FloatStack, Floating},
    geometry::{Axis, Constraint, Direction, Rect},
};
use crate::widget::Widget;

new_key_type! {
    pub struct NodeId;
}

pub struct Leaf<U, S> {
    widget: Arc<RwLock<dyn Widget<U, S>>>,
    parent: Option<NodeId>,
}

impl<U, S> Leaf<U, S> {
    pub fn new(widget: impl Widget<U, S> + 'static) -> Self {
        Self {
            widget: Arc::new(RwLock::new(widget)),
            parent: None,
        }
    }

    pub fn from_widget(widget: Arc<RwLock<dyn Widget<U, S>>>) -> Self {
        Self {
            widget,
            parent: None,
        }
    }
}

impl<U, S> Clone for Leaf<U, S> {
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.clone(),
            // When a leaf is cloned, the intention is to clone its widget. Parent can be set
            // separately if needed.
            parent: None,
        }
    }
}

#[derive(Debug)]
pub struct Container {
    direction: Axis,
    size: Option<Constraint>,
    children: Vec<NodeId>,
    parent: Option<NodeId>,
}

pub enum LayoutNode<U, S> {
    Container(Container),
    Leaf(Leaf<U, S>),
    Floating(Floating<U, S>),
}

impl<U, S> LayoutNode<U, S> {
    pub fn is_leaf(&self) -> bool {
        matches!(self, Self::Leaf(_))
    }

    pub fn is_container(&self) -> bool {
        matches!(self, Self::Container(_))
    }

    pub fn is_floating(&self) -> bool {
        matches!(self, Self::Floating(_))
    }

    pub fn leaf(&self) -> Option<&Leaf<U, S>> {
        match self {
            Self::Leaf(leaf) => Some(leaf),
            _ => None,
        }
    }

    pub fn container(&self) -> Option<&Container> {
        match self {
            Self::Container(container) => Some(container),
            _ => None,
        }
    }

    pub fn floating(&self) -> Option<&Floating<U, S>> {
        match self {
            Self::Floating(floating) => Some(floating),
            _ => None,
        }
    }
}

pub struct Layout<U = (), S = ()> {
    /// The arena containing all nodes, keyed by unique id.
    nodes: SlotMap<NodeId, LayoutNode<U, S>>,
    /// Render results. Will be stale or zeroed if [`Layout::compute`] isn't called after each
    /// change.
    layout: SecondaryMap<NodeId, Rect>,
    /// The root node of the layout.
    root: NodeId,
    /// Floating windows attached to the layout
    floating: FloatStack<U, S>,
    /// Whether the layout should be recomputed
    dirty: bool,
}

impl<U, S> Default for Layout<U, S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<U, S> Layout<U, S> {
    /// Initializes a new layout, and creates a root node
    pub fn new() -> Self {
        let mut nodes = SlotMap::with_key();
        let mut layout = SecondaryMap::new();
        let root = nodes.insert(LayoutNode::Container(Container {
            direction: Axis::Vertical,
            size: None,
            children: vec![],
            parent: None,
        }));
        layout.insert(root, Rect::default());
        Self {
            nodes,
            layout,
            root,
            floating: FloatStack::new(),
            // True so that the first call to `compute` will always recompute the layout
            dirty: true,
        }
    }

    pub fn node_at_pos(&self, pos: (u16, u16)) -> Option<NodeId> {
        self.floating
            .iter()
            .find_map(|id| {
                self.layout(*id)
                    .map(|rect| (id, rect))
                    .and_then(|(id, rect)| {
                        if rect.contains(pos.0 as f32, pos.1 as f32) {
                            Some(*id)
                        } else {
                            None
                        }
                    })
            })
            .or_else(|| {
                self.leaves().into_iter().find(|v| {
                    let Some(rect) = self.layout(*v) else {
                        return false;
                    };

                    rect.contains(pos.0 as f32, pos.1 as f32)
                })
            })
    }

    /// Returns nodes adjacent to the given node, along with the direction to get to them
    pub fn adjacent(&self, node: NodeId) -> Vec<(NodeId, Direction)> {
        let mut neighbors = Vec::new();
        if self.is_floating(node) {
            return neighbors;
        }
        let parent = self.parent(node).unwrap();
        let direction = self.direction(parent).unwrap();
        let children = self.children(parent).unwrap();
        let index = children.iter().position(|id| *id == node).unwrap();
        if index > 0 {
            let node = children[index - 1];
            if self.is_leaf(node) {
                neighbors.push((
                    node,
                    match direction {
                        Axis::Vertical => Direction::Up,
                        Axis::Horizontal => Direction::Left,
                    },
                ));
            } else {
                let direction = self.direction(node).unwrap();
                let children = self.children(node).unwrap();
                children.iter().for_each(|id| {
                    neighbors.push((
                        *id,
                        match direction {
                            Axis::Vertical => Direction::Up,
                            Axis::Horizontal => Direction::Left,
                        },
                    ));
                });
            }
        }
        if index < children.len() - 1 {
            let node = children[index + 1];
            if self.is_leaf(node) {
                neighbors.push((
                    node,
                    match direction {
                        Axis::Vertical => Direction::Down,
                        Axis::Horizontal => Direction::Right,
                    },
                ));
            } else {
                let direction = self.direction(node).unwrap();
                let children = self.children(node).unwrap();
                children.iter().for_each(|id| {
                    neighbors.push((
                        *id,
                        match direction {
                            Axis::Vertical => Direction::Right,
                            Axis::Horizontal => Direction::Down,
                        },
                    ));
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
                neighbors.push((
                    *id,
                    match direction {
                        Axis::Vertical => Direction::Down,
                        Axis::Horizontal => Direction::Left,
                    },
                ));
            });
        }

        neighbors
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
        if self.is_floating(node) {
            return intersections;
        }
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
        self.nodes.clear();
    }

    /// Computes the layout of the tree for the given bounds. This must be called after each change to the tree.
    pub fn compute(&mut self, bounds: &Rect) {
        if self.dirty {
            self.compute_tree(None, bounds);
            self.dirty = false;
        }
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Recursively computes the layout of the tree.
    fn compute_tree(&mut self, node: Option<NodeId>, bounds: &Rect) {
        let node = node.unwrap_or(self.root());
        self.compute_node(node, bounds);
        if self.is_leaf(node) {
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
            self.compute_sizes(bounds, &sizes, &axis)
                .iter()
                .for_each(|(k, v)| {
                    let size = match v {
                        Constraint::Fixed(size) => *size as f32,
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

    /// Actual size computation for layout
    fn compute_sizes(
        &mut self,
        bounds: &Rect,
        sizes: &[(NodeId, Constraint)],
        axis: &Axis,
    ) -> Vec<(NodeId, Constraint)> {
        let mut new_sizes = Vec::new();
        let width = match axis {
            Axis::Horizontal => bounds.width,
            Axis::Vertical => bounds.height,
        };
        let mut remaining = width;

        let fixed = sizes
            .iter()
            .filter_map(|(k, size)| match size {
                Constraint::Fixed(size) => {
                    new_sizes.push((*k, Constraint::Fixed(*size)));
                    Some(size)
                }
                _ => None,
            })
            .sum::<usize>();

        remaining -= fixed as f32;

        let mut percents = sizes
            .iter()
            .filter_map(|(k, size)| match size {
                Constraint::Percentage(percent) => Some((k, *percent)),
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
            *f *= remaining;
            let size = f.round() as usize;
            pct_total += size;
            new_sizes.push((**k, Constraint::Fixed(size)));
        });
        remaining -= pct_total as f32;

        let fill = sizes
            .iter()
            .enumerate()
            .filter_map(|(i, (k, size))| match size {
                Constraint::Fill => Some((k, i)),
                _ => None,
            })
            .collect::<Vec<_>>();

        let nfill = fill.len();

        let fill_size = (remaining.floor() as usize / nfill) as f32;
        let mut diff = remaining.floor() as usize % nfill;
        fill.iter()
            .map(|(k, _)| {
                if diff > 0 {
                    diff -= 1;
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
                new_sizes.push((**k, Constraint::Fixed(v/* fill_size.floor() */ as usize)));
            });

        new_sizes
    }

    /// Get the size hint of a given node
    pub fn size(&self, node: NodeId) -> Constraint {
        match self.nodes.get(node) {
            Some(LayoutNode::Container(container)) => {
                container.size.clone().unwrap_or(Constraint::Fill)
            }
            Some(LayoutNode::Leaf(leaf)) => leaf.widget.read().unwrap().constraint(),
            Some(LayoutNode::Floating(_)) => Constraint::Fill,
            None => Constraint::Fill,
        }
    }

    /// Get the id of the root node
    pub fn root(&self) -> NodeId {
        self.root
    }

    /// Retrieve the computed layout for a given node
    pub fn layout(&self, node: NodeId) -> Option<&Rect> {
        self.layout.get(node)
    }

    /// Helper for gathering leaves recursively
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

    /// Get the leaves of the layout tree
    pub fn leaves(&self) -> Vec<NodeId> {
        let mut leaves = vec![];

        self.leaves_inner(self.root, &mut leaves);

        leaves
    }

    /// Get the floats of the layout tree
    pub fn floats(&self) -> Vec<NodeId> {
        self.floating.iter().copied().collect()
    }

    /// Traverse the layout tree
    pub fn traverse(&self, mut f: impl FnMut(NodeId, &LayoutNode<U, S>)) {
        self.traverse_recursive(self.root, &mut f);
    }

    /// Recursive traversal helper
    fn traverse_recursive(&self, node_id: NodeId, f: &mut impl FnMut(NodeId, &LayoutNode<U, S>)) {
        let node = self.nodes.get(node_id).unwrap();
        f(node_id, node);
        match node {
            LayoutNode::Container(container) => {
                for child in &container.children {
                    self.traverse_recursive(*child, f);
                }
            }
            LayoutNode::Leaf(_) => {}
            LayoutNode::Floating(_) => {}
        }
    }

    /// Recursively print the layout.
    ///
    /// Intended for debug use only
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
            LayoutNode::Floating(_) => println!("Floating: {:?}", self.layout(node_id)),
        }
    }

    /// Drops a node from the layout. This will not drop children of the node.
    /// Use of the provided NodeId after calling this is invalid.
    pub fn remove_node(&mut self, node: NodeId) {
        self.dirty = true;
        self.nodes.remove(node);
        self.layout.remove(node);
    }

    /// Sets the size hint for a container
    pub fn set_size(&mut self, node: NodeId, size: Constraint) {
        self.dirty = true;
        if let Some(LayoutNode::Container(container)) = self.nodes.get_mut(node) {
            container.size = Some(size);
        }
    }

    /// Sets the direction of a container node.
    pub fn set_direction(&mut self, node: NodeId, axis: Axis) {
        self.dirty = true;
        if let Some(LayoutNode::Container(container)) = self.nodes.get_mut(node) {
            container.direction = axis;
        }
    }

    /// Adds a new (empty) container node to the layout.
    pub fn add_container(&mut self, direction: Axis, size: Option<Constraint>) -> NodeId {
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

    /// Adds a new container node to the layout with the given children.
    pub fn add_with_children(
        &mut self,
        direction: Axis,
        size: Option<Constraint>,
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
        c.iter().for_each(|v| match self.nodes.get_mut(*v) {
            Some(LayoutNode::Container(container)) => {
                container.parent = Some(id);
            }
            Some(LayoutNode::Leaf(leaf)) => {
                leaf.parent = Some(id);
            }
            _ => {}
        });
        self.layout.insert(id, Rect::default());
        id
    }

    /// Adds a new leaf node to the layout.
    pub fn add_leaf(&mut self, widget: impl Widget<U, S> + 'static) -> NodeId {
        self.dirty = true;
        let node = LayoutNode::Leaf(Leaf::new(widget));
        let id = self.nodes.insert(node);
        self.layout.insert(id, Rect::default());
        id
    }

    /// Adds a new leaf from Arc'd widget
    pub fn add_leaf_raw(&mut self, widget: Arc<RwLock<dyn Widget<U, S>>>) -> NodeId {
        self.dirty = true;
        let node = LayoutNode::Leaf(Leaf::from_widget(widget));
        let id = self.nodes.insert(node);
        self.layout.insert(id, Rect::default());
        id
    }

    pub fn add_floating(&mut self, widget: impl Widget<U, S> + 'static, rect: Rect) -> NodeId {
        self.dirty = true;
        let node = LayoutNode::Floating(Floating::new(widget, rect.clone()));
        let id = self.nodes.insert(node);
        self.layout.insert(id, rect);
        self.floating.push(id, &self.nodes);
        id
    }

    pub fn make_leaf(&mut self, node: NodeId) {
        self.dirty = true;
        if !self.is_floating(node) {
            return;
        }
        let widget = self.widget(node).unwrap();
        if let Some(floating) = self.nodes.get_mut(node) {
            let leaf = Leaf::from_widget(widget);
            let new = LayoutNode::Leaf(leaf);
            self.floating.remove(node);
            *floating = new;
        }
    }

    /// Directly adds a leaf node to the layout.
    pub fn clone_leaf(&mut self, leaf: NodeId) -> NodeId {
        self.dirty = true;
        let widget = self
            .nodes
            .get(leaf)
            .map(|l| match l {
                LayoutNode::Leaf(leaf) => leaf.clone(),
                _ => panic!("Node is not a leaf"),
            })
            .unwrap();
        let node = LayoutNode::Leaf(widget);
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

    /// Replace the child of a container with another.
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

    /// Sets the parent of the given node.
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

    /// Checks if the given node is a leaf node.
    pub fn is_leaf(&self, node: NodeId) -> bool {
        matches!(self.nodes.get(node), Some(LayoutNode::Leaf(_)))
    }

    /// Checks if the given node is a container node.
    pub fn is_container(&self, node: NodeId) -> bool {
        matches!(self.nodes.get(node), Some(LayoutNode::Container(_)))
    }

    /// If the given node is a container, returns a reference to its children.
    pub fn children(&self, node: NodeId) -> Option<&Vec<NodeId>> {
        match self.nodes.get(node) {
            Some(LayoutNode::Container(container)) => Some(&container.children),
            _ => None,
        }
    }

    /// If the given node is a container, returns its layout direction.
    pub fn direction(&self, node: NodeId) -> Option<Axis> {
        match self.nodes.get(node) {
            Some(LayoutNode::Container(container)) => Some(container.direction),
            _ => None,
        }
    }

    /// If the given node is a container, returns the number of children it has.
    pub fn child_count(&self, node: NodeId) -> Option<usize> {
        match self.nodes.get(node) {
            Some(LayoutNode::Container(container)) => Some(container.children.len()),
            _ => None,
        }
    }

    /// If the given node is a leaf, returns a Arc pointing to its widget.
    pub fn widget(&self, node: NodeId) -> Option<Arc<RwLock<dyn Widget<U, S>>>> {
        match self.nodes.get(node) {
            Some(LayoutNode::Leaf(leaf)) => Some(leaf.widget.clone()),
            Some(LayoutNode::Floating(float)) => Some(float.widget()),
            _ => None,
        }
    }

    /// Returns the parent of the given node, if any.
    pub fn parent(&self, node: NodeId) -> Option<NodeId> {
        match self.nodes.get(node) {
            Some(LayoutNode::Container(container)) => container.parent,
            Some(LayoutNode::Leaf(leaf)) => leaf.parent,
            _ => None,
        }
    }

    /// Checks if the node is the root node
    pub fn is_root(&self, node: NodeId) -> bool {
        node == self.root()
    }

    /// Inserts a new child node at the given index.
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

    /// Adds a new container node to the layout by splitting the given node.
    ///
    /// If the node is a container and has the same direction as the requested split, a child
    /// container will be added containing the node and the new node.
    ///
    /// If the node is a container and has the opposite direction, a new container will be added to
    /// its parent, owning the node and the newly created node.
    ///
    /// If the node is a leaf, it will be replaced by a container, which will contain it and the
    /// newly created node.
    pub fn split(
        &mut self,
        node: NodeId,
        direction: Axis,
        widget: impl Widget<U, S> + 'static,
    ) -> NodeId {
        self.dirty = true;
        if self.is_leaf(node) {
            let new = self.add_container(direction, None);
            let new_leaf = self.add_leaf(widget);
            let parent = self.parent(node).unwrap();
            let index = self.child_index(parent, node).unwrap();
            self.remove_child_by_index(parent, index);
            self.add_child(new, node);
            self.add_child(new, new_leaf);
            self.insert_child_at(parent, new, index);
            new_leaf
        } else {
            let self_dir = self.direction(node).unwrap();
            let new_leaf = self.add_leaf(widget);
            if self_dir == direction {
                self.add_child(node, new_leaf);
                new_leaf
            } else {
                let new = self.add_container(direction, None);
                let parent = self.parent(node).unwrap();
                let index = self.child_index(parent, node).unwrap();
                self.remove_child_by_index(parent, index);
                self.add_child(new, node);
                self.add_child(new, new_leaf);
                self.insert_child_at(parent, new, index);
                node
            }
        }
    }

    fn is_floating(&self, node: NodeId) -> bool {
        matches!(self.nodes.get(node), Some(LayoutNode::Floating(_)))
    }
}

#[cfg(test)]
pub mod tests {
    use crate::{
        layout::{Axis, Constraint},
        widgets::{Border, TextBox},
    };

    use super::Layout;

    #[test]
    fn adjacent() {
        // Create the layout struct
        let mut layout = Layout::<(), ()>::new();

        // Create a TextBox widget, wrapped by a Border widget
        let editor_1 = Border::new("textbox 1".to_owned(), TextBox::new());

        // Add the first editor to the layout
        let left = layout.add_leaf(editor_1);

        // Add the menu widget
        let top_right = layout.clone_leaf(left);

        // Clone the first editor to add it to the layout again
        // This widget will be *shared* between the two windows, meaning that changes to the underlying
        // buffer will be shown in both windows and focusing on either window will allow you to edit
        // the same buffer.
        let bot_right = layout.clone_leaf(left);

        // Add the second editor to the layout
        // let bot_right = layout.add_leaf(editor_2);

        // Create a container to hold the two right hand side editors
        let right = layout.add_with_children(
            // The container will be a vertical layout
            Axis::Vertical,
            // The container will take up all available space
            Some(Constraint::fill()),
            // The container will contain the cloned first editor, and the second editor
            [top_right, bot_right],
        );

        // Get the root node of the layout
        let root = layout.root();
        // Ensure that the root container is laid out horizontally
        layout.set_direction(root, Axis::Horizontal);

        // Add the left window (leaf) and the right container to the root
        layout.add_child(root, left);
        layout.add_child(root, right);

        let _adjacent = layout.adjacent(left);
    }
}
