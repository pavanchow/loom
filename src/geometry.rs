//! Basic geometry primitives used across the layout engine.

/// A width and height pair, measured in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    pub w: f64,
    pub h: f64,
}

impl Size {
    pub const ZERO: Size = Size { w: 0.0, h: 0.0 };

    #[must_use]
    pub fn new(w: f64, h: f64) -> Size {
        Size { w, h }
    }
}

/// A point in the coordinate space, with the origin at the top left.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    #[must_use]
    pub fn new(x: f64, y: f64) -> Point {
        Point { x, y }
    }
}

/// An axis aligned rectangle. This is the computed border box of a node,
/// meaning it includes the node border and padding but excludes its margin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub const ZERO: Rect = Rect {
        x: 0.0,
        y: 0.0,
        w: 0.0,
        h: 0.0,
    };

    #[must_use]
    pub fn new(x: f64, y: f64, w: f64, h: f64) -> Rect {
        Rect { x, y, w, h }
    }

    #[must_use]
    pub fn right(&self) -> f64 {
        self.x + self.w
    }

    #[must_use]
    pub fn bottom(&self) -> f64 {
        self.y + self.h
    }

    /// Returns true when the point lies inside the rectangle. The left and top
    /// edges are inclusive and the right and bottom edges are exclusive, so
    /// adjacent rectangles never both claim the same point.
    #[must_use]
    pub fn contains(&self, px: f64, py: f64) -> bool {
        px >= self.x && px < self.right() && py >= self.y && py < self.bottom()
    }

    /// Returns true when `inner` is fully contained within `self`, allowing a
    /// small epsilon for floating point rounding.
    #[must_use]
    pub fn contains_rect(&self, inner: &Rect, eps: f64) -> bool {
        inner.x >= self.x - eps
            && inner.y >= self.y - eps
            && inner.right() <= self.right() + eps
            && inner.bottom() <= self.bottom() + eps
    }

    /// Returns true when two rectangles share interior area. Touching edges do
    /// not count as an overlap.
    #[must_use]
    pub fn overlaps(&self, other: &Rect, eps: f64) -> bool {
        self.x < other.right() - eps
            && other.x < self.right() - eps
            && self.y < other.bottom() - eps
            && other.y < self.bottom() - eps
    }
}
