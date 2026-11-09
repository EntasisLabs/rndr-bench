# Production-integration benchmark report

## Workspace and baseline

- Repository: `/home/theelevators/benchmarks/rndr-liquid-canvas-20261108/worktrees/medousa`
- Branch: `benchmark/medousa`
- Starting commit: `b2a1c214cb9856a73efca25000a0fa5bf51c1b25`; starting Git status: clean.
- Sourced `~/benchmarks/rndr-liquid-canvas-20261108/enter.sh medousa` without modifying it.
- Read the complete brief at `$BENCH_TASK` (`.../rndr-liquid-canvas-20261108/TASK.md`).
- Build output used the assigned inherited directory throughout:
  `/srv/benchmarks/rndr-liquid-canvas-20261108/build/medousa`.
- Inspected repository/architecture instructions and verified both gaps: source
  replacement detached/recreated Liquid blocks; canvas editing only appended or
  removed trailing characters. The pre-change baseline command
  `cargo test -p rndr_liquid -p rndr_liquid_ui -p rndr_canvas_ui` passed.

## Delivered implementation

### Slice 1 — Liquid (`crates/rndr_liquid`, `crates/rndr_liquid_ui`)

Added typed checkbox/input/slider fences and `parse_with_metadata`, preserving
`LiquidDocument`'s existing fields and legacy feed/styled-block domain IDs.
`update_liquid_source` accepts whole snapshots, rejects duplicate IDs atomically,
and defers incomplete Liquid syntax without destroying the accepted UI.
Explicit IDs retain same-kind content through insertion/reordering; anonymous
identity is deterministically `(kind, same-kind anonymous ordinal)`.

Controls default to uncontrolled local values; `controlled: true` applies host
values on accepted submissions. Identity, local text history, directed selection,
focus and component-owned resources survive appropriate updates. Nested controls
also survive changed containers and the existing host `refresh` API. Core owns
MarkdownSlot roots; only host-owned content is retained/reparented. Removal/type
changes retire subtrees and interaction targets. Reset remains intentional via
`set_liquid_source`. Drop-counted regression leases verify resource retirement.

Public additions include `UpdateStatus`, `LiquidUpdateError`, `LiquidValue`,
`LiquidControlChanged`, `control_value` and `LiquidView::control`. See
[the Liquid UI contract](crates/rndr_liquid_ui/README.md) and
[grammar documentation](crates/rndr_liquid/README.md). New enum variants can
require downstream exhaustive matches to be extended; no existing API was removed.

### Slice 2 — canvas (`crates/rndr_canvas_ui`)

Extended the existing editing draft with directed UTF-8 selection, pointer caret
placement/drag selection, keyboard navigation, arbitrary replacement/deletion,
core grapheme-safe `edit_string` operations, and platform clipboard shortcuts via
core `Clipboard`. Painting and editing share shaping/style/wrapping geometry and
object/parent/camera transforms. Focus ownership, early shortcut reservation,
Tab traversal, foreign controls and window blur are covered by regressions.

Text sessions commit through existing CanvasStore transactions/history, including
no-op admission and undo/redo across text and object manipulation. Composition
uses core frame filtering/commit deduplication and the runner's CapturesKeys route.
Host APIs include `edit_text`, `text_selection`, `set_text_selection`,
`set_text_engine`, `handle_native_text` and `view::focus`/`take_keyboard`.
See [canvas integration contracts and limits](crates/rndr_canvas_ui/README.md).
No new text engine, document authority or service was introduced.

### Single integrated host fixture

`examples/liquid_canvas_integration` provides visible staged and timed mock-host
streaming, control state/focus status, controlled-data/reorder/partial/type-change/
reset transitions, plus canvas editing, commit/history and camera controls.
It has five combined integration tests, headless smoke mode and real offscreen
export. Workspace and lockfile changes only register this local example.

```sh
source ~/benchmarks/rndr-liquid-canvas-20261108/enter.sh medousa
cargo run -p liquid_canvas_integration
# Suitable without a desktop:
cargo run -p liquid_canvas_integration -- --headless
cargo run -p liquid_canvas_integration -- --export "$CARGO_TARGET_DIR/liquid-canvas.png"
```

See the [fixture README](examples/liquid_canvas_integration/README.md) for a
reproducible native acceptance checklist and its separate validation record.

## Verification evidence

Final automated checks below passed (exit 0). Logs are outside the source tree
in `$CARGO_TARGET_DIR/production-integration-*.log`.

| Executable command | Result |
| --- | --- |
| `cargo fmt --all --check` | Passed |
| `git diff --check` | Passed |
| `cargo clippy -p rndr_canvas_ui -p rndr_liquid -p rndr_liquid_ui -p liquid_canvas_integration --all-targets -- -D warnings` | Passed, no weakened lint gate |
| `cargo check --workspace` | Passed |
| `cargo test -p rndr_core -p rndr_ui -p rndr_paint -p rndr_render -p rndr_canvas -p rndr_canvas_ui -p rndr_liquid -p rndr_liquid_ui -p liquid_canvas_integration` | **1,374 passed, 0 failed, 6 ignored**, including doctests |
| `cargo test -p source-session` | 20 passed |
| `cargo check -p rndr_core -p rndr_ui -p rndr_paint --no-default-features` | Passed |
| `cargo test -p rndr --no-default-features --test editing_compatibility` | 2 passed |
| `cargo test -p rndr --no-default-features --features ui --test editing_compatibility` | 3 passed |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p rndr_liquid -p rndr_liquid_ui -p rndr_canvas_ui --no-deps` | Passed |
| `cargo run -p liquid_canvas_integration -- --headless` | All staged lifecycle smoke transitions passed |
| `cargo run -p liquid_canvas_integration -- --export "$CARGO_TARGET_DIR/liquid-canvas.png"` | Offscreen renderer wrote a 1500×960 RGBA PNG |

The six existing opt-in probes remained ignored: `snapshot_storage_accounting`,
`editor_scale_costs`, `model_costs`, `live_only_frame_cost`,
`scale::editor_scale_costs`, and `shaped_editor_scale_costs`. No performance,
allocation/RSS or cross-platform equivalence claim is made. No test was weakened.

Verification found and resolved a new canvas Tab-traversal regression. A new
Liquid event test initially observed the writer queue too early; it now asserts
publication at core's actual frame boundary and that host updates emit no user
change event. Final reruns passed, including seven new Liquid lifecycle tests,
six grammar regressions and the combined fixture tests. No baseline failure was
observed in the affected pre-change test set.

### Pixels versus native interaction

- **Automated/headless:** editor state, transform geometry, focus/input routing,
  clipboard-resource protocol, composition filtering and transactions were tested.
- **Rendered output:** the real offscreen renderer successfully produced
  `$CARGO_TARGET_DIR/liquid-canvas.png`. It was not visually inspected; no golden
  pixel comparison or human-rendered-image acceptance is claimed. Fixture tests
  separately check that both panes/canvas have visible geometry.
- **Actual native interaction:** not performed. `DISPLAY` and `WAYLAND_DISPLAY`
  were unset; neither Xvfb nor xdotool was available. The missing prerequisite is
  a suitable running desktop with pointer/keyboard, system clipboard, configured
  IME and (for AT claims) an assistive-technology test setup. No system packages
  were installed. Native focus, clipboard, accessibility and IME remain unverified.

## Explicit remaining limitations and risks

- Native/manual acceptance and visual inspection above remain outstanding; this
  result is not claimed to be platform-certified or fully production-ready.
- Anonymous same-kind collection changes cannot infer logical identity; use IDs.
  Initial/reset constructors remain infallible for compatibility, so callers must
  validate untrusted initial duplicate IDs rather than depend on ambiguous lookup.
- Incomplete snapshots hold all last-valid content, not just the unfinished block.
  Nested Markdown mounting follows normal frame synchronization. No cost bound or
  performance equivalence was measured.
- Canvas editing is plain text: changed content clears inline UTF-16 style spans,
  while object-level style/layout remain. Undo is per committed text session, not
  per keystroke. Word/page navigation is not exposed by this adapter.
- Preedit overlay does not reflow surrounding text and does not publish a
  transformed candidate-window rectangle. Native composition must be verified
  separately. Slider semantics are named/button-like, not a certified native
  accessibility range/SetValue implementation.
- Cross-platform CI, ignored scale probes and native manual checks were not run.

## Commits and final Git status

Final HEAD is still `b2a1c214cb9856a73efca25000a0fa5bf51c1b25` on
`benchmark/medousa`. **No final deliverable commits were created:** the active
attached-checkout policy explicitly forbids staging, committing or changing HEAD.
Consequently all intended deliverables are intentionally uncommitted, and the
working tree cannot be left clean under that policy. The logical review slices
are the two crate groups above, then the example/workspace registration and this
report. No other workspace/branch, benchmark brief, setup script, shared tool
configuration or sibling build directory was modified. Generated images/build
outputs/logs remain outside the source tree; no secrets were added.

Final `git status --short`:

```text
 M Cargo.lock
 M Cargo.toml
 M crates/rndr_canvas_ui/README.md
 M crates/rndr_canvas_ui/src/editor.rs
 M crates/rndr_canvas_ui/src/lib.rs
 M crates/rndr_canvas_ui/src/overlay.rs
 M crates/rndr_canvas_ui/src/paint.rs
 M crates/rndr_canvas_ui/src/view.rs
 M crates/rndr_liquid/README.md
 M crates/rndr_liquid/src/block.rs
 M crates/rndr_liquid/src/document.rs
 M crates/rndr_liquid/src/lib.rs
 M crates/rndr_liquid_ui/README.md
 M crates/rndr_liquid_ui/src/lib.rs
 M crates/rndr_liquid_ui/src/render.rs
 M crates/rndr_liquid_ui/src/view.rs
?? BENCHMARK_REPORT.md
?? crates/rndr_canvas_ui/src/text_edit.rs
?? crates/rndr_canvas_ui/tests/text_editing.rs
?? crates/rndr_liquid/src/controls.rs
?? crates/rndr_liquid/tests/reconciliation.rs
?? crates/rndr_liquid_ui/src/controls.rs
?? crates/rndr_liquid_ui/tests/reconcile.rs
?? examples/liquid_canvas_integration/
```
