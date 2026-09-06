//! Style resolution: directions, alignment, sizing and the box model.

/// The main axis direction of a container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Children are placed left to right; the main axis is horizontal.
    Row,
    /// Children are placed top to bottom; the main axis is vertical.
    Column,
}

/// Distribution of free space along the main axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Justify {
    Start,
    Center,
    End,
    SpaceBetween,
}

/// Whether children wrap onto new lines when they exceed the main axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlexWrap {
    /// One line, overflowing if the children cannot shrink to fit.
    #[default]
    NoWrap,
    /// Children break onto new lines so each line's main sizes fit.
    Wrap,
}

/// Alignment of a child along the cross axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Start,
    Center,
    End,
    /// Grow to fill the cross axis of the parent content box, unless the child
    /// has a fixed cross size.
    Stretch,
}

/// A single length: either automatic (content derived) or a fixed number of
/// logical pixels. A fixed length sets the border box size of the node.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Dimension {
    Auto,
    Points(f64),
}

impl Dimension {
    #[must_use]
    pub fn is_fixed(&self) -> bool {
        matches!(self, Dimension::Points(_))
    }

    #[must_use]
    pub fn resolve_or(&self, fallback: f64) -> f64 {
        match self {
            Dimension::Auto => fallback,
            Dimension::Points(v) => *v,
        }
    }
}

/// Insets on the four edges of a box, used for margin, border and padding.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeInsets {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

impl EdgeInsets {
    pub const ZERO: EdgeInsets = EdgeInsets {
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
        left: 0.0,
    };

    #[must_use]
    pub fn all(v: f64) -> EdgeInsets {
        EdgeInsets {
            top: v,
            right: v,
            bottom: v,
            left: v,
        }
    }

    #[must_use]
    pub fn symmetric(vertical: f64, horizontal: f64) -> EdgeInsets {
        EdgeInsets {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }

    #[must_use]
    pub fn horizontal(&self) -> f64 {
        self.left + self.right
    }

    #[must_use]
    pub fn vertical(&self) -> f64 {
        self.top + self.bottom
    }

    /// The inset at the start of the main axis for the given direction.
    #[must_use]
    pub fn main_start(&self, dir: Direction) -> f64 {
        match dir {
            Direction::Row => self.left,
            Direction::Column => self.top,
        }
    }

    /// The inset at the end of the main axis for the given direction.
    #[must_use]
    pub fn main_end(&self, dir: Direction) -> f64 {
        match dir {
            Direction::Row => self.right,
            Direction::Column => self.bottom,
        }
    }

    /// The inset at the start of the cross axis for the given direction.
    #[must_use]
    pub fn cross_start(&self, dir: Direction) -> f64 {
        match dir {
            Direction::Row => self.top,
            Direction::Column => self.left,
        }
    }

    /// The inset at the end of the cross axis for the given direction.
    #[must_use]
    pub fn cross_end(&self, dir: Direction) -> f64 {
        match dir {
            Direction::Row => self.bottom,
            Direction::Column => self.right,
        }
    }

    #[must_use]
    pub fn main_total(&self, dir: Direction) -> f64 {
        self.main_start(dir) + self.main_end(dir)
    }

    #[must_use]
    pub fn cross_total(&self, dir: Direction) -> f64 {
        self.cross_start(dir) + self.cross_end(dir)
    }
}

/// The resolved style of a node. Every field has a sensible default so a node
/// can be built by overriding only what matters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    pub direction: Direction,
    pub flex_wrap: FlexWrap,
    pub justify: Justify,
    pub align: Align,
    pub gap: f64,
    pub width: Dimension,
    pub height: Dimension,
    /// Lower bound on the resolved width. `Auto` means no minimum.
    pub min_width: Dimension,
    /// Upper bound on the resolved width. `Auto` means no maximum.
    pub max_width: Dimension,
    /// Lower bound on the resolved height. `Auto` means no minimum.
    pub min_height: Dimension,
    /// Upper bound on the resolved height. `Auto` means no maximum.
    pub max_height: Dimension,
    pub flex_grow: f64,
    pub flex_shrink: f64,
    pub margin: EdgeInsets,
    pub border: EdgeInsets,
    pub padding: EdgeInsets,
}

impl Default for Style {
    fn default() -> Style {
        Style {
            direction: Direction::Row,
            flex_wrap: FlexWrap::NoWrap,
            justify: Justify::Start,
            align: Align::Stretch,
            gap: 0.0,
            width: Dimension::Auto,
            height: Dimension::Auto,
            min_width: Dimension::Auto,
            max_width: Dimension::Auto,
            min_height: Dimension::Auto,
            max_height: Dimension::Auto,
            flex_grow: 0.0,
            flex_shrink: 0.0,
            margin: EdgeInsets::ZERO,
            border: EdgeInsets::ZERO,
            padding: EdgeInsets::ZERO,
        }
    }
}

impl Style {
    /// The main axis fixed length for the given direction, if any.
    #[must_use]
    pub fn main_dim(&self, dir: Direction) -> Dimension {
        match dir {
            Direction::Row => self.width,
            Direction::Column => self.height,
        }
    }

    /// The cross axis fixed length for the given direction, if any.
    #[must_use]
    pub fn cross_dim(&self, dir: Direction) -> Dimension {
        match dir {
            Direction::Row => self.height,
            Direction::Column => self.width,
        }
    }

    /// The main axis minimum length for the given direction.
    #[must_use]
    pub fn min_main_dim(&self, dir: Direction) -> Dimension {
        match dir {
            Direction::Row => self.min_width,
            Direction::Column => self.min_height,
        }
    }

    /// The main axis maximum length for the given direction.
    #[must_use]
    pub fn max_main_dim(&self, dir: Direction) -> Dimension {
        match dir {
            Direction::Row => self.max_width,
            Direction::Column => self.max_height,
        }
    }

    /// The cross axis minimum length for the given direction.
    #[must_use]
    pub fn min_cross_dim(&self, dir: Direction) -> Dimension {
        match dir {
            Direction::Row => self.min_height,
            Direction::Column => self.min_width,
        }
    }

    /// The cross axis maximum length for the given direction.
    #[must_use]
    pub fn max_cross_dim(&self, dir: Direction) -> Dimension {
        match dir {
            Direction::Row => self.max_height,
            Direction::Column => self.max_width,
        }
    }

    /// Combined border plus padding insets. Content sits inside these.
    #[must_use]
    pub fn inner(&self) -> EdgeInsets {
        EdgeInsets {
            top: self.border.top + self.padding.top,
            right: self.border.right + self.padding.right,
            bottom: self.border.bottom + self.padding.bottom,
            left: self.border.left + self.padding.left,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)] // asserted floats are literal constants assigned once

    use super::*;

    #[test]
    fn edge_axis_mapping_row() {
        let e = EdgeInsets {
            top: 1.0,
            right: 2.0,
            bottom: 3.0,
            left: 4.0,
        };
        assert_eq!(e.main_start(Direction::Row), 4.0);
        assert_eq!(e.main_end(Direction::Row), 2.0);
        assert_eq!(e.cross_start(Direction::Row), 1.0);
        assert_eq!(e.cross_end(Direction::Row), 3.0);
        assert_eq!(e.main_total(Direction::Row), 6.0);
        assert_eq!(e.cross_total(Direction::Row), 4.0);
    }

    #[test]
    fn edge_axis_mapping_column() {
        let e = EdgeInsets {
            top: 1.0,
            right: 2.0,
            bottom: 3.0,
            left: 4.0,
        };
        assert_eq!(e.main_start(Direction::Column), 1.0);
        assert_eq!(e.main_end(Direction::Column), 3.0);
        assert_eq!(e.cross_start(Direction::Column), 4.0);
        assert_eq!(e.cross_end(Direction::Column), 2.0);
    }

    #[test]
    fn dimension_resolves() {
        assert_eq!(Dimension::Auto.resolve_or(10.0), 10.0);
        assert_eq!(Dimension::Points(5.0).resolve_or(10.0), 5.0);
        assert!(Dimension::Points(1.0).is_fixed());
        assert!(!Dimension::Auto.is_fixed());
    }

    #[test]
    fn inner_combines_border_and_padding() {
        let s = Style {
            border: EdgeInsets::all(2.0),
            padding: EdgeInsets::all(3.0),
            ..Style::default()
        };
        assert_eq!(s.inner(), EdgeInsets::all(5.0));
    }
}
