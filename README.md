# Loom

Loom is a dependency free, retained mode GUI layout engine written in pure Rust. It takes a tree of widgets and a container size and computes the exact pixel rectangle of every node using a flexbox style constraint solver, with optional wrapping so children break onto new lines when a line fills up. It has no external crates, targets edition 2021, and the whole core is headless so it can be tested without a screen.

Live playground: https://pavanchow.github.io/loom/

## The gap it fills

Most layout code is welded to a rendering stack. To compute where a button lands you often need a windowing crate, a font backend, and a GPU surface. That is a lot of surface area when all you want is the geometry.

Loom separates the math from the pixels. The engine answers one question well. Given this tree and this available space, where does every box go. A `Renderer` trait is the only seam to the outside, so the same layout can drive a terminal, an SVG file, a canvas, or a real GPU backend.

Why a person would use it. You get a small, readable, fully tested layout core with zero supply chain risk and no build surprises. You can drop it into a CLI, a game, an embedded target, or a test harness and it just computes rectangles.

Why an AI agent would use it. An agent that generates or reasons about user interfaces needs a way to turn a described layout into concrete coordinates it can verify. Loom is deterministic, headless, and self checking. An agent can build a tree, run layout, and assert on the resulting rectangles without spinning up a display server. The playground ships the same algorithm in JavaScript so a browser based agent can reason about identical results.

## Quickstart

```rust
use loom::prelude::*;

let mut root = Node::row()
    .width(200.0)
    .height(40.0)
    .gap(8.0)
    .wrap()
    .child(Node::text("File").width(48.0))
    .child(Node::spacer().grow(1.0))
    .child(Node::button("Save").width(64.0));

assign_ids(&mut root);
compute_layout(&mut root, Size::new(200.0, 40.0));

// The spacer absorbs all leftover space on the main axis.
assert_eq!(root.children[1].rect.w, 200.0 - 48.0 - 64.0 - 8.0 * 2.0);
```

Run the command line tool to see a full sample laid out.

```
cargo run -- 800 600      # print the computed rectangle tree at 800 x 600
cargo run -- demo         # also print draw calls and a hit test probe
```

## API

Build a tree with `Node`. Every widget is one of `Container`, `Text`, `Button`, `Box`, or `Spacer`. Containers are made with `Node::row()` or `Node::column()` and configured with a fluent builder.

- Sizing: `.width(px)`, `.height(px)`, or leave a dimension automatic so it is derived from content.
- Clamps: `.min_width(px)`, `.max_width(px)`, `.min_height(px)`, `.max_height(px)` bound the resolved size on each axis. An unset bound means no limit, and when a minimum exceeds a maximum the minimum wins.
- Flex: `.grow(f)` distributes leftover main axis space, `.shrink(f)` absorbs overflow.
- Spacing: `.gap(px)`, `.padding(insets)`, `.border(insets)`, `.margin(insets)`.
- Alignment: `.justify(Justify::...)` on the main axis, `.align(Align::...)` on the cross axis.
- Composition: `.child(node)` or `.children(vec)`.

Then run the pipeline.

- `assign_ids(&mut root)` numbers nodes in preorder for hit testing and rendering.
- `compute_layout(&mut root, Size::new(w, h))` fills in every `node.rect`.
- `render(&root, &mut backend)` walks the tree and emits draw calls to any `Renderer`.
- `hit_test(&root, x, y)` returns the id of the topmost node under a point, and `hit_path` returns the full root to leaf path.

The box model. A `rect` is the border box, which includes border and padding but excludes margin. Fixed `width` and `height` set the border box size. Content lives inside border plus padding.

Non-finite input rejection. Any NaN or infinity handed to a style value or to the available size is normalized before the solver runs, so a bad number can never poison a computed rectangle. A non-finite scalar such as a gap or a margin becomes zero, and a non-finite fixed length degrades to automatic so the node falls back to content or fill sizing. Every coordinate the engine produces is finite.

## The correctness gate

The claim that this engine is correct is backed by tests that run on every build.

1. Golden layout. Several known trees laid out at known sizes must produce exact, hand computed rectangles. These cover row, column, nesting, flex grow distribution, weighted grow, padding, margin, gap, justify plus align, wrapping, and the min and max size clamps. See `tests/golden.rs`.
2. Invariants over random trees, including wrap-on trees whose leaves always fit their lines and clamp trees with random min and max bounds. The random trees deliberately feed degenerate, huge, fractional, and non-finite sizes so both axes are driven into extremes. For every generated tree, no coordinate may be non-finite, every child rectangle must lie inside its parent content box on any axis that fits, sibling rectangles must never overlap, no width or height may be negative, equal flex children must fill the parent main axis within one pixel of rounding, and every clamped child must land inside its own bounds. See `tests/invariants.rs`.
3. Determinism. The same tree at the same size must produce identical rectangles on every run. See `tests/invariants.rs`.
4. The checker checks itself. A unit test corrupts a good layout in four ways, a child that escapes the content box, a non-finite coordinate, and overlapping siblings, and asserts the invariant checker rejects each one, so a passing gate cannot be a blind gate. See `tests/invariants.rs`.

Plus unit tests per module for the box model math, flex distribution, and hit testing. The fuzz iteration count is bounded for CI and controlled by `LOOM_FUZZ_OPS`, with the seed set by `LOOM_FUZZ_SEED` given as a decimal number.

```
cargo test
cargo clippy --all-targets -- -D warnings
LOOM_FUZZ_OPS=5000 cargo test invariants
```

## Design

See `DESIGN.md` for the architecture, the box model, the flex constraint algorithm step by step, hit testing, and why each gate proves what it claims.
