pub struct Size {
    pub width: usize,
    pub height: usize,
}

impl From<taffy::geometry::Size<f32>> for Size {
    fn from(size: taffy::geometry::Size<f32>) -> Self {
        Self {
            width: size.width.round() as usize,
            height: size.height.round() as usize,
        }
    }
}

impl Into<taffy::geometry::Size<f32>> for Size {
    fn into(self) -> taffy::geometry::Size<f32> {
        taffy::geometry::Size {
            width: self.width as f32,
            height: self.height as f32,
        }
    }
}

impl Into<taffy::geometry::Size<taffy::style::Dimension>> for Size {
    fn into(self) -> taffy::geometry::Size<taffy::style::Dimension> {
        use taffy::style::Dimension;
        taffy::geometry::Size {
            width: Dimension::Points(self.width as f32),
            height: Dimension::Points(self.height as f32),
        }
    }
}

pub struct Point {
    pub x: usize,
    pub y: usize,
}

impl Point {
    pub fn as_margin(&self) -> taffy::prelude::Rect<taffy::style::LengthPercentageAuto> {
        use taffy::style::LengthPercentageAuto;
        taffy::prelude::Rect {
            left: LengthPercentageAuto::Points(self.x as f32),
            right: LengthPercentageAuto::Auto,
            top: LengthPercentageAuto::Points(self.y as f32),
            bottom: LengthPercentageAuto::Auto,
        }
    }
}

impl From<taffy::geometry::Point<f32>> for Point {
    fn from(point: taffy::geometry::Point<f32>) -> Self {
        Self {
            x: point.x.round() as usize,
            y: point.y.round() as usize,
        }
    }
}

pub struct Layout {
    pub size: Size,
    pub location: Point,
}

impl From<taffy::layout::Layout> for Layout {
    fn from(layout: taffy::layout::Layout) -> Self {
        Self {
            size: layout.size.into(),
            location: layout.location.into(),
        }
    }
}
