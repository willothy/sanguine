use std::sync::{mpsc::Sender, Arc};

use termwiz::surface::Surface;

use crate::{layout::*, Event};

/// The core widget trait that all widgets must implement.
/// This trait provides the methods that the layout engine uses to interact with widgets.
///
/// Implementors of `Widget` can be displayed inside of a window (a layout `Leaf`), or
/// nested in other widgets.
///
/// Widgets can be shared behind an `Arc<RwLock<dyn Widget>>` to show the same widget in multiple
/// windows.
pub trait Widget {
    /// This method is called every render loop, and is responsible for rendering the widget onto
    /// the provided surface.
    fn render(&self, layout: &Layout, surface: &mut Surface, focused: bool);

    #[allow(unused_variables)]
    /// This method is called when an input event is received that targets this widget.
    /// It allows the widget to update its internal state in response to an event.
    fn update(&mut self, event: Event, exit_tx: Arc<Sender<()>>) {}

    /// This method provides a hint to the layout engine about how much
    /// space the widget should take up.
    fn size_hint(&self) -> SizeHint {
        SizeHint::Fill
    }
}
