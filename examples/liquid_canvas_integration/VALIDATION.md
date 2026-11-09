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
