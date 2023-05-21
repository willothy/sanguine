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

    pub fn left(&self) -> f32 {
        self.x
    }

    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    pub fn top(&self) -> f32 {
        self.y
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }

    pub fn from_size(dims: (usize, usize)) -> Rect {
        Rect {
            width: dims.0 as f32,
            height: dims.1 as f32,
            ..Default::default()
        }
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
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone)]
pub enum Constraint {
    Fixed(usize),
    Percentage(f32),
    Fill,
}

impl Constraint {
    pub fn fill() -> Constraint {
        Constraint::Fill
    }
}
