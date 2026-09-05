//! Structural invariants checked over many randomly generated trees, plus
//! determinism and flex fill gates.
//!
//! The iteration count is bounded for CI and controllable with the LOOM_FUZZ_OPS
//! environment variable. The starting seed can be set with LOOM_FUZZ_SEED.

use loom::prelude::*;
use loom::style::Style;

const EPS: f64 = 1e-6;

// A tiny deterministic pseudo random generator so the fuzz tests need no
// external dependency and reproduce exactly from a seed.
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng {
            state: seed ^ 0x9e37_79b9_7f4a_7c15,
        }
    }

    fn next_u64(&mut self) -> u64 {
        // SplitMix64.
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }

    fn range_f64(&mut self, lo: f64, hi: f64) -> f64 {
        let t = (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
        lo + t * (hi - lo)
    }

    // A varied shrink factor. Zero appears often so that main axis overflow is
    // deliberately left unresolved and the containment behaviour is exercised.
    fn shrink_factor(&mut self) -> f64 {
        match self.below(4) {
            0 | 1 => 0.0,
            2 => 1.0,
            _ => self.range_f64(0.0, 2.0),
        }
    }

    fn align(&mut self) -> Align {
        match self.below(4) {
            0 => Align::Start,
            1 => Align::Center,
            2 => Align::End,
            _ => Align::Stretch,
        }
    }
}

fn env_u64(name: &str, fallback: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(fallback)
}

// Builds a random tree. Containers appear only above `leaf_depth`; the deepest
// level is always leaves. Children use varied flex shrink factors, including
// zero, and varied align values, so both axes are regularly driven into
// intentional overflow. The containment check is written to assert the
// documented overflow semantics on the overflowing axis rather than to assume
// everything always fits.
fn random_node(rng: &mut Rng, depth: u32, max_depth: u32) -> Node {
    let make_leaf = depth >= max_depth || rng.below(3) == 0;
    if make_leaf {
        let mut n = match rng.below(4) {
            0 => Node::text("widget"),
            1 => Node::button("go"),
            2 => Node::boxed(),
            _ => Node::spacer(),
        };
        n = n.grow(rng.range_f64(0.0, 2.0)).shrink(rng.shrink_factor());
        n = n.margin(EdgeInsets::all(rng.range_f64(0.0, 4.0)));
        return n;
    }

    let dir = if rng.below(2) == 0 {
        Direction::Row
    } else {
        Direction::Column
    };
    let justify = match rng.below(4) {
        0 => Justify::Start,
        1 => Justify::Center,
        2 => Justify::End,
        _ => Justify::SpaceBetween,
    };

    let count = 1 + rng.below(4) as usize;
    let align = rng.align();
    let mut node = Node::container(dir)
        .justify(justify)
        .align(align)
        .gap(rng.range_f64(0.0, 8.0))
        .padding(EdgeInsets::all(rng.range_f64(0.0, 10.0)))
        .grow(rng.range_f64(0.0, 2.0))
        .shrink(rng.shrink_factor())
        .margin(EdgeInsets::all(rng.range_f64(0.0, 4.0)));

    for _ in 0..count {
        let child = random_node(rng, depth + 1, max_depth);
        node = node.child(child);
    }
    node
}

fn content_box(node: &Node) -> Rect {
    let inner = node.style.inner();
    Rect::new(
        node.rect.x + inner.left,
        node.rect.y + inner.top,
        (node.rect.w - inner.horizontal()).max(0.0),
        (node.rect.h - inner.vertical()).max(0.0),
    )
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

fn child_base_main(child: &Node, dir: Direction) -> f64 {
    match child.style.main_dim(dir) {
        Dimension::Points(v) => v.max(0.0),
        Dimension::Auto => axis_main(natural_size(child), dir),
    }
}

// Whether the children overflow the parent content box along the main axis.
// The solver shrinks flexible children to fit whenever it can, collapsing each
// down to at most zero. So the smallest total main extent the children can take
// is the sum of the bases of the children that cannot shrink, plus their margins
// and the gaps. Overflow is unavoidable, and therefore documented and allowed,
// exactly when even that minimum exceeds the content main length.
fn main_axis_overflows(node: &Node, cbox: &Rect) -> bool {
    let dir = node.style.direction;
    let n = node.children.len();
    if n == 0 {
        return false;
    }
    let main_avail = axis_main(Size::new(cbox.w, cbox.h), dir);
    let total_gap = if n > 1 {
        node.style.gap * (n as f64 - 1.0)
    } else {
        0.0
    };
    let mut min_base = 0.0;
    let mut margin_sum = 0.0;
    for child in &node.children {
        if child.style.flex_shrink <= 0.0 {
            min_base += child_base_main(child, dir);
        }
        margin_sum += child.style.margin.main_total(dir);
    }
    min_base + margin_sum + total_gap > main_avail + 1e-4
}

// Whether a single child overflows the parent content box along the cross axis.
// A stretched child is clamped to the content box, so this only happens when the
// align is not stretch and the child intrinsic cross size exceeds what is left.
fn cross_axis_overflows(node: &Node, child: &Node, cbox: &Rect) -> bool {
    let dir = node.style.direction;
    let cross_avail = axis_cross(Size::new(cbox.w, cbox.h), dir);
    let margin = child.style.margin.cross_total(dir);
    let cross_size = match child.style.cross_dim(dir) {
        Dimension::Points(v) => v.max(0.0),
        Dimension::Auto => {
            if node.style.align == Align::Stretch {
                (cross_avail - margin).max(0.0)
            } else {
                axis_cross(natural_size(child), dir)
            }
        }
    };
    cross_size + margin > cross_avail + EPS
}

fn contained_on(near: f64, far: f64, lo: f64, hi: f64) -> bool {
    near >= lo - 1e-4 && far <= hi + 1e-4
}

fn check_node(node: &Node) {
    // No negative sizes anywhere.
    assert!(
        node.rect.w >= -EPS && node.rect.h >= -EPS,
        "negative size at #{}: {:?}",
        node.id,
        node.rect
    );

    let cbox = content_box(node);
    let dir = node.style.direction;
    let main_overflow = main_axis_overflows(node, &cbox);

    // Containment is asserted per axis. On an axis that is not overflowing the
    // child border box lies fully within the parent content box, which is the
    // core promise of nesting. On the main axis under documented overflow the
    // child still begins at or after the content origin and only extends past
    // the far edge (the solver never places it before the origin). On the cross
    // axis under documented overflow a non stretch align may push the child past
    // either edge, so containment is not asserted there.
    for child in &node.children {
        let r = &child.rect;
        let cross_overflow = cross_axis_overflows(node, child, &cbox);
        let (main_near, main_far, main_lo, main_hi, cross_near, cross_far, cross_lo, cross_hi) =
            match dir {
                Direction::Row => (
                    r.x,
                    r.right(),
                    cbox.x,
                    cbox.right(),
                    r.y,
                    r.bottom(),
                    cbox.y,
                    cbox.bottom(),
                ),
                Direction::Column => (
                    r.y,
                    r.bottom(),
                    cbox.y,
                    cbox.bottom(),
                    r.x,
                    r.right(),
                    cbox.x,
                    cbox.right(),
                ),
            };

        if main_overflow {
            assert!(
                main_near >= main_lo - 1e-4,
                "child #{} {:?} placed before parent #{} content origin {:?} under main overflow",
                child.id,
                r,
                node.id,
                cbox
            );
        } else {
            assert!(
                contained_on(main_near, main_far, main_lo, main_hi),
                "child #{} {:?} escapes parent #{} content {:?} on main axis",
                child.id,
                r,
                node.id,
                cbox
            );
        }

        if !cross_overflow {
            assert!(
                contained_on(cross_near, cross_far, cross_lo, cross_hi),
                "child #{} {:?} escapes parent #{} content {:?} on cross axis",
                child.id,
                r,
                node.id,
                cbox
            );
        }
    }

    // Siblings in normal flow never overlap.
    for i in 0..node.children.len() {
        for j in (i + 1)..node.children.len() {
            let a = &node.children[i].rect;
            let b = &node.children[j].rect;
            assert!(
                !a.overlaps(b, 1e-4),
                "siblings overlap in #{}: {:?} and {:?}",
                node.id,
                a,
                b
            );
        }
    }

    for child in &node.children {
        check_node(child);
    }
}

#[test]
fn invariants_hold_over_random_trees() {
    let ops = env_u64("LOOM_FUZZ_OPS", 300);
    let seed = env_u64("LOOM_FUZZ_SEED", 0xD00D);
    let mut rng = Rng::new(seed);

    for _ in 0..ops {
        let mut root = random_node(&mut rng, 0, 3);
        assign_ids(&mut root);
        let w = rng.range_f64(400.0, 1000.0);
        let h = rng.range_f64(400.0, 1000.0);
        compute_layout(&mut root, Size::new(w, h));
        check_node(&root);
    }
}

#[test]
fn layout_is_deterministic() {
    let ops = env_u64("LOOM_FUZZ_OPS", 300);
    let mut rng = Rng::new(0xBEEF);

    for _ in 0..ops {
        // Build the same tree twice from identical sub seeds.
        let sub = rng.next_u64();
        let w = rng.range_f64(400.0, 1000.0);
        let h = rng.range_f64(400.0, 1000.0);

        let mut a = random_node(&mut Rng::new(sub), 0, 3);
        let mut b = random_node(&mut Rng::new(sub), 0, 3);
        assign_ids(&mut a);
        assign_ids(&mut b);
        compute_layout(&mut a, Size::new(w, h));
        compute_layout(&mut b, Size::new(w, h));

        assert_eq!(a, b, "same tree at same size produced different layouts");
    }
}

#[test]
fn flex_children_fill_main_axis_across_sizes() {
    // Three equal growers must exactly span the content main axis at any size.
    for &size in &[100.0, 137.0, 640.0, 999.0] {
        let mut root = Node::row()
            .width(size)
            .height(20.0)
            .gap(7.0)
            .child(Node::boxed().grow(1.0))
            .child(Node::boxed().grow(1.0))
            .child(Node::boxed().grow(1.0));
        assign_ids(&mut root);
        compute_layout(&mut root, Size::new(size, 20.0));

        let first = root.children.first().unwrap().rect;
        let last = root.children.last().unwrap().rect;
        let span = last.right() - first.x;
        assert!(
            (span - size).abs() <= 1.0,
            "flex children span {span} does not fill {size} within one pixel"
        );

        // The three growers are equal within a rounding pixel.
        let w0 = root.children[0].rect.w;
        for child in &root.children {
            assert!(
                (child.rect.w - w0).abs() <= 1.0,
                "flex children not equal: {} vs {}",
                child.rect.w,
                w0
            );
        }
    }
}

#[test]
fn style_default_is_sane() {
    let s = Style::default();
    assert_eq!(s.flex_grow, 0.0);
    assert_eq!(s.direction, Direction::Row);
}
