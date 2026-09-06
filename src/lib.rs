//! Loom is a dependency free, retained mode GUI layout engine.
//!
//! The crate has no external dependencies and targets the 2021 edition. The
//! provable core is a flexbox style constraint solver that turns a tree of
//! widgets and a container size into an absolute rectangle for every node.
//!
//! # Quickstart
//!
//! ```
//! use loom::prelude::*;
//!
//! let mut root = Node::row()
//!     .width(200.0)
//!     .height(40.0)
//!     .gap(8.0)
//!     .child(Node::text("File").width(48.0))
//!     .child(Node::spacer().grow(1.0))
//!     .child(Node::button("Save").width(64.0));
//!
//! assign_ids(&mut root);
//! compute_layout(&mut root, Size::new(200.0, 40.0));
//!
//! // The spacer absorbs all the leftover space on the main axis.
//! assert_eq!(root.children[1].rect.w, 200.0 - 48.0 - 64.0 - 8.0 * 2.0);
//! ```

pub mod event;
pub mod geometry;
pub mod layout;
pub mod render;
pub mod style;
pub mod widget;

pub mod prelude {
    //! The common imports for building and laying out trees.
    pub use crate::event::{hit_path, hit_test};
    pub use crate::geometry::{Point, Rect, Size};
    pub use crate::layout::{compute_layout, natural_size};
    pub use crate::render::{render, DrawCall, RecordingRenderer, Renderer};
    pub use crate::style::{Align, Dimension, Direction, EdgeInsets, FlexWrap, Justify, Style};
    pub use crate::widget::{assign_ids, Node, WidgetKind, CHAR_W, LINE_H};
}

use std::fmt::Write;

use crate::widget::{Node, WidgetKind};

/// Formats a laid out tree as an indented rectangle listing. Each line shows
/// the node id, kind and its computed border box.
#[must_use]
pub fn format_tree(root: &Node) -> String {
    let mut out = String::new();
    format_node(root, 0, &mut out);
    out
}

fn format_node(node: &Node, depth: usize, out: &mut String) {
    let indent = "  ".repeat(depth);
    let label = match &node.kind {
        WidgetKind::Container => "Container".to_string(),
        WidgetKind::Text(s) => format!("Text {s:?}"),
        WidgetKind::Button(s) => format!("Button {s:?}"),
        WidgetKind::Box => "Box".to_string(),
        WidgetKind::Spacer => "Spacer".to_string(),
    };
    let _ = writeln!(
        out,
        "{indent}#{id} {label} x={x:.1} y={y:.1} w={w:.1} h={h:.1}",
        id = node.id,
        x = node.rect.x,
        y = node.rect.y,
        w = node.rect.w,
        h = node.rect.h,
    );
    for child in &node.children {
        format_node(child, depth + 1, out);
    }
}

/// Builds a representative sample UI used by the CLI and the demo. It exercises
/// nesting, flex grow, a spacer, gaps, padding and fixed sizes.
#[must_use]
pub fn sample_ui() -> Node {
    use crate::style::{Align, EdgeInsets, Justify};

    let toolbar = Node::row()
        .height(48.0)
        .padding(EdgeInsets::symmetric(8.0, 12.0))
        .gap(8.0)
        .align(Align::Center)
        .child(Node::button("File").width(56.0).height(28.0))
        .child(Node::button("Edit").width(56.0).height(28.0))
        .child(Node::spacer().grow(1.0))
        .child(Node::button("Save").width(72.0).height(28.0));

    let sidebar = Node::column()
        .width(160.0)
        .padding(EdgeInsets::all(12.0))
        .gap(6.0)
        .child(Node::text("Home").height(20.0))
        .child(Node::text("Projects").height(20.0))
        .child(Node::text("Settings").height(20.0))
        .child(Node::spacer().grow(1.0))
        .child(Node::text("Account").height(20.0));

    let content = Node::column()
        .grow(1.0)
        .padding(EdgeInsets::all(16.0))
        .gap(12.0)
        .child(Node::text("Welcome to Loom").height(24.0))
        .child(Node::boxed().grow(1.0))
        .child(
            Node::row()
                .height(36.0)
                .gap(8.0)
                .justify(Justify::End)
                .child(Node::button("Cancel").width(80.0).height(36.0))
                .child(Node::button("Confirm").width(80.0).height(36.0)),
        );

    let body = Node::row()
        .grow(1.0)
        .child(sidebar)
        .child(content);

    Node::column().child(toolbar).child(body)
}
