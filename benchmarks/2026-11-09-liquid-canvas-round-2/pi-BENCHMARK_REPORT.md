# Benchmark delivery — partial implementation

## Repository and scope

- Root: `/home/theelevators/benchmarks/rndr-liquid-canvas-20261108/worktrees/pi`
- Branch: `benchmark/pi`; starting commit: `b2a1c214cb9856a73efca25000a0fa5bf51c1b25`.
- Starting working tree was clean. Inherited build output remained
  `/srv/benchmarks/rndr-liquid-canvas-20261108/build/pi`.
- Read the complete inherited BENCH_TASK, repository README, CONTRIBUTING,
  Liquid rendering/Markdown slot ownership and canvas editing paths.
- Confirmed both reported gaps against source. No pre-existing focused test
  failures were found.

**The full benchmark is incomplete. These changes are not production-ready
keyed reconciliation or a complete interactive canvas text editor.**

## Implemented contracts

1. `1ea9605`: conservative `append_liquid_source` snapshot admission. Existing
   parsed blocks and Markdown must be exact prefixes; refusal is atomic.
   Existing reader state is uncontrolled. `set_liquid_source` remains an
   intentional reset. Regression checks retained accordion/control entities,
   state after interaction, refusal and reset. No existing API was removed.
2. `9838a90`: canvas text now reuses `rndr_core::edit_string` for grapheme-aware
   Left/Right, Shift selection, insertion/replacement, Backspace and Delete;
   primary+A selects all. `text_selection()` returns directed UTF-8 byte offsets.
   Drafts still commit through the existing canvas transaction history.
   Regression covers an emoji cluster, middle replacement, deletion and
   commit undo/redo.
3. `28ca4a0`: one runnable integration fixture with host append/reset buttons,
   interactive Liquid accordion, canvas tools, text object and Undo/Redo.
   Added headless shared-application activation test and an end-of-text Delete
   no-op fix. Liquid is outside the inspector's subtree rebuild authority.

## Executable verification

All listed final checks passed:

```sh
# Baseline, before edits:
cargo test -p rndr_liquid_ui -p rndr_canvas_ui

# Combined final regression suite:
cargo test -p rndr_liquid -p rndr_liquid_ui -p rndr_canvas -p rndr_canvas_ui -p liquid_canvas_integration
# Final targeted rerun after adding the Delete-at-end assertion: 9 passed
cargo test -p rndr_canvas_ui --test editor

cargo check -p liquid_canvas_integration
cargo clippy -p rndr_liquid_ui -p rndr_canvas_ui --lib -- -D warnings
cargo fmt --all --check
git diff --check

cargo run -p liquid_canvas_integration -- --export /tmp/pi-liquid-canvas.png
```

During development, new tests initially failed because their Liquid fixture used
unsupported steps syntax, and later because an accordion button label includes
`+ `. Corrected the test inputs; no existing assertions or safeguards were
weakened. Image inspection caught the inspector rebuilding away the Liquid
panel; separated their subtree ownership and rerendered successfully.

### Evidence boundaries

- Automated: headless control activation, entity retention, transaction/history
  regression, existing canvas geometry/view/input and Liquid parser/render suites.
- Rendered image: exported 1280x800 PNG and inspected it. Both host buttons,
  Liquid accordion, canvas text/object selection and toolbar are visible.
  PNG stayed outside the repository; no generated build artifacts committed.
- Native interaction: **not performed**. DISPLAY and WAYLAND_DISPLAY are unset.
  A suitable desktop session is needed for native focus, clipboard, accessibility
  and IME/composition verification. No such claims are made.
- Not run: full workspace/CI, cross-platform matrix, broad core/UI regression
  gates, no-default-feature matrix, performance measurements or rustdoc gates.

## Run the fixture

```sh
cargo run -p liquid_canvas_integration
```

Open the accordion, append several host chunks, then intentionally reset.
Double-click the canvas title, move with Left/Right, select with Shift, replace
text, and commit with Escape or primary+Enter. Move an object and use Undo/Redo.
See `examples/liquid_canvas_integration/README.md` for contracts and controls.

## Incomplete requirements and risks

- Liquid: no explicit-ID reconciliation, duplicate-ID policy, inferred identity,
  insertion/reorder/removal/type-change reconciliation, controlled host value
  updates, checkbox/input/slider acceptance fixture, subscription retirement
  tests or focused-input retention tests. Prefix refusal preserves the last UI,
  but arbitrary incomplete syntax is not automatically withheld. Initial values
  cannot be updated through append; reset is required.
- Canvas: no pointer caret positioning/drag selection, caret/selection drawing,
  vertical navigation, clipboard integration, native text focus routing or IME
  integration. Existing rendering transforms remain; transformed caret hit
  testing is not implemented. No text-local undo while editing. Existing object
  manipulation tests and new text transaction tests pass separately; the full
  requested editing/manipulation/clipboard acceptance sequence is not verified.
- Example uses an accordion instead of the required checkbox/input/slider.
  Streaming is button-driven suffix snapshots, not arbitrary insertion streaming.
  Invisible canvas selection/caret makes keyboard editing less discoverable.
- Remaining work is implementation, not an external dependency blocker. Only
  native verification requires an unavailable desktop session.

## Delivery status

Implementation commits are listed above; this report is committed separately.
Final commit references and final `git status` are also returned in the delivery
message. Intended changes are committed; no pre-existing work was discarded.
Commit identity was absent; used command-local `git -c` identity, without changing
shared or repository Git configuration.
