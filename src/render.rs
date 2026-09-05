//! A headless rendering abstraction.
//!
//! The core never touches a real graphics API. Instead it walks the laid out
//! tree and emits draw calls into a [`Renderer`]. The [`RecordingRenderer`]
//! captures those calls so tests can assert on exactly what would be drawn.

use crate::geometry::Rect;
use crate::widget::{Node, WidgetKind};

/// A single primitive the engine asks the backend to draw.
#[derive(Debug, Clone, PartialEq)]
pub enum DrawCall {
    /// A filled or bordered rectangle for a container, box, button or spacer.
    Rect { id: usize, kind: &'static str, rect: Rect },
    /// A text run positioned at its border box.
    Text { id: usize, rect: Rect, text: String },
}

/// A drawing backend. Implementors turn draw calls into pixels, terminal
/// output, SVG or anything else.
pub trait Renderer {
    fn draw_rect(&mut self, id: usize, kind: &'static str, rect: Rect);
    fn draw_text(&mut self, id: usize, rect: Rect, text: &str);
}

/// A renderer that records every draw call in order instead of drawing.
#[derive(Debug, Default)]
pub struct RecordingRenderer {
    pub calls: Vec<DrawCall>,
}

impl RecordingRenderer {
    pub fn new() -> RecordingRenderer {
        RecordingRenderer { calls: Vec::new() }
    }
}

impl Renderer for RecordingRenderer {
    fn draw_rect(&mut self, id: usize, kind: &'static str, rect: Rect) {
        self.calls.push(DrawCall::Rect { id, kind, rect });
    }

    fn draw_text(&mut self, id: usize, rect: Rect, text: &str) {
        self.calls.push(DrawCall::Text {
            id,
            rect,
            text: text.to_string(),
        });
    }
}

fn kind_name(kind: &WidgetKind) -> &'static str {
    match kind {
        WidgetKind::Container => "container",
        WidgetKind::Text(_) => "text",
        WidgetKind::Button(_) => "button",
        WidgetKind::Box => "box",
        WidgetKind::Spacer => "spacer",
    }
}

/// Walks the laid out tree in preorder and emits draw calls. The tree must be
/// laid out first so every node has a computed rect.
pub fn render<R: Renderer>(node: &Node, r: &mut R) {
    match &node.kind {
        WidgetKind::Text(s) | WidgetKind::Button(s) => {
            r.draw_rect(node.id, kind_name(&node.kind), node.rect);
            r.draw_text(node.id, node.rect, s);
        }
        _ => {
            r.draw_rect(node.id, kind_name(&node.kind), node.rect);
        }
    }
    for child in &node.children {
        render(child, r);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Size;
    use crate::layout::compute_layout;
    use crate::widget::{assign_ids, Node};

    #[test]
    fn records_calls_in_preorder() {
        let mut root = Node::row()
            .width(100.0)
            .height(20.0)
            .child(Node::boxed().grow(1.0))
            .child(Node::text("hi").width(20.0));
        assign_ids(&mut root);
        compute_layout(&mut root, Size::new(100.0, 20.0));

        let mut rec = RecordingRenderer::new();
        render(&root, &mut rec);

        // container, box, then text emits both a rect and a text call.
        assert_eq!(rec.calls.len(), 4);
        match &rec.calls[0] {
            DrawCall::Rect { kind, .. } => assert_eq!(*kind, "container"),
            _ => panic!("expected container rect first"),
        }
        match &rec.calls[3] {
            DrawCall::Text { text, .. } => assert_eq!(text, "hi"),
            _ => panic!("expected text call last"),
        }
    }
}
