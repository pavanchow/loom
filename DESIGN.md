# Loom design

This document explains how Loom is built, the box model it uses, the layout algorithm in full, how hit testing works, and why each correctness gate proves the claim it makes. The prose avoids em dashes and semicolons on purpose so it stays easy to read aloud.

## Architecture

Loom is a small set of modules with a clear one way flow of data.

- `geometry` holds the value types. `Size`, `Point`, and `Rect`. `Rect` also carries the containment and overlap helpers the gates rely on.
- `style` holds the resolved style of a node. Direction, justify, align, gap, the fixed or automatic dimensions, flex grow and shrink, and the three edge insets that make up the box model. It also maps the four physical edges onto main and cross axes for a given direction, which is what lets one algorithm serve both rows and columns.
- `widget` holds the retained tree. A `Node` owns its kind, its style, its children, and after layout its computed rectangle. A fluent builder makes trees readable. `assign_ids` numbers the tree in preorder.
- `layout` holds the solver. It has two passes. A pure measurement pass called `natural_size` and an arranging pass called through `compute_layout`.
- `render` holds the `Renderer` trait and a `RecordingRenderer` that captures draw calls so the output is testable without a screen.
- `event` holds hit testing.

The tree is retained, which means it persists between frames and layout mutates the rectangles in place. Nothing in the core imports a graphics library. The only way out is the `Renderer` trait, so the same computed layout can drive any backend.

## The box model

Every node has three nested boxes described by edge insets. From the outside in they are margin, border, and padding. The rectangle Loom computes and stores in `node.rect` is the border box. The border box includes the border and the padding but excludes the margin.

A fixed `width` or `height` sets the size of the border box. When a dimension is automatic the border box is the content size plus that node border and padding. Content therefore always lives inside the sum of border and padding, which the style module exposes as `inner`.

Margins live outside the border box. They push a node away from its siblings and from the parent content edges, and they are counted as consumed space on the main axis and as offset on the cross axis, but they are never part of the stored rectangle. Keeping margin out of the rectangle is what makes the containment invariant clean. A child border box must sit inside the parent content box even when the child has margin.

## Axes

A row lays children along the horizontal main axis and aligns them on the vertical cross axis. A column swaps these. Rather than write the algorithm twice, the style module projects the physical edges onto main and cross for the active direction. For a row the main start edge is the left and the cross start edge is the top. For a column the main start edge is the top and the cross start edge is the left. The solver is written entirely in main and cross terms and reads exactly the same for both directions.

## The measurement pass

`natural_size` returns the border box a node would take with no external constraint. For a leaf the content size comes from `leaf_content_size`. Text and buttons measure their label as the character count times a fixed character width, by a fixed line height. Boxes and spacers have no intrinsic content. For a container the content size is derived from its children. Along the main axis it is the sum of child natural main sizes, plus each child main margin, plus the gaps between children. Along the cross axis it is the largest child natural cross size including that child cross margins. Fixed dimensions override the derived values.

This pass is pure. It reads the tree and returns sizes and never writes. That purity is what makes layout deterministic and easy to reason about.

## The arranging pass

`compute_layout` sets the root border box. The root takes its fixed size if it has one, otherwise it fills the available size passed in. Then `layout_node` runs. It records the assigned rectangle, clamped so width and height are never negative, and if the node is a container with children it arranges them.

`arrange_children` is the heart of the engine. For a container it does the following.

1. Compute the content box by subtracting the inner insets from the border box. Derive the main available length and the cross available length from the content box for the active direction.
2. Measure each child. The base main size is the child fixed main size if present, otherwise its natural main size. Record each child main margins, grow factor, and shrink factor.
3. Compute the free space. Free equals the main available length minus the sum of base sizes, minus the sum of main margins, minus the total gap between children.
4. Resolve flexible sizing. If there is positive free space and at least one child wants to grow, hand each grower a share of the free space in proportion to its grow factor, added on top of its base size. If instead free space is negative, shrink each child in proportion to its shrink factor times its base size, so the row or column collapses to fit rather than overflowing. Shrinking is resolved iteratively. A child that would collapse below zero is clamped to zero and frozen, and the remaining deficit is redistributed among the children that can still shrink. This makes the children fit the available main length exactly whenever that is geometrically possible, that is whenever the children that cannot shrink, together with the margins and gaps, still fit. When even the non shrinkable children do not fit, the content overflows the main axis. Overflow is visible and is never clamped away.
5. Place along the main axis. When nothing grew, any leftover free space is distributed by the justify rule. Start puts the leftover after the children. Center splits it before and after. End puts it before. Space between spreads it into the gaps between children. A running cursor walks the main axis, adding each child leading margin, placing the child, then advancing by the child main size, its trailing margin, the gap, and any justify spacing.
6. Place along the cross axis. A child cross size is its fixed cross size if present. Otherwise, when the container aligns children by stretch, the child fills the cross available length minus its cross margins. Otherwise the child keeps its natural cross size. The child is then positioned by the align rule, which is start, center, end, or stretch, inside the leftover cross space.
7. Recurse. Each child is laid out into the rectangle just computed, which becomes its own border box, and the same routine runs for its children.

Because step four guarantees the children fit the main axis whenever any child can shrink, and step six never gives a child more cross size than the content box, the arranged children stay inside the parent content box.

## Size clamping

Each node carries an optional minimum and maximum on both width and height. The clamp is applied wherever a size is resolved. The base main size is clamped before free space is computed, so growth and shrink start from a bounded value. The final main size is clamped again after grow or shrink, because growth can push a child past its maximum and shrink can pull it below its minimum. The cross size is clamped after stretch or content resolution. A node also clamps its own natural size, so a container that measures its children honours their bounds. An unset bound means no minimum, taken as zero, and no maximum, taken as infinity. When a minimum exceeds a maximum the minimum wins, which matches the resolution order used by the web box model.

Clamping interacts cleanly with flex. When no clamp is active a line of growers still fills the main axis exactly, so the flex fill gate is untouched. When a clamp is active it can deliberately break that fill, for example a grower pinned to a maximum leaves space unfilled and a shrinker pinned to a minimum overflows. Both are the documented behaviour of a clamp and the gate accounts for them.

## Non-finite input rejection

A layout is only meaningful for finite numbers, so a NaN or an infinity is treated as invalid input rather than propagated. Before the solver runs, `compute_layout` walks the tree once and normalizes every style value. A non-finite scalar such as a gap, a grow factor, or a margin edge becomes zero. A non-finite fixed length degrades to automatic, so the node falls back to content or fill sizing instead of carrying the bad number into every rectangle it touches. The available size passed in is normalized the same way. This single pass is the one choke point, which means the guarantee holds no matter how a style was constructed, whether through the builder or by assembling a struct directly. The invariant gate then asserts that every computed coordinate is finite, so a regression that let a bad number through would fail immediately.

## Wrapping

A container with `FlexWrap::Wrap` lays its children out in lines instead of one run. Lines break greedily in child order: a line stays open while the running main sizes plus margins plus gaps fit the content main axis, and a child that alone exceeds the main axis gets a line of its own, overflowing that line per the overflow rules above. Each line then resolves exactly like a NoWrap container restricted to its members: grow distributes the line's free main space, the iterative shrink pass collapses the line's members to fit, and justify distributes whatever is left within the line, so a lone item on a line sits at the line start under space between. A line's cross extent is the largest member cross footprint, stretch children fill their line rather than the container, and lines stack from the content origin along the cross axis, which means a wrapping column grows new columns sideways. The gate covers wrap with exact goldens for line breaking, per line justify, stretch to the line, the column case, and an oversized child, plus a fuzz test asserting main axis containment, cross axis starts, non negative sizes, and recomputation determinism for random wrap trees whose leaves always fit.

## Hit testing

Hit testing takes a point and returns the topmost node under it. Children are painted after their parent and later siblings are painted after earlier ones, so the visually topmost node is the deepest last matching node. `hit_test` checks that the point is inside the node border box, then searches the children in reverse order so a later sibling wins over an earlier one, and returns the deepest match. `hit_path` returns the whole chain of ids from the root down to that deepest node, which is what an event system needs for bubbling. Rectangle containment treats the left and top edges as inside and the right and bottom edges as outside, so two touching rectangles never both claim the shared edge and a point lands in exactly one leaf.

## Why each gate proves its claim

The correctness of a layout engine is a claim about geometry, so the gates are stated as geometry.

The golden tests prove that specific, human checkable cases are exactly right. Each expected rectangle was computed by hand from the algorithm above, so if the code drifts from the documented behavior a golden test fails with a precise mismatch. They cover row and column, nesting across both axes, even and weighted flex grow, padding, margin, gap, the justify variants, cross axis centering, and text intrinsic sizing. Exactness here means these are not tolerance checks. The numbers must match to the pixel.

The invariant tests prove that the properties which must hold for any tree do hold across a wide random sample. Containment proves the engine never places a child outside its parent content box, which is the core promise of nesting. Non overlap proves that siblings in normal flow tile their parent rather than colliding. Non negative sizes prove the clamping holds so a renderer never sees an inverted rectangle. The flex fill check proves that equal growers span the parent main axis within one pixel of rounding, which is the core promise of flex. The random trees deliberately use varied shrink factors, including zero, and varied align values, so both axes are regularly driven into intentional overflow. Containment is therefore asserted per axis and gated on overflow: on an axis that fits, the child lies fully inside the content box. Under unavoidable main axis overflow the child still begins at or after the content origin and only spills past the far edge. Under cross axis overflow from a non stretch align the child may spill past either edge. This exercises the documented overflow behaviour instead of assuming everything always fits.

Two further invariants harden the same sample. The finite check proves that no computed coordinate is ever a NaN or an infinity, which is what makes the non-finite rejection pass trustworthy rather than merely present. The clamp check runs its own random trees with min and max bounds on both axes and proves every child size lands inside its own bounds, so the clamp feature is held to the same geometric standard as the rest of the solver. The random generator also feeds degenerate, huge, fractional, and non-finite sizes into the deep trees, so the structural invariants are proven under extremes and not only on tidy inputs.

A gate is only as good as its willingness to fail, so the checker is tested against itself. A separate unit test lays out a good tree, then corrupts it in turn by pushing a child outside the content box, by writing a non-finite coordinate, and by overlapping two siblings, and asserts the checker rejects each corruption. This rules out the worst failure mode of a property test, a checker that passes everything because it actually asserts nothing.

The determinism test proves that layout is a pure function of tree and size. It builds the same tree twice and lays both out at the same size and asserts the results are identical. Combined with the pure measurement pass, this rules out any hidden state or ordering effect and means a rectangle computed once can be trusted on every later run.

Together the three gates cover the two ways a layout engine can be wrong. The golden tests catch a wrong formula on known inputs. The invariants catch a formula that is right on the easy cases but breaks a structural rule on some unusual tree. Determinism catches nondeterminism that would make either of the first two flaky. The fuzz sample size is bounded for continuous integration and can be raised with `LOOM_FUZZ_OPS` for a deeper local run.
