use std::sync::{mpsc::Sender, Arc};

use crate::{
    event::{Event, UserEvent},
    layout::*,
    surface::Surface,
    WidgetStore,
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
    widgets: *mut WidgetStore<U, S>,
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
        widgets: *mut WidgetStore<U, S>,
        layout: &'update mut Layout<U, S>,
        tx: Arc<Sender<UserEvent<U>>>,
        state: &'update mut S,
    ) -> Self {
        Self {
            owner,
            bounds,
            widgets,
            layout,
            tx,
            state,
        }
    }

    pub fn get_widget<'a>(&self, id: WidgetId) -> Option<&'a dyn Widget<U, S>> {
        unsafe { (*self.widgets).get(id) }
    }

    pub fn get_widget_mut<'a>(&mut self, id: WidgetId) -> Option<&'a mut dyn Widget<U, S>> {
        unsafe { (*self.widgets).get_mut(id) }
    }

    pub fn resolve<'a, T: Widget<U, S> + 'static>(&self, id: WidgetId) -> Option<&T> {
        unsafe { (*self.widgets).resolve(id) }
    }

    pub fn resolve_mut<'a, T: Widget<U, S> + 'static>(&mut self, id: WidgetId) -> Option<&mut T> {
        unsafe { (*self.widgets).resolve_mut(id) }
    }

    pub fn register_widget(&mut self, widget: impl Widget<U, S> + 'static) -> WidgetId {
        unsafe { (*self.widgets).register(widget) }
    }

    pub fn with_rect<'u>(&'u mut self, rect: Rect) -> UpdateCtx<'u, U, S> {
        UpdateCtx {
            owner: self.owner,
            bounds: rect,
            widgets: self.widgets,
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
#[allow(unused_variables)]
pub trait Widget<U, S> {
    /// This method is called every render loop, and is responsible for rendering the widget onto
    /// the provided surface.
    fn render(&self, cx: &RenderCtx<U, S>, surface: &mut Surface) -> Option<Vec<(Rect, WidgetId)>>;

    /// This method is called when an input event is received that targets this widget.
    /// It allows the widget to update its internal state in response to an event.
    fn update(&mut self, cx: &mut UpdateCtx<U, S>, event: Event<U>) -> crate::error::Result<()> {
        Ok(())
    }

    /// This method is called when the widget is focused, to determine where (or if) to display the
    /// cursor.
    fn cursor(&self, widgets: &WidgetStore<U, S>) -> Option<(Option<usize>, usize, usize)> {
        None
    }

    /// This method provides a hint to the layout engine about how much
    /// space the widget should take up.
    fn constraint(&self, widgets: &WidgetStore<U, S>) -> Constraint {
        Constraint::Fill
    }

    fn as_any(&self) -> &dyn std::any::Any;

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}
