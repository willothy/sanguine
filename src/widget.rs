use std::{
    ptr::NonNull,
    sync::{mpsc::Sender, Arc},
};

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
    widgets: &'render WidgetStore<U, S>,
}

/// The data passed to [`Widget::update`]
pub struct UpdateCtx<'update, U, S> {
    pub owner: NodeId,
    pub bounds: Rect,
    pub layout: &'update mut Layout<U, S>,
    pub tx: Arc<Sender<UserEvent<U>>>,
    pub state: &'update mut S,
    widgets: NonNull<WidgetStore<U, S>>,
}

impl<'render, U, S> RenderCtx<'render, U, S> {
    pub fn new(
        focused: bool,
        layout: &'render Layout<U, S>,
        widgets: &'render WidgetStore<U, S>,
        state: &'render S,
    ) -> Self {
        Self {
            focused,
            layout,
            widgets,
            state,
        }
    }

    pub fn get_widget(&self, id: WidgetId) -> Option<&'render dyn Widget<U, S>> {
        self.widgets.get(id)
    }

    pub fn resolve<T: Widget<U, S> + 'static>(&self, id: WidgetId) -> Option<&T> {
        self.widgets.resolve(id)
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
            widgets: unsafe { NonNull::new_unchecked(widgets) },
            layout,
            tx,
            state,
        }
    }

    /// Get a reference to a widget by its ID, as an immutable `dyn Widget` trait object.
    pub fn get_widget(&self, id: WidgetId) -> Option<&'update dyn Widget<U, S>> {
        unsafe { self.widgets.as_ref().get(id) }
    }

    /// Get a reference to a widget by its ID, as a mutable `dyn Widget` trait object.
    pub fn get_widget_mut(&mut self, id: WidgetId) -> Option<&'update mut dyn Widget<U, S>> {
        unsafe { self.widgets.as_mut().get_mut(id) }
    }

    /// Remove a widget from the widget store.
    ///
    /// Note that any references to the widget following this call are invalid.
    pub fn remove_widget(&mut self, id: WidgetId) {
        unsafe { self.widgets.as_mut().remove(id) };
    }

    /// Get an immutable reference to a widget by its ID, and attempt to downcast it to a concrete type.
    pub fn resolve<W: Widget<U, S> + 'static>(&self, id: WidgetId) -> Option<&'update W> {
        unsafe { self.widgets.as_ref().resolve::<W>(id) }
    }

    /// Get a mutable reference to a widget by its ID, and attempt to downcast it to a concrete
    /// type.
    pub fn resolve_mut<W: Widget<U, S> + 'static>(
        &mut self,
        id: WidgetId,
    ) -> Option<&'update mut W> {
        unsafe { self.widgets.as_mut().resolve_mut::<W>(id) }
    }

    /// Register a new widget with the widget store.
    pub fn register_widget(&mut self, widget: impl Widget<U, S> + 'static) -> WidgetId {
        unsafe { self.widgets.as_mut().register(widget) }
    }

    /// Create a new [`UpdateCtx`] with different bounds, intended for rendering inner widgets.
    pub fn with_rect<'inner>(&'inner mut self, rect: Rect) -> UpdateCtx<'inner, U, S> {
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

    /// Convert the widget into an immutable [`std::any::Any`] trait object, for use when resolving
    /// widgets to concrete types. This should usually return `self`. They are required to be
    /// implemented by each widget because a ref'd concrete type (&Self) implementing widget can be cast to &dyn Any,
    /// but trait objects such as &dyn Widget cannot.
    ///
    /// ```rust
    /// fn as_any(&self) -> &dyn std::any::Any {
    ///		self
    /// }
    /// ```
    ///
    fn as_any(&self) -> &dyn std::any::Any;

    /// Convert the widget into a mutable [`std::any::Any`] trait object, for use when resolving
    /// widgets to concrete types. This should usually return `self`. They are required to be
    /// implemented by each widget because a ref'd concrete type (&Self) implementing widget can be cast to &dyn Any,
    /// but trait objects such as &dyn Widget cannot.
    ///
    /// ```rust
    /// fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
    ///		self
    /// }
    /// ```
    ///
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}
