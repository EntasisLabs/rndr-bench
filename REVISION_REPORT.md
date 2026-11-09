# Revision status — incomplete

This revision does **not** satisfy the requested complete Liquid/canvas acceptance brief.

## Implemented

- Double-clicking text now places the caret at the pointer rather than the end.
- Clicking active text places the caret; Shift-click extends its selection; primary-pointer dragging selects text.
- Hit-testing uses inverse scene/node transforms and the existing wrapped canvas text layout. Native `EngineMeasure` delegates line picking to `TextEngine::layout(...).index_at(...)` rather than estimating glyph positions.
- Text selection remains transient; replacement uses the existing core string interpreter, and finishing still publishes one canvas transaction. Programmatic `edit_text` retains its end-of-text initial caret.
- Existing append and reset APIs are unchanged.

## Executable evidence

- Updated `typing_creates_text_and_double_click_edits_it` asserts the pointer-selected byte position and insertion at the beginning rather than the previous end-of-text behavior.
- Added `pointer_selection_tracks_rotated_text_through_camera_and_commits_once`: rotated text, 2× camera, middle caret, drag selection, no document revision on selection, replacement, commit, undo and redo.
- Added `crates/rndr_canvas/tests/text_hit.rs`: UTF-8 versus UTF-16 offsets, CRLF/CR mapping, wrapped-line starts.

## Checks

All commands below passed locally:

- `cargo test -p rndr_canvas -p rndr_canvas_ui -p rndr_liquid -p rndr_liquid_ui`: 78 passed, zero failed/ignored, including doctests.
- `cargo test -p rndr_core -p rndr_ui -p rndr_paint -p rndr_render`: 1,276 passed, six ignored, zero failed, including doctests. Ignored probes were not run.
- `cargo test -p source-session`: 20 passed.
- `cargo test -p rndr --no-default-features --test editing_compatibility`: two passed.
- `cargo test -p rndr --no-default-features --features ui --test editing_compatibility`: three passed.
- `cargo check --workspace`.
- `cargo check -p rndr_core -p rndr_ui -p rndr_paint --no-default-features`.
- `cargo clippy -p rndr_canvas -p rndr_canvas_ui -p rndr_liquid -p rndr_liquid_ui --lib --bins -- -D warnings`.
- `cargo clippy -p rndr_core -p rndr_ui -p rndr_paint -p rndr_render -p source-session --lib --bins -- -D warnings`.
- `RUSTDOCFLAGS='-D warnings' cargo doc -p rndr_core -p rndr_ui --no-deps`.
- `cargo fmt --all --check`; `git diff --check`.

Repository-wide checks ran before the final canvas-only newline mapping tests and cleanup; affected package tests and Clippy were rerun afterward.

## Unfinished / unverified

- Stable-ID Liquid reconciliation, anonymous identity and duplicate-ID policies, reorder/removal/update/type replacement, controlled/uncontrolled values, incomplete-stream admission and reconciliation-specific cleanup are **not implemented**. Their acceptance scenarios have not been converted into tests.
- Checkbox/input/slider lifecycle fixture and the requested integrated host transition sequence are **not implemented**. The existing fixture remains unchanged.
- Visible caret/selection editing chrome, clipboard, full focus/shortcut routing and composition integration remain **unimplemented**.
- New pointer tests use the headless estimate measure. The native shaping adapter compiles, but this revision has no new native interaction or pixel evidence. There is no native IME/clipboard/assistive-technology claim.
- Styled letter-spacing picking, grapheme/bidi/ligature pointer selection, nested transformed frames and off-canvas capture need additional targeted acceptance evidence. The default measure's fallback picks scalar boundaries; it is not a native shaping substitute.
- No cross-platform CI matrix or performance probes were run.
