use std::sync::{mpsc::Sender, Arc, RwLock};

use crate::{
    event::{Event, UserEvent},
    layout::*,
    surface::Surface,
};

/// The data passed to [`Widget::render`]
pub struct RenderCtx<'render, U, S> {
    pub focused: bool,
    pub layout: &'render Layout<U, S>,
    pub state: &'render S,
}

/// The data passed to [`Widget::update`]
pub struct UpdateCtx<'update, U, S> {
    pub owner: NodeId,
    pub bounds: Rect,
    pub layout: &'update mut Layout<U, S>,
    pub tx: Arc<Sender<UserEvent<U>>>,
    pub state: &'update mut S,
}

impl<'render, U, S> RenderCtx<'render, U, S> {
    pub fn new(focused: bool, layout: &'render Layout<U, S>, state: &'render S) -> Self {
        Self {
            focused,
            layout,
            state,
        }
    }
}

impl<'update, U, S> UpdateCtx<'update, U, S> {
    pub fn new(
        owner: NodeId,
        bounds: Rect,
        layout: &'update mut Layout<U, S>,
        tx: Arc<Sender<UserEvent<U>>>,
        state: &'update mut S,
    ) -> Self {
        Self {
            owner,
            bounds,
            layout,
            tx,
            state,
        }
    }

    pub fn with_rect<'u>(&'u mut self, rect: Rect) -> UpdateCtx<'u, U, S> {
        UpdateCtx {
            owner: self.owner,
            bounds: rect,
            layout: self.layout,
            tx: self.tx.clone(),
            state: self.state,
        }
    }
}

/// The core widget trait that all widgets must implement.
/// This trait provides the methods that the layout engine uses to interact with widgets.
///
/// Implementors of [`Widget`] can be displayed inside of a window, or
/// nested in other widgets.
///
/// Widgets can be shared behind an [`Arc<RwLock<dyn Widget>>`] to show the same widget in multiple
/// windows.
pub trait Widget<U, S> {
    /// This method is called every render loop, and is responsible for rendering the widget onto
    /// the provided surface.
    fn render(
        &self,
        cx: &RenderCtx<U, S>,
        surface: &mut Surface,
    ) -> Option<Vec<(Rect, Arc<RwLock<dyn Widget<U, S>>>)>>;

    #[allow(unused_variables)]
    /// This method is called when an input event is received that targets this widget.
    /// It allows the widget to update its internal state in response to an event.
    fn update(&mut self, cx: &mut UpdateCtx<U, S>, event: Event<U>) -> crate::error::Result<()> {
        Ok(())
    }

    /// This method is called when the widget is focused, to determine where (or if) to display the
    /// cursor.
    fn cursor(&self) -> Option<(Option<usize>, usize, usize)> {
        None
    }

    /// This method provides a hint to the layout engine about how much
    /// space the widget should take up.
    fn constraint(&self) -> Constraint {
        Constraint::Fill
    }
}
