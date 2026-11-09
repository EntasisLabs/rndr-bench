# Fixture validation record

Environment: assigned `medousa` checkout; setup sourced from
`~/benchmarks/rndr-liquid-canvas-20261108/enter.sh medousa`.
`BENCH_TASK` was readable in full;
`CARGO_TARGET_DIR=/srv/benchmarks/rndr-liquid-canvas-20261108/build/medousa`.

## Passed for the integrated example

- `cargo test -p liquid_canvas_integration`: **5 passed**.
  - Both slices run in one application/world.
  - Uncontrolled draft value, entity identity and focus survive Markdown insertion,
    host data updates, reorder and partial/complete stream submissions.
  - Controlled slider host value applies; removal, type change and replacement
    reset the expected identities/state.
  - Native-adapter-shaped text input reaches canvas editing in the combined world;
    a Liquid update does not discard the active draft; commit → object manipulation
    → undo twice → redo twice reconstructs exact node states.
  - Canvas manipulation history survives Liquid updates independently.
  - Canvas slot is in bounds and larger than 400 × 300 logical points. Timed host
    simulation uses core's native-waking timer rather than relying on mouse motion.
- `cargo clippy -p liquid_canvas_integration --all-targets --no-deps -- -D warnings`:
  **passed**, with no warning suppressions added.
- `cargo fmt -p liquid_canvas_integration -- --check`: **passed**.
- `cargo run -p liquid_canvas_integration -- --headless`: **passed**; all eight host
  stages exercised; incomplete stage reports `Ok(Deferred)` and completion reports
  `Ok(Applied)`.
- `cargo run -p liquid_canvas_integration -- --export /tmp/medousa-liquid-canvas.png`:
  **passed**; real offscreen renderer wrote a 1500 × 960 PNG. The image is an
  ephemeral `/tmp` artifact, not an added source file.

## Limits and integration checks still owned by the parent

- No active `DISPLAY` or `WAYLAND_DISPLAY` was available, and no `xvfb-run`,
  `xdotool` or OCR tool was installed. No native window interaction, clipboard,
  actual IME, accessibility, or human visual inspection is claimed.
- The PNG was generated successfully; image appearance was not visually inspected.
  The layout regression is numeric evidence, not a substitute for visual review.
- An earlier dependency-inclusive strict Clippy attempt failed in concurrently
  edited `crates/rndr_canvas_ui/src/view.rs` (`if_same_then_else`). This worker did
  not edit that peer-owned file. The parent must run combined strict verification
  on the final integrated source; the fixture-only `--no-deps` gate is green.
- The first requested baseline of `canvas-editor` and `liquid-view` intersected
  concurrent canvas implementation and failed on temporarily absent `text_edit`
  methods/modules. The final integrated fixture builds and its tests pass.
- Git read commands were rejected by the runtime's shared hazardous-resource claim
  guard, despite being read-only. No Git mutations, commits, staging or reference
  changes were attempted. Final combined diff/status review remains with the host.

## Actual review a22dfd0: duplicate IDs and legibility follow-up

The preceding entries are historical; this section records the follow-up run.
This worker changed only `rndr_liquid_ui` and this fixture, preserving previous
working-tree changes. `BENCHMARK_REPORT.md` remains owned by the integrating host.

- `cargo test -p rndr_liquid_ui --test reconcile duplicate_initial_and_reset_ids -- --nocapture`:
  first reproduced an identity mismatch (the later duplicate survived instead of
  the first); after occurrence-qualified keys, construction and intentional reset
  both retain only the first field on a unique-ID update, retire the duplicate,
  clear its focus, and retire all fields on removal. Incremental duplicates still
  refuse atomically. Public identity remains first-occurrence-wins.
- `cargo test -p liquid_canvas_integration fixture_has_explicit_readable_background -- --nocapture`:
  first failed because the root had no explicit fill, exposing the renderer's
  dark clear behind light-theme text. The root now uses semantic `Canvas` fill;
  inputs/slider use semantic `Content`/`Text`, content-metric wrapping labels,
  visible theme-derived focus borders, and an ASCII slider track rather than
  optional box-drawing/symbol glyphs.
- `cargo test -p rndr_liquid_ui -p liquid_canvas_integration`: **26 passed** on
  the final observed tree (7 fixture, 18 Liquid integration including the host's
  concurrent idle-reset review regression, 1 doctest). No failures/ignores.
  Bundled Parley-font tests measure header/status/control text fit and assert
  light/dark input/slider contrast >= 4.5:1 and focused borders >= 3:1.
- `cargo clippy -p rndr_liquid_ui -p liquid_canvas_integration --all-targets --no-deps -- -D warnings`:
  **passed**. Scoped formatting and `git diff --check` also passed.
- `cargo run -p liquid_canvas_integration -- --export /tmp/medousa-review-initial.png`
  and matching exports `/tmp/medousa-review-input-focus.png --export-focus draft`
  and `/tmp/medousa-review-slider-focus.png --export-focus volume`: **passed**,
  three real-renderer 1500 x 960 offscreen PNGs. Ephemeral artifacts stay outside
  the source tree. These commands reuse the bundled native text engine.
- Numeric PNG inspection using Python standard-library PNG decoding confirmed
  corner/root RGB `(244,245,247)` and 413,013 semantic-canvas pixels in all three
  exports. Focus RGB `(0,95,204)` pixel counts were 0 / 2,040 / 2,000; changed
  bounds were input `(28,422)-(496,458)` and slider `(28,490)-(496,526)`,
  respectively. These are offscreen pixel/layout checks, **not human visual review**.
- `timeout 8s target/debug/liquid_canvas_integration`: **blocked**, exit 101:
  event loop `NotSupported`, neither WAYLAND_DISPLAY, WAYLAND_SOCKET nor DISPLAY
  is set. No `xvfb-run` or local-image visual inspection capability was available.
  A real desktop human review of appearance/focus/slider remains needed; no live
  language-IME, native clipboard or accessibility certification is claimed.
