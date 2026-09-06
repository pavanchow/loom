//! Golden layout tests. Every expected rectangle here was computed by hand from
//! the documented algorithm. They pin down row, column, nesting, flex grow
//! distribution, padding, margin, gap, and justify plus align behaviour.

use loom::prelude::*;

fn laid_out(mut root: Node, w: f64, h: f64) -> Node {
    assign_ids(&mut root);
    compute_layout(&mut root, Size::new(w, h));
    root
}

// A: row, fixed child then two equal flex children, with padding and gap.
// Content box is (10, 10, 280, 80). Free main space after the fixed 40 and two
// 20px gaps is 200, split evenly, so each flex child is 100 wide.
#[test]
fn golden_row_flex_padding_gap() {
    let root = laid_out(
        Node::row()
            .width(300.0)
            .height(100.0)
            .padding(EdgeInsets::all(10.0))
            .gap(20.0)
            .child(Node::boxed().width(40.0))
            .child(Node::boxed().grow(1.0))
            .child(Node::boxed().grow(1.0)),
        300.0,
        100.0,
    );
    assert_eq!(root.children[0].rect, Rect::new(10.0, 10.0, 40.0, 80.0));
    assert_eq!(root.children[1].rect, Rect::new(70.0, 10.0, 100.0, 80.0));
    assert_eq!(root.children[2].rect, Rect::new(190.0, 10.0, 100.0, 80.0));
}

// B: column with a fixed header, a growing body and a fixed footer. The body is
// itself a row of two fixed boxes. This checks nesting across both axes.
#[test]
fn golden_column_nesting_with_flex_body() {
    let body = Node::row()
        .grow(1.0)
        .child(Node::boxed().width(60.0).height(50.0))
        .child(Node::boxed().width(60.0).height(50.0));

    let root = laid_out(
        Node::column()
            .width(200.0)
            .height(200.0)
            .child(Node::boxed().height(30.0))
            .child(body)
            .child(Node::boxed().height(20.0)),
        200.0,
        200.0,
    );

    // Header, body and footer stack vertically. Body grows to 150.
    assert_eq!(root.children[0].rect, Rect::new(0.0, 0.0, 200.0, 30.0));
    assert_eq!(root.children[1].rect, Rect::new(0.0, 30.0, 200.0, 150.0));
    assert_eq!(root.children[2].rect, Rect::new(0.0, 180.0, 200.0, 20.0));

    // Body children keep their fixed sizes and sit at the top left of the body.
    let body = &root.children[1];
    assert_eq!(body.children[0].rect, Rect::new(0.0, 30.0, 60.0, 50.0));
    assert_eq!(body.children[1].rect, Rect::new(60.0, 30.0, 60.0, 50.0));
}

// C: justify space between spreads two fixed children to the far edges.
#[test]
fn golden_justify_space_between() {
    let root = laid_out(
        Node::row()
            .width(100.0)
            .height(40.0)
            .justify(Justify::SpaceBetween)
            .child(Node::boxed().width(20.0).height(40.0))
            .child(Node::boxed().width(20.0).height(40.0)),
        100.0,
        40.0,
    );
    assert_eq!(root.children[0].rect, Rect::new(0.0, 0.0, 20.0, 40.0));
    assert_eq!(root.children[1].rect, Rect::new(80.0, 0.0, 20.0, 40.0));
}

// D: a margin plus cross axis centering. The 20x20 child has 5px margin on all
// sides and is centered vertically in the 100px tall row.
#[test]
fn golden_margin_and_align_center() {
    let root = laid_out(
        Node::row()
            .width(100.0)
            .height(100.0)
            .align(Align::Center)
            .child(Node::boxed().width(20.0).height(20.0).margin(EdgeInsets::all(5.0))),
        100.0,
        100.0,
    );
    assert_eq!(root.children[0].rect, Rect::new(5.0, 40.0, 20.0, 20.0));
}

// E: border plus padding form the box model. Content sits inside 15px of inset
// on every edge, so the growing child fills a 70x70 area.
#[test]
fn golden_box_model_border_padding() {
    let root = laid_out(
        Node::column()
            .width(100.0)
            .height(100.0)
            .border(EdgeInsets::all(5.0))
            .padding(EdgeInsets::all(10.0))
            .child(Node::boxed().grow(1.0)),
        100.0,
        100.0,
    );
    assert_eq!(root.children[0].rect, Rect::new(15.0, 15.0, 70.0, 70.0));
}

// F: justify end pushes a single fixed child to the far edge.
#[test]
fn golden_justify_end() {
    let root = laid_out(
        Node::row()
            .width(100.0)
            .height(10.0)
            .justify(Justify::End)
            .child(Node::boxed().width(30.0).height(10.0)),
        100.0,
        10.0,
    );
    assert_eq!(root.children[0].rect, Rect::new(70.0, 0.0, 30.0, 10.0));
}

// G: weighted flex grow. Free space splits 1:3 between the two growers after a
// fixed 20px child, so they receive 20 and 60 of the remaining 80.
#[test]
fn golden_weighted_flex_grow() {
    let root = laid_out(
        Node::row()
            .width(100.0)
            .height(10.0)
            .child(Node::boxed().width(20.0))
            .child(Node::boxed().grow(1.0))
            .child(Node::boxed().grow(3.0)),
        100.0,
        10.0,
    );
    assert_eq!(root.children[0].rect, Rect::new(0.0, 0.0, 20.0, 10.0));
    assert_eq!(root.children[1].rect, Rect::new(20.0, 0.0, 20.0, 10.0));
    assert_eq!(root.children[2].rect, Rect::new(40.0, 0.0, 60.0, 10.0));
}

// I: max_width caps a grower. The child would grow to fill the 100px row but its
// maximum width of 30 clamps the used size, so it stays 30 wide at the origin.
#[test]
fn golden_max_width_caps_grower() {
    let root = laid_out(
        Node::row()
            .width(100.0)
            .height(20.0)
            .child(Node::boxed().grow(1.0).max_width(30.0)),
        100.0,
        20.0,
    );
    assert_eq!(root.children[0].rect, Rect::new(0.0, 0.0, 30.0, 20.0));
}

// J: min_width floors a shrinking child. A 100px child under shrink would shrink
// to fit the 50px row, but its 70px minimum floors it, so it overflows to 70.
#[test]
fn golden_min_width_floors_shrinker() {
    let root = laid_out(
        Node::row()
            .width(50.0)
            .height(20.0)
            .child(Node::boxed().width(100.0).shrink(1.0).min_width(70.0)),
        50.0,
        20.0,
    );
    assert_eq!(root.children[0].rect, Rect::new(0.0, 0.0, 70.0, 20.0));
}

// K: max_height caps a cross axis stretch. The auto height child would stretch to
// the 40px row, but its 10px maximum height clamps the cross size.
#[test]
fn golden_max_height_caps_cross_stretch() {
    let root = laid_out(
        Node::row()
            .width(100.0)
            .height(40.0)
            .child(Node::boxed().width(20.0).max_height(10.0)),
        100.0,
        40.0,
    );
    assert_eq!(root.children[0].rect, Rect::new(0.0, 0.0, 20.0, 10.0));
}

// H: text intrinsic width drives an auto sized leaf. "OK" is two characters at
// 8px each, so it is 16px wide, centered in a 100px row.
#[test]
fn golden_text_intrinsic_width() {
    let root = laid_out(
        Node::row()
            .width(100.0)
            .height(16.0)
            .justify(Justify::Center)
            .child(Node::text("OK")),
        100.0,
        16.0,
    );
    // 16px wide, so left edge at (100 - 16) / 2 = 42.
    assert_eq!(root.children[0].rect, Rect::new(42.0, 0.0, 16.0, 16.0));
}
