//! The flexbox style constraint layout solver.
//!
//! The solver runs top down. Given a root node and an available size it assigns
//! an absolute border box rectangle to every node in the tree. A border box
//! includes the node border and padding but excludes its margin.

use crate::geometry::{Rect, Size};
use crate::style::{Align, Dimension, Direction, EdgeInsets, FlexWrap, Justify, Style};
use crate::widget::Node;

/// Maps a non-finite value (NaN or an infinity) to zero, leaving finite values
/// untouched. Used to reject non-finite inputs at the layout boundary.
fn finite(v: f64) -> f64 {
    if v.is_finite() {
        v
    } else {
        0.0
    }
}

/// A non-finite fixed length is not a usable size, so it degrades to `Auto` and
/// the node falls back to content or fill sizing instead of poisoning the tree.
fn sanitize_dim(d: Dimension) -> Dimension {
    match d {
        Dimension::Points(v) if !v.is_finite() => Dimension::Auto,
        other => other,
    }
}

fn sanitize_insets(e: EdgeInsets) -> EdgeInsets {
    EdgeInsets {
        top: finite(e.top),
        right: finite(e.right),
        bottom: finite(e.bottom),
        left: finite(e.left),
    }
}

fn sanitize_style(s: &mut Style) {
    s.gap = finite(s.gap).max(0.0);
    s.flex_grow = finite(s.flex_grow);
    s.flex_shrink = finite(s.flex_shrink);
    s.width = sanitize_dim(s.width);
    s.height = sanitize_dim(s.height);
    s.min_width = sanitize_dim(s.min_width);
    s.max_width = sanitize_dim(s.max_width);
    s.min_height = sanitize_dim(s.min_height);
    s.max_height = sanitize_dim(s.max_height);
    s.margin = sanitize_insets(s.margin);
    s.border = sanitize_insets(s.border);
    s.padding = sanitize_insets(s.padding);
}

/// Walks the tree once and normalizes every non-finite style value. This is the
/// single choke point that guarantees the solver only ever sees finite input,
/// no matter how a `Style` was constructed.
fn sanitize_tree(node: &mut Node) {
    sanitize_style(&mut node.style);
    for child in &mut node.children {
        sanitize_tree(child);
    }
}

/// Clamps a resolved length to the `[min, max]` bounds for one axis. `Auto`
/// bounds mean no minimum (zero) and no maximum (infinity). When a minimum
/// exceeds a maximum the minimum wins, matching the CSS resolution order.
fn clamp_axis(v: f64, min: Dimension, max: Dimension) -> f64 {
    let lo = match min {
        Dimension::Points(m) => m.max(0.0),
        Dimension::Auto => 0.0,
    };
    let hi = match max {
        Dimension::Points(m) => m.max(0.0),
        Dimension::Auto => f64::INFINITY,
    };
    v.clamp(lo, lo.max(hi))
}

fn axis_main(s: Size, dir: Direction) -> f64 {
    match dir {
        Direction::Row => s.w,
        Direction::Column => s.h,
    }
}

fn axis_cross(s: Size, dir: Direction) -> f64 {
    match dir {
        Direction::Row => s.h,
        Direction::Column => s.w,
    }
}

fn from_axes(dir: Direction, main: f64, cross: f64) -> Size {
    match dir {
        Direction::Row => Size::new(main, cross),
        Direction::Column => Size::new(cross, main),
    }
}

/// The natural border box size of a node with no external constraint. For a
/// leaf this is the intrinsic content size plus padding and border, unless a
/// fixed size overrides it. For a container it is the shrink to fit size of its
/// children plus its own padding and border.
#[must_use]
pub fn natural_size(node: &Node) -> Size {
    let s = &node.style;
    let inner = s.inner();
    let content = if node.is_container() {
        container_content_size(node)
    } else {
        node.leaf_content_size()
    };
    let w = match s.width {
        Dimension::Points(v) => v,
        Dimension::Auto => content.w + inner.horizontal(),
    };
    let h = match s.height {
        Dimension::Points(v) => v,
        Dimension::Auto => content.h + inner.vertical(),
    };
    let w = clamp_axis(w.max(0.0), s.min_width, s.max_width);
    let h = clamp_axis(h.max(0.0), s.min_height, s.max_height);
    Size::new(w, h)
}

/// The natural content size of a container derived from its children. The main
/// axis is the sum of child main sizes, gaps and child main margins. The cross
/// axis is the largest child cross size including its cross margins.
fn container_content_size(node: &Node) -> Size {
    let dir = node.style.direction;
    let n = node.children.len();
    #[allow(clippy::cast_precision_loss)] // child counts sit far below the f64 mantissa limit
    let nf = n as f64;
    let mut main = 0.0;
    let mut cross: f64 = 0.0;
    for child in &node.children {
        let cs = natural_size(child);
        let m = &child.style.margin;
        main += axis_main(cs, dir) + m.main_total(dir);
        cross = cross.max(axis_cross(cs, dir) + m.cross_total(dir));
    }
    if n > 1 {
        main += node.style.gap * (nf - 1.0);
    }
    from_axes(dir, main, cross)
}

/// Runs layout for a whole tree. The root takes its fixed size if one is set,
/// otherwise it fills the available size.
pub fn compute_layout(root: &mut Node, avail: Size) {
    sanitize_tree(root);
    let avail = Size::new(finite(avail.w), finite(avail.h));
    let w = root.style.width.resolve_or(avail.w);
    let h = root.style.height.resolve_or(avail.h);
    let w = clamp_axis(w.max(0.0), root.style.min_width, root.style.max_width);
    let h = clamp_axis(h.max(0.0), root.style.min_height, root.style.max_height);
    layout_node(root, Rect::new(0.0, 0.0, w, h));
}

/// Assigns `rect` as this node border box, then arranges its children inside
/// the resulting content box.
fn layout_node(node: &mut Node, rect: Rect) {
    node.rect = Rect::new(rect.x, rect.y, rect.w.max(0.0), rect.h.max(0.0));
    if !node.is_container() || node.children.is_empty() {
        return;
    }
    arrange_children(node);
}

fn justify_offset(justify: Justify, free: f64, n: usize) -> (f64, f64) {
    #[allow(clippy::cast_precision_loss)] // child counts sit far below the f64 mantissa limit
    let nf = n as f64;
    match justify {
        Justify::Start => (0.0, 0.0),
        Justify::Center => (free / 2.0, 0.0),
        Justify::End => (free, 0.0),
        Justify::SpaceBetween => {
            if n > 1 {
                (0.0, free / (nf - 1.0))
            } else {
                // A lone item has no gaps to spread, so it sits at the start.
                (0.0, 0.0)
            }
        }
    }
}

fn align_offset(align: Align, cross_free: f64) -> f64 {
    match align {
        Align::Start | Align::Stretch => 0.0,
        Align::Center => cross_free / 2.0,
        Align::End => cross_free,
    }
}

#[allow(clippy::too_many_lines)] // the main-axis solver reads as one continuous pass
fn arrange_children(node: &mut Node) {
    if node.style.flex_wrap == FlexWrap::Wrap {
        arrange_wrapped(node);
        return;
    }
    let dir = node.style.direction;
    let align = node.style.align;
    let justify = node.style.justify;
    let gap = node.style.gap;
    let inner = node.style.inner();

    let content_x = node.rect.x + inner.left;
    let content_y = node.rect.y + inner.top;
    let content_w = (node.rect.w - inner.horizontal()).max(0.0);
    let content_h = (node.rect.h - inner.vertical()).max(0.0);
    let content = Size::new(content_w, content_h);
    let main_avail = axis_main(content, dir);
    let cross_avail = axis_cross(content, dir);

    let n = node.children.len();
    #[allow(clippy::cast_precision_loss)] // child counts sit far below the f64 mantissa limit
    let nf = n as f64;
    let total_gap = if n > 1 { gap * (nf - 1.0) } else { 0.0 };

    // Per child measurements along the main axis.
    let naturals: Vec<Size> = node.children.iter().map(natural_size).collect();
    let base: Vec<f64> = node
        .children
        .iter()
        .zip(naturals.iter())
        .map(|(child, nat)| {
            let raw = match child.style.main_dim(dir) {
                Dimension::Points(v) => v.max(0.0),
                Dimension::Auto => axis_main(*nat, dir),
            };
            clamp_axis(raw, child.style.min_main_dim(dir), child.style.max_main_dim(dir))
        })
        .collect();
    let grow: Vec<f64> = node.children.iter().map(|c| c.style.flex_grow).collect();
    let shrink: Vec<f64> = node.children.iter().map(|c| c.style.flex_shrink).collect();

    let used_margin: f64 = node
        .children
        .iter()
        .map(|c| c.style.margin.main_total(dir))
        .sum();
    let base_sum: f64 = base.iter().sum();
    let free = main_avail - base_sum - used_margin - total_gap;
    let total_grow: f64 = grow.iter().sum();

    let mut main_size = base.clone();
    let should_grow = free > 0.0 && total_grow > 0.0;
    if should_grow {
        for (ms, (g, b)) in main_size.iter_mut().zip(grow.iter().zip(base.iter())) {
            if *g > 0.0 {
                *ms = *b + free * *g / total_grow;
            }
        }
    } else if free < 0.0 {
        // Resolve shrink iteratively: clamped children freeze and the rest of
        // the deficit redistributes until the line fits or nothing can shrink.
        resolve_shrink(&mut main_size, &base, &shrink, -free);
    }

    // Justify only distributes leftover space when nothing grew to consume it.
    let (offset, spacing) = if should_grow {
        (0.0, 0.0)
    } else {
        justify_offset(justify, free.max(0.0), n)
    };

    let main_origin = match dir {
        Direction::Row => content_x,
        Direction::Column => content_y,
    };
    let cross_origin = match dir {
        Direction::Row => content_y,
        Direction::Column => content_x,
    };

    let mut cursor = main_origin + offset;
    for i in 0..n {
        let margin = node.children[i].style.margin;
        cursor += margin.main_start(dir);

        let cross_raw = match node.children[i].style.cross_dim(dir) {
            Dimension::Points(v) => v.max(0.0),
            Dimension::Auto => {
                if align == Align::Stretch {
                    (cross_avail - margin.cross_total(dir)).max(0.0)
                } else {
                    axis_cross(naturals[i], dir)
                }
            }
        };
        let cross_size = clamp_axis(
            cross_raw,
            node.children[i].style.min_cross_dim(dir),
            node.children[i].style.max_cross_dim(dir),
        );
        let main = clamp_axis(
            main_size[i],
            node.children[i].style.min_main_dim(dir),
            node.children[i].style.max_main_dim(dir),
        );
        let cross_free = cross_avail - cross_size - margin.cross_total(dir);
        let cross_pos = cross_origin + margin.cross_start(dir) + align_offset(align, cross_free);

        let child_rect = match dir {
            Direction::Row => Rect::new(cursor, cross_pos, main, cross_size),
            Direction::Column => Rect::new(cross_pos, cursor, cross_size, main),
        };
        layout_node(&mut node.children[i], child_rect);

        cursor += main + margin.main_end(dir);
        if i + 1 < n {
            cursor += gap + spacing;
        }
    }
}

/// Iteratively resolve a main-axis deficit across shrinkable children. A
/// child that would collapse below zero clamps to zero, freezes, and the
/// remaining deficit redistributes among the children that can still shrink.
fn resolve_shrink(main_size: &mut [f64], base: &[f64], shrink: &[f64], deficit: f64) {
    let mut remaining = deficit;
    let mut frozen = vec![false; main_size.len()];
    loop {
        let scaled: f64 = (0..main_size.len())
            .filter(|&i| !frozen[i] && shrink[i] > 0.0)
            .map(|i| shrink[i] * base[i])
            .sum();
        if scaled <= 0.0 || remaining <= 1e-9 {
            break;
        }
        let mut froze_any = false;
        for i in 0..main_size.len() {
            if frozen[i] || shrink[i] <= 0.0 {
                continue;
            }
            let reduce = remaining * (shrink[i] * base[i]) / scaled;
            if reduce >= main_size[i] {
                remaining -= main_size[i];
                main_size[i] = 0.0;
                frozen[i] = true;
                froze_any = true;
            }
        }
        if froze_any {
            continue;
        }
        for i in 0..main_size.len() {
            if frozen[i] || shrink[i] <= 0.0 {
                continue;
            }
            let reduce = remaining * (shrink[i] * base[i]) / scaled;
            main_size[i] = (main_size[i] - reduce).max(0.0);
        }
        break;
    }
}

/// Arrange children of a wrap container. Children break onto lines greedily:
/// a line stays open while the running main sizes plus gaps fit the content
/// main axis, and a child that alone overflows gets a line of its own. Each
/// line resolves grow, shrink, justify, and child align exactly like a
/// `NoWrap` container over its members. A line's cross size is the largest
/// member cross extent, and stretch children fill their line rather than the
/// container. Lines stack from the content origin along the cross axis.
#[allow(clippy::too_many_lines)] // the wrap solver reads as one continuous pass
fn arrange_wrapped(node: &mut Node) {
    let dir = node.style.direction;
    let align = node.style.align;
    let justify = node.style.justify;
    let gap = node.style.gap;
    let inner = node.style.inner();

    let content_x = node.rect.x + inner.left;
    let content_y = node.rect.y + inner.top;
    let content_w = (node.rect.w - inner.horizontal()).max(0.0);
    let content_h = (node.rect.h - inner.vertical()).max(0.0);
    let content = Size::new(content_w, content_h);
    let main_avail = axis_main(content, dir);

    let naturals: Vec<Size> = node.children.iter().map(natural_size).collect();
    let base: Vec<f64> = node
        .children
        .iter()
        .zip(naturals.iter())
        .map(|(child, nat)| {
            let raw = match child.style.main_dim(dir) {
                Dimension::Points(v) => v.max(0.0),
                Dimension::Auto => axis_main(*nat, dir),
            };
            clamp_axis(raw, child.style.min_main_dim(dir), child.style.max_main_dim(dir))
        })
        .collect();
    let grow: Vec<f64> = node.children.iter().map(|c| c.style.flex_grow).collect();
    let shrink: Vec<f64> = node.children.iter().map(|c| c.style.flex_shrink).collect();

    // Greedy line breaking over members in order.
    let mut lines: Vec<Vec<usize>> = Vec::new();
    let mut current: Vec<usize> = Vec::new();
    let mut current_cost = 0.0;
    for (i, &b) in base.iter().enumerate() {
        let cost = b + node.children[i].style.margin.main_total(dir);
        let add_gap = if current.is_empty() { 0.0 } else { gap };
        if !current.is_empty() && current_cost + add_gap + cost > main_avail + 1e-9 {
            lines.push(std::mem::take(&mut current));
            current_cost = 0.0;
        }
        let add_gap = if current.is_empty() { 0.0 } else { gap };
        current.push(i);
        current_cost += add_gap + cost;
    }
    if !current.is_empty() {
        lines.push(current);
    }

    let mut cross_cursor = match dir {
        Direction::Row => content_y,
        Direction::Column => content_x,
    };

    for members in &lines {
        let line_len = members.len();
        #[allow(clippy::cast_precision_loss)] // line member counts sit far below the f64 mantissa limit
        let nf = line_len as f64;
        let line_gaps = if line_len > 1 { gap * (nf - 1.0) } else { 0.0 };
        let line_margins: f64 = members
            .iter()
            .map(|&i| node.children[i].style.margin.main_total(dir))
            .sum();
        let line_base: f64 = members.iter().map(|&i| base[i]).sum();
        let free = main_avail - line_base - line_margins - line_gaps;
        let total_grow: f64 = members.iter().map(|&i| grow[i]).sum();

        // Per-member main sizes within this line.
        let mut sizes: Vec<f64> = members.iter().map(|&i| base[i]).collect();
        let should_grow = free > 0.0 && total_grow > 0.0;
        if should_grow {
            for (ms, &i) in sizes.iter_mut().zip(members.iter()) {
                if grow[i] > 0.0 {
                    *ms = base[i] + free * grow[i] / total_grow;
                }
            }
        } else if free < 0.0 {
            let line_shrink: Vec<f64> = members.iter().map(|&i| shrink[i]).collect();
            let line_base_vec: Vec<f64> = members.iter().map(|&i| base[i]).collect();
            resolve_shrink(&mut sizes, &line_base_vec, &line_shrink, -free);
        }

        // The line's cross extent is the largest member cross footprint.
        let line_cross = members
            .iter()
            .map(|&i| {
                let m = &node.children[i].style.margin;
                let cs_raw = match node.children[i].style.cross_dim(dir) {
                    Dimension::Points(v) => v.max(0.0),
                    Dimension::Auto => axis_cross(naturals[i], dir),
                };
                let cs = clamp_axis(
                    cs_raw,
                    node.children[i].style.min_cross_dim(dir),
                    node.children[i].style.max_cross_dim(dir),
                );
                cs + m.cross_total(dir)
            })
            .fold(0.0, f64::max);

        let (offset, spacing) = if should_grow {
            (0.0, 0.0)
        } else {
            justify_offset(justify, free.max(0.0), line_len)
        };

        let main_origin = match dir {
            Direction::Row => content_x,
            Direction::Column => content_y,
        };
        let mut cursor = main_origin + offset;
        for (j, &i) in members.iter().enumerate() {
            let slot = sizes[j];
            let margin = node.children[i].style.margin;
            cursor += margin.main_start(dir);

            let cross_raw = match node.children[i].style.cross_dim(dir) {
                Dimension::Points(v) => v.max(0.0),
                Dimension::Auto => {
                    if align == Align::Stretch {
                        (line_cross - margin.cross_total(dir)).max(0.0)
                    } else {
                        axis_cross(naturals[i], dir)
                    }
                }
            };
            let cross_size = clamp_axis(
                cross_raw,
                node.children[i].style.min_cross_dim(dir),
                node.children[i].style.max_cross_dim(dir),
            );
            let main = clamp_axis(
                slot,
                node.children[i].style.min_main_dim(dir),
                node.children[i].style.max_main_dim(dir),
            );
            let cross_free = line_cross - cross_size - margin.cross_total(dir);
            let cross_pos = cross_cursor + margin.cross_start(dir) + align_offset(align, cross_free);

            let child_rect = match dir {
                Direction::Row => Rect::new(cursor, cross_pos, main, cross_size),
                Direction::Column => Rect::new(cross_pos, cursor, cross_size, main),
            };
            layout_node(&mut node.children[i], child_rect);

            cursor += main + margin.main_end(dir);
            if j + 1 < line_len {
                cursor += gap + spacing;
            }
        }
        cross_cursor += line_cross;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::EdgeInsets;
    use crate::widget::{assign_ids, Node};

    #[test]
    fn single_flex_child_fills_main_axis() {
        let mut root = Node::row()
            .width(100.0)
            .height(40.0)
            .child(Node::boxed().grow(1.0));
        assign_ids(&mut root);
        compute_layout(&mut root, Size::new(100.0, 40.0));
        assert_eq!(root.children[0].rect, Rect::new(0.0, 0.0, 100.0, 40.0));
    }

    #[test]
    fn two_equal_flex_children_split_main_axis() {
        let mut root = Node::row()
            .width(100.0)
            .height(20.0)
            .child(Node::boxed().grow(1.0))
            .child(Node::boxed().grow(1.0));
        compute_layout(&mut root, Size::new(100.0, 20.0));
        assert_eq!(root.children[0].rect, Rect::new(0.0, 0.0, 50.0, 20.0));
        assert_eq!(root.children[1].rect, Rect::new(50.0, 0.0, 50.0, 20.0));
    }


    #[test]
    fn wrap_breaks_lines_and_stacks() {
        let mut root = Node::row()
            .width(100.0)
            .height(40.0)
            .wrap()
            .child(Node::boxed().width(40.0).height(10.0))
            .child(Node::boxed().width(40.0).height(10.0))
            .child(Node::boxed().width(40.0).height(10.0));
        compute_layout(&mut root, Size::new(100.0, 40.0));
        assert_eq!(root.children[0].rect, Rect::new(0.0, 0.0, 40.0, 10.0));
        assert_eq!(root.children[1].rect, Rect::new(40.0, 0.0, 40.0, 10.0));
        assert_eq!(root.children[2].rect, Rect::new(0.0, 10.0, 40.0, 10.0));
    }

    #[test]
    fn wrap_justify_applies_per_line() {
        let mut root = Node::row()
            .width(100.0)
            .height(30.0)
            .wrap()
            .justify(Justify::SpaceBetween)
            .child(Node::boxed().width(40.0).height(10.0))
            .child(Node::boxed().width(40.0).height(10.0))
            .child(Node::boxed().width(40.0).height(10.0));
        compute_layout(&mut root, Size::new(100.0, 30.0));
        // The first line holds two children with 20 free, spread apart. The
        // second line starts a fresh justify pass at the line origin.
        assert_eq!(root.children[0].rect, Rect::new(0.0, 0.0, 40.0, 10.0));
        assert_eq!(root.children[1].rect, Rect::new(60.0, 0.0, 40.0, 10.0));
        assert_eq!(root.children[2].rect, Rect::new(0.0, 10.0, 40.0, 10.0));
    }

    #[test]
    fn wrap_stretch_fills_line_cross() {
        let mut root = Node::row()
            .width(100.0)
            .height(40.0)
            .wrap()
            .child(Node::boxed().width(40.0).height(10.0))
            .child(Node::boxed().width(40.0))
            .child(Node::boxed().width(40.0).height(10.0));
        compute_layout(&mut root, Size::new(100.0, 40.0));
        // The auto-height child stretches to its line's cross extent of 10.
        assert_eq!(root.children[0].rect, Rect::new(0.0, 0.0, 40.0, 10.0));
        assert_eq!(root.children[1].rect, Rect::new(40.0, 0.0, 40.0, 10.0));
        assert_eq!(root.children[2].rect, Rect::new(0.0, 10.0, 40.0, 10.0));
    }

    #[test]
    fn wrapped_column_stacks_lines_along_x() {
        let mut root = Node::container(Direction::Column)
            .width(40.0)
            .height(100.0)
            .wrap()
            .child(Node::boxed().width(10.0).height(40.0))
            .child(Node::boxed().width(10.0).height(40.0))
            .child(Node::boxed().width(10.0).height(40.0));
        compute_layout(&mut root, Size::new(40.0, 100.0));
        assert_eq!(root.children[0].rect, Rect::new(0.0, 0.0, 10.0, 40.0));
        assert_eq!(root.children[1].rect, Rect::new(0.0, 40.0, 10.0, 40.0));
        assert_eq!(root.children[2].rect, Rect::new(10.0, 0.0, 10.0, 40.0));
    }

    #[test]
    fn wrap_oversized_child_gets_its_own_line() {
        let mut root = Node::row()
            .width(100.0)
            .height(40.0)
            .wrap()
            .child(Node::boxed().width(30.0).height(10.0))
            .child(Node::boxed().width(120.0).height(10.0))
            .child(Node::boxed().width(30.0).height(10.0));
        compute_layout(&mut root, Size::new(100.0, 40.0));
        // The 120-wide child cannot share a line, overflows its own line on
        // the main axis, and later children continue on a fresh line.
        assert_eq!(root.children[0].rect, Rect::new(0.0, 0.0, 30.0, 10.0));
        assert_eq!(root.children[1].rect, Rect::new(0.0, 10.0, 120.0, 10.0));
        assert_eq!(root.children[2].rect, Rect::new(0.0, 20.0, 30.0, 10.0));
    }

    #[test]
    fn padding_shrinks_content_box() {
        let mut root = Node::row()
            .width(100.0)
            .height(100.0)
            .padding(EdgeInsets::all(10.0))
            .child(Node::boxed().grow(1.0));
        compute_layout(&mut root, Size::new(100.0, 100.0));
        assert_eq!(root.children[0].rect, Rect::new(10.0, 10.0, 80.0, 80.0));
    }

    #[test]
    fn gap_between_flex_children() {
        let mut root = Node::row()
            .width(100.0)
            .height(10.0)
            .gap(10.0)
            .child(Node::boxed().grow(1.0))
            .child(Node::boxed().grow(1.0));
        compute_layout(&mut root, Size::new(100.0, 10.0));
        assert_eq!(root.children[0].rect, Rect::new(0.0, 0.0, 45.0, 10.0));
        assert_eq!(root.children[1].rect, Rect::new(55.0, 0.0, 45.0, 10.0));
    }

    #[test]
    fn justify_center_positions_fixed_child() {
        let mut root = Node::row()
            .width(100.0)
            .height(10.0)
            .justify(Justify::Center)
            .child(Node::boxed().width(20.0).height(10.0));
        compute_layout(&mut root, Size::new(100.0, 10.0));
        assert_eq!(root.children[0].rect, Rect::new(40.0, 0.0, 20.0, 10.0));
    }
}
