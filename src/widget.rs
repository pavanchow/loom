//! The retained mode widget tree.

use crate::geometry::{Rect, Size};
use crate::style::{Align, Dimension, Direction, EdgeInsets, FlexWrap, Justify, Style};

/// Width of a single character used to measure text intrinsic size.
pub const CHAR_W: f64 = 8.0;
/// Height of a single line of text.
pub const LINE_H: f64 = 16.0;

/// The kind of a widget. Containers hold children; leaves do not.
#[derive(Debug, Clone, PartialEq)]
pub enum WidgetKind {
    /// A layout container that arranges its children along an axis.
    Container,
    /// A run of text. Its intrinsic size is derived from the string length.
    Text(String),
    /// A clickable button that carries a text label.
    Button(String),
    /// A plain rectangle with no intrinsic content size.
    Box,
    /// An empty flexible gap, typically given `flex_grow` to push siblings apart.
    Spacer,
}

/// A node in the widget tree. A node owns its children and, after layout, its
/// computed border box rectangle.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub id: usize,
    pub kind: WidgetKind,
    pub style: Style,
    pub children: Vec<Node>,
    /// The computed border box, filled in by the layout pass.
    pub rect: Rect,
}

impl Node {
    #[must_use]
    pub fn new(kind: WidgetKind) -> Node {
        Node {
            id: 0,
            kind,
            style: Style::default(),
            children: Vec::new(),
            rect: Rect::ZERO,
        }
    }

    #[must_use]
    pub fn container(direction: Direction) -> Node {
        let mut n = Node::new(WidgetKind::Container);
        n.style.direction = direction;
        n
    }

    #[must_use]
    pub fn row() -> Node {
        Node::container(Direction::Row)
    }

    #[must_use]
    pub fn column() -> Node {
        Node::container(Direction::Column)
    }

    #[must_use]
    pub fn text(s: &str) -> Node {
        Node::new(WidgetKind::Text(s.to_string()))
    }

    #[must_use]
    pub fn button(s: &str) -> Node {
        Node::new(WidgetKind::Button(s.to_string()))
    }

    #[must_use]
    pub fn boxed() -> Node {
        Node::new(WidgetKind::Box)
    }

    #[must_use]
    pub fn spacer() -> Node {
        Node::new(WidgetKind::Spacer)
    }

    // Builder style setters. These consume and return self so trees can be
    // written declaratively.

    #[must_use]
    pub fn with_style(mut self, style: Style) -> Node {
        self.style = style;
        self
    }

    #[must_use]
    pub fn width(mut self, v: f64) -> Node {
        self.style.width = Dimension::Points(v);
        self
    }

    #[must_use]
    pub fn height(mut self, v: f64) -> Node {
        self.style.height = Dimension::Points(v);
        self
    }

    #[must_use]
    pub fn grow(mut self, v: f64) -> Node {
        self.style.flex_grow = v;
        self
    }

    #[must_use]
    pub fn shrink(mut self, v: f64) -> Node {
        self.style.flex_shrink = v;
        self
    }

    #[must_use]
    pub fn gap(mut self, v: f64) -> Node {
        self.style.gap = v;
        self
    }

    #[must_use]
    pub fn justify(mut self, v: Justify) -> Node {
        self.style.justify = v;
        self
    }

    #[must_use]
    /// Enable wrapping so children break onto new lines when the line is full.
    #[must_use]
    pub fn wrap(mut self) -> Node {
        self.style.flex_wrap = FlexWrap::Wrap;
        self
    }

    pub fn align(mut self, v: Align) -> Node {
        self.style.align = v;
        self
    }

    #[must_use]
    pub fn padding(mut self, v: EdgeInsets) -> Node {
        self.style.padding = v;
        self
    }

    #[must_use]
    pub fn border(mut self, v: EdgeInsets) -> Node {
        self.style.border = v;
        self
    }

    #[must_use]
    pub fn margin(mut self, v: EdgeInsets) -> Node {
        self.style.margin = v;
        self
    }

    #[must_use]
    pub fn child(mut self, c: Node) -> Node {
        self.children.push(c);
        self
    }

    #[must_use]
    pub fn children(mut self, cs: Vec<Node>) -> Node {
        self.children = cs;
        self
    }

    #[must_use]
    pub fn is_container(&self) -> bool {
        matches!(self.kind, WidgetKind::Container)
    }

    /// The intrinsic content size of a leaf, before padding and border are
    /// added. Containers return zero here because their content size is derived
    /// from their children in `natural_size`.
    #[must_use]
    pub fn leaf_content_size(&self) -> Size {
        match &self.kind {
            WidgetKind::Text(s) | WidgetKind::Button(s) => {
                #[allow(clippy::cast_precision_loss)] // text widths are character counts times a constant
                let w = s.chars().count() as f64 * CHAR_W;
                Size::new(w, LINE_H)
            }
            WidgetKind::Box | WidgetKind::Spacer | WidgetKind::Container => Size::ZERO,
        }
    }
}

/// Assigns a unique, stable id to every node in preorder. Ids are used by hit
/// testing and the recording renderer to refer to nodes.
pub fn assign_ids(root: &mut Node) {
    let mut next = 0usize;
    assign_ids_inner(root, &mut next);
}

fn assign_ids_inner(node: &mut Node, next: &mut usize) {
    node.id = *next;
    *next += 1;
    for child in &mut node.children {
        assign_ids_inner(child, next);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)] // asserted floats are literal constants assigned once

    use super::*;

    #[test]
    fn text_intrinsic_size() {
        let n = Node::text("hello");
        let s = n.leaf_content_size();
        assert_eq!(s.w, 5.0 * CHAR_W);
        assert_eq!(s.h, LINE_H);
    }

    #[test]
    fn box_has_no_intrinsic_content() {
        assert_eq!(Node::boxed().leaf_content_size(), Size::ZERO);
        assert_eq!(Node::spacer().leaf_content_size(), Size::ZERO);
    }

    #[test]
    fn ids_are_preorder_and_unique() {
        let mut root = Node::row().children(vec![
            Node::text("a"),
            Node::column().children(vec![Node::text("b"), Node::text("c")]),
        ]);
        assign_ids(&mut root);
        assert_eq!(root.id, 0);
        assert_eq!(root.children[0].id, 1);
        assert_eq!(root.children[1].id, 2);
        assert_eq!(root.children[1].children[0].id, 3);
        assert_eq!(root.children[1].children[1].id, 4);
    }
}
