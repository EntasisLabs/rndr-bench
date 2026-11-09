# Production-integration benchmark report

## Corrected review target — 2026-11-09

The principal corrected the review: the actual reviewed revision is
`a22dfd015a7dcdfb742926cc8269c547d7180aab`, not `d73ace95…`. This remediation
addresses the five findings in that corrected review. The prior three-finding
section below is retained as **wrong-review provenance**, not evidence that the
actual P1 findings were already resolved. Its idle invalidation fix is relevant
and retained; disabled-input/physical-shortcut changes and tests are explicitly
carryover from the mistakenly supplied review. No prior work was reverted.
HEAD/index remain untouched; changes are uncommitted in the attached checkout.

### Actual P1 canvas findings: reproduction and contracts

- Added `crates/rndr_canvas_ui/tests/publication.rs`. Before the fix,
  `cargo test -p rndr_canvas_ui --test publication` failed both the conflict
  traversal and IME+Escape tests (1 passed, 2 failed). The history reproduction
  returned true after undoing the other editor's movement; IME+Escape produced
  `abc` rather than `abc確`. Log: `$CARGO_TARGET_DIR/actual-review-publication-before.log`.
- Internal text publication now distinguishes unchanged/absent drafts from
  refused transactions. Undo **and Redo** stop on refusal without a history
  request, preserving the other writer's document/revision/history flags and
  retaining the exact conflict in `last_error`. Existing conflict policy still
  retires the disposable draft and reloads the authoritative store. A subsequent
  deliberate history request may operate on the refreshed snapshot; an unchanged
  draft is not blocked by a stale prior error. Public `finish_text() -> bool`
  remains compatible. Compact/native key handlers no longer publish separately
  before entering this shared history gate.
- Accepted IME commits are inserted once **before** same-batch Escape,
  primary+Enter or history actions. Preedit remains transient and unpublished;
  while it is active, IME key ownership is unchanged. The early Tab adapter exit
  also accepts a pending commit before finishing and clears that commit from the
  world input, while leaving Tab available for traversal.
- `cargo test -p rndr_canvas_ui --test publication --test text_editing` passed
  **15 tests**: five publication tests plus ten existing/carryover editing tests.
  The cases cover direct/compact/native Undo and Redo conflicts, history flags,
  later intentional traversal, unchanged draft after a conflict, committed IME
  text with Escape/legacy Escape/primary Enter/Undo/Redo, Tab and preedit.
  These are automated adapter tests, **not actual language-IME validation**.

### Liquid identities and idle Reset

Initial construction and intentional reset remain infallible, but now use
occurrence-qualified internal identity keys. The first occurrence owns the public
ID and retains state on a later unique-ID snapshot; later duplicates have distinct
live ownership and retire normally, including focused fields. Incremental
snapshots containing duplicate IDs still refuse atomically. The new regression
reproduced the wrong retained identity before the fix, and now covers construction,
reset, duplicate removal, focus cleanup and complete removal. This supersedes the
historical limitation telling callers merely to validate initial duplicates.

The retained core fix makes pending Markdown source, host bindings and measured
host heights authoritative full-update work (`App::wants_update`/`wants_frame`).
The new `idle_reset_review` test follows streamed removal and incompatible
same-ID type replacement, focuses the replacement, then triggers Reset in Update.
After that event frame it advances **only while `wants_frame()` asks for work**,
asserting full updates, no raw slot markers, no retired controls/focus, restored
values and return to idle. The loop bound is a failure guard, not a fixed frame
pump. No timer/frame workaround was added to the example. Generic Markdown
mount/measure/resize/detach and the earlier event-driven reset tests remain green.
The new test initially needed a compile correction to use the existing resource
API (`Option::take`, not nonexistent `World::remove_resource`); final tests pass.

### Legibility implementation and evidence

The fixture root previously exposed the renderer's dark clear behind light-theme
text. A new regression reproduced the missing semantic background. It now paints
semantic Canvas; Liquid controls use semantic Content/Text colors, wrapping
content-metric labels and theme-derived focus borders. The slider uses a portable
ASCII track instead of optional symbol glyphs. Bundled Parley-font regressions
measure header/status/control text fit, light/dark text contrast >=4.5:1 and focus
border contrast >=3:1. Focused input and slider exports are reproducible via
`--export-focus draft|volume`. These numerical checks are not accessibility
certification or human visual acceptance.

Three real-renderer 1500×960 PNGs were exported (outside the source tree):

```sh
cargo run -p liquid_canvas_integration -- --export /tmp/medousa-review-initial.png
cargo run -p liquid_canvas_integration -- --export /tmp/medousa-review-input-focus.png --export-focus draft
cargo run -p liquid_canvas_integration -- --export /tmp/medousa-review-slider-focus.png --export-focus volume
```

All passed. Standard-library PNG decoding confirmed root RGB `(244,245,247)`;
focus-blue pixel counts were 0/2,040/2,000 with localized input/slider changes.
**No human image inspection is claimed.** Exact scoped evidence and pixel bounds
are in `examples/liquid_canvas_integration/VALIDATION.md`.

### Final combined checks on the corrected-review tree

Setup: `source ~/benchmarks/rndr-liquid-canvas-20261108/enter.sh medousa`.
All commands below passed except the explicitly blocked native launch.

| Exact command | Result |
| --- | --- |
| `cargo test -p rndr_canvas_ui --test publication --test text_editing` | 15 passed |
| `cargo test -p rndr_core --test markdown_idle` | 1 passed |
| `cargo test -p rndr_liquid_ui --test idle_reset_review` | 1 passed |
| `cargo test -p rndr_canvas -p rndr_canvas_ui -p rndr_liquid -p rndr_liquid_ui -p liquid_canvas_integration` | **111 passed, 0 failed/ignored** |
| `cargo test -p rndr_core -p rndr_ui -p rndr_paint -p rndr_render -p rndr_canvas -p rndr_canvas_ui -p rndr_liquid -p rndr_liquid_ui -p liquid_canvas_integration` | **1,388 passed, 0 failed, 6 existing probes ignored**, including doctests |
| `cargo test -p rndr_core -p rndr_ui -p rndr_paint -p rndr_render -p source-session` | 1,346 passed, 0 failed, 6 ignored (this command's feature union) |
| `cargo clippy -p rndr_core -p rndr_ui -p rndr_paint -p rndr_render -p source-session --lib --bins -- -D warnings` | Passed |
| `cargo clippy -p rndr_canvas_ui -p rndr_liquid -p rndr_liquid_ui -p liquid_canvas_integration --all-targets -- -D warnings` | Passed |
| `cargo check --workspace` | Passed |
| `cargo check -p rndr_core -p rndr_ui -p rndr_paint --no-default-features` | Passed |
| `cargo test -p rndr --no-default-features --test editing_compatibility` | 2 passed |
| `cargo test -p rndr --no-default-features --features ui --test editing_compatibility` | 3 passed |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p rndr_core -p rndr_ui -p rndr_liquid -p rndr_liquid_ui -p rndr_canvas_ui --no-deps` | Passed |
| `cargo fmt --all --check` | Passed |
| `git diff --check` | Passed |
| `cargo run -p liquid_canvas_integration -- --headless` | All seven lifecycle stages passed |
| `timeout 8s target/debug/liquid_canvas_integration` | **Blocked, exit 101**: no WAYLAND_DISPLAY, WAYLAND_SOCKET or DISPLAY |

Final source had all delegated implementation changes before focused/combined
verification. Counts describe this tree and its feature unions; they do not claim
an unchanged benchmark after the mistakenly supplied review. Logs are outside the
source tree under `$CARGO_TARGET_DIR`: `actual-review-publication-before.log`,
`actual-review-publication-final.log`, `actual-review-idle-core.log`,
`actual-review-idle-reset.log`, `actual-review-focused.log`,
`actual-review-combined.log`, `actual-review-core-gates.log`,
`actual-review-slice-clippy.log`, `actual-review-workspace.log`,
`actual-review-doc.log` and `actual-review-headless.log`.

### Remaining gaps / concrete native blocker

Implementation and automated verification cover all five findings, but **native
visual acceptance and native Reset-and-idle revalidation are outstanding**. This
environment has no configured display, Xvfb/xdotool/Weston or image-view capability.
A suitable desktop session is needed to inspect the actual window, focus and
slider states and confirm Reset with no further interaction. Offscreen pixels,
bundled-font measurements and synthetic input do not substitute for that review.
Actual language-IME, native clipboard revalidation, assistive-technology testing
and cross-platform runtime certification were not performed. Existing plain-text,
preedit reflow/candidate-window and slider accessibility-semantic limitations
remain; no ignored performance probes were enabled. No staging/commits or other
Git mutations were performed; HEAD remains the reviewed `a22dfd0` and the index
is unchanged.

## Earlier, mistakenly supplied review remediation — 2026-11-09

This section supersedes the historical delivery/status record below. Work began
with a clean attached checkout at `a22dfd015a7dcdfb742926cc8269c547d7180aab`.
The reviewed `d73ace95f5293da4d97aa8d34fbab5d83d3ad8a2` object is not available
in this repository (`git show --no-patch` reports bad object). Consequently these
results describe the supplied checkout, not a replay of that exact revision;
test counts are not assumed comparable with the review's 105/771 totals.
No staging, commits, branch changes or other Git mutations were performed.
HEAD remains `a22dfd0`; remediation changes are intentionally uncommitted.

### Findings and contracts

1. **Ineligible canvas input:** this checkout already refused new characters
   after canonical eligibility loss, but disabling before the first adapter
   update left a text draft and `CapturesKeys` alive. The new regression failed
   on that lifecycle assertion before the fix. `view::update` now retires the
   draft whenever current canonical eligibility/focus is absent, without an
   observed-focus cache, and clears pointer capture on ineligibility/blur.
   Existing draft content commits once under the existing focus-loss contract;
   rejected input does not enter that commit. Direct/pre-Chrome and previously
   reserved queues are tested; the direct queue remains unconsumed on refusal,
   reserved input is discarded and cannot replay when re-enabled. No revision
   is created by unchanged drafts or refused characters/shortcuts.
2. **Physical Undo/Redo:** physical Undo during an active draft already worked
   here, but non-Latin physical Redo after that Undo exited editing failed the new
   regression (`abc` stayed `abc`, expected `abcx`). Native translation now uses
   core `standard_action` before losing physical-key information, both for direct
   `editor_input` callers and reserved/direct `update` paths. Core owns the native
   platform policy; compact `EditorInput` remains platform-neutral. Physical Z
   Undo, Shift+Z Redo and Windows/Linux Y Redo are covered for Mac/Windows/X11/
   Wayland table variants. Alt/mixed-modifier history, Cmd+Y, released and
   synthetic keys cannot fall through to the compact history interpreter.
3. **Idle Reset:** Liquid mounts hosts after Markdown's PostUpdate sync. Mounting
   dirties host bindings, but the runner previously saw no reason for a full
   update and could sleep with raw slot Labels. Core now includes dirty Markdown
   source/bindings, stale hosts and post-layout measured-height changes in
   `App::wants_update` and therefore `wants_frame`. This preserves the lifecycle
   owner and the native runner's full-update versus live-only distinction. No
   example frame pumping, timer or perpetual redraw was added. The idle reset
   regression failed before the fix with a raw marker remaining at idle.

Five new tests cover disabled direct/reserved input (before/after observed
focus), platform physical history, event-driven reset, nested mount/refresh,
and generic Markdown host mount/measurement/resize/fixed-height/detach. Idle
checks model one input frame, then advance **only** while `wants_frame` requests
work; they require full updates, disappearance of raw markers and return to idle.
Existing retained identity/focus, text/object history and clipboard tests remain.

### Exact checks on the final source

Setup for all Cargo commands:

```sh
source ~/benchmarks/rndr-liquid-canvas-20261108/enter.sh medousa
```

| Command | Result |
| --- | --- |
| `cargo test -p rndr_canvas_ui --test text_editing` | 10 passed |
| `cargo test -p rndr_canvas -p rndr_canvas_ui -p rndr_liquid -p rndr_liquid_ui -p liquid_canvas_integration` | 102 passed, 0 failed/ignored |
| `cargo test -p rndr_core` | 765 passed, 0 failed, 3 ignored |
| `cargo test -p rndr_core -p rndr_ui -p rndr_paint -p rndr_render -p rndr_canvas -p rndr_canvas_ui -p rndr_liquid -p rndr_liquid_ui -p liquid_canvas_integration` | 1,379 passed, 0 failed, 6 ignored, including doctests |
| `cargo fmt --all --check` and `git diff --check` | Passed |
| `cargo clippy -p rndr_core -p rndr_ui -p rndr_paint -p rndr_render -p source-session --lib --bins -- -D warnings` | Passed |
| `cargo clippy -p rndr_canvas_ui -p rndr_liquid -p rndr_liquid_ui -p liquid_canvas_integration --all-targets -- -D warnings` | Passed |
| `cargo check --workspace` | Passed |
| `cargo check -p rndr_core -p rndr_ui -p rndr_paint --no-default-features` | Passed |
| `cargo test -p source-session` | 20 passed |
| `cargo test -p rndr --no-default-features --test editing_compatibility` | 2 passed |
| `cargo test -p rndr --no-default-features --features ui --test editing_compatibility` | 3 passed |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p rndr_core -p rndr_ui -p rndr_liquid -p rndr_liquid_ui -p rndr_canvas_ui --no-deps` | Passed |
| `cargo run -p liquid_canvas_integration -- --headless` | Staged lifecycle smoke passed |
| `cargo run -p liquid_canvas_integration -- --export "$CARGO_TARGET_DIR/review-liquid-canvas.png"` | Wrote 1500×960 PNG |
| `cargo run -p liquid_canvas_integration` | Blocked, exit 101: neither WAYLAND_DISPLAY, WAYLAND_SOCKET nor DISPLAY set |

Final logs live outside the source tree under `$CARGO_TARGET_DIR`:
`review-regression-final.log`, `review-focused-final.log`, `review-core-final.log`,
`review-gates.log`, and `review-native-launch.log`. Six unchanged opt-in probes
remain ignored: snapshot_storage_accounting, editor_scale_costs, model_costs,
live_only_frame_cost, scale::editor_scale_costs and shaped_editor_scale_costs.
The first combined run passed 1,378 tests but enumerated targets before the new
core idle test existed; the final rerun above includes it. One attempted rerun
wrapper failed shell quoting before Cargo execution and was corrected; no test
failure was hidden. The newly added regressions' pre-fix failures are described
above; final tests and lint gates are green.

### Remaining evidence limits

Native launch was attempted on the rebuilt fixture, but this environment has no
configured desktop, Xvfb or xdotool. The reviewer's independent native observations
are acknowledged, not relabeled as our own revalidation. The PNG export is real
offscreen rendering, not a visually inspected idle-reset recording or golden
comparison. Platform table tests and synthetic key events establish routing
contracts, **not** physical keyboard-layout or cross-platform certification.
System clipboard, real IME/candidate positioning and accessibility/AT were not
retested. Their previously documented limits remain. An actual supported desktop
session is required for native Reset-and-idle acceptance.

The pending Markdown query preserves continuation after supported event/update
mutations; it does not itself wake an already sleeping runner for arbitrary
external world mutation. Use the existing runtime wake route for asynchronous
host work. No performance bound for the pending-state query is claimed.

---

## Historical initial delivery record

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
