//! A simple event and hit testing model.
//!
//! Given a laid out tree and a point, hit testing returns the deepest node
//! whose border box contains the point. Because children are drawn on top of
//! their parents, the deepest match is the visually topmost one.

use crate::widget::Node;

/// Returns the id of the deepest node containing the point, or `None` when the
/// point falls outside the root. The last matching child in document order wins
/// among overlapping siblings, matching paint order.
#[must_use]
pub fn hit_test(root: &Node, px: f64, py: f64) -> Option<usize> {
    if !root.rect.contains(px, py) {
        return None;
    }
    // Search children in reverse so later siblings, painted on top, win.
    for child in root.children.iter().rev() {
        if let Some(hit) = hit_test(child, px, py) {
            return Some(hit);
        }
    }
    Some(root.id)
}

/// Returns the full path of node ids from the root down to the deepest hit.
/// Useful for event bubbling. Empty when the point is outside the root.
#[must_use]
pub fn hit_path(root: &Node, px: f64, py: f64) -> Vec<usize> {
    let mut path = Vec::new();
    collect_path(root, px, py, &mut path);
    path
}

fn collect_path(node: &Node, px: f64, py: f64, path: &mut Vec<usize>) -> bool {
    if !node.rect.contains(px, py) {
        return false;
    }
    path.push(node.id);
    for child in node.children.iter().rev() {
        if collect_path(child, px, py, path) {
            return true;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Size;
    use crate::layout::compute_layout;
    use crate::widget::{assign_ids, Node};

    fn tree() -> Node {
        let mut root = Node::row()
            .width(100.0)
            .height(50.0)
            .child(Node::boxed().width(40.0).height(50.0))
            .child(Node::boxed().width(60.0).height(50.0));
        assign_ids(&mut root);
        compute_layout(&mut root, Size::new(100.0, 50.0));
        root
    }

    #[test]
    fn point_inside_first_child() {
        let root = tree();
        assert_eq!(hit_test(&root, 10.0, 10.0), Some(root.children[0].id));
    }

    #[test]
    fn point_inside_second_child() {
        let root = tree();
        assert_eq!(hit_test(&root, 70.0, 10.0), Some(root.children[1].id));
    }

    #[test]
    fn point_outside_returns_none() {
        let root = tree();
        assert_eq!(hit_test(&root, 200.0, 200.0), None);
    }

    #[test]
    fn boundary_is_exclusive_on_right_edge() {
        let root = tree();
        // x == 40 belongs to the second child, not the first.
        assert_eq!(hit_test(&root, 40.0, 0.0), Some(root.children[1].id));
    }

    #[test]
    fn path_runs_root_to_leaf() {
        let mut root = Node::row()
            .width(100.0)
            .height(50.0)
            .child(
                Node::column()
                    .grow(1.0)
                    .child(Node::text("x").width(30.0).height(20.0)),
            );
        assign_ids(&mut root);
        compute_layout(&mut root, Size::new(100.0, 50.0));
        let path = hit_path(&root, 5.0, 5.0);
        assert_eq!(path, vec![0, 1, 2]);
    }
}
