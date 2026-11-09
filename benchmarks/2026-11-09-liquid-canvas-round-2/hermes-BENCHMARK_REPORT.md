# Production-integration benchmark delivery

## Repository and scope

- Root: `/home/theelevators/benchmarks/rndr-liquid-canvas-20261108/worktrees/hermes`
- Branch: `benchmark/hermes`
- Starting commit: `b2a1c214cb9856a73efca25000a0fa5bf51c1b25`; starting status clean.
- Specification read in full from inherited `BENCH_TASK`, without modifying it.
- Inherited build directory verified and preserved:
  `/srv/benchmarks/rndr-liquid-canvas-20261108/build/hermes`.
- No sibling solutions, shared configuration, system packages, pushes or merges.

## Delivered changes and commits

1. `66f0309`: Liquid grammar: explicit `key:` metadata, checkbox/input/slider
   payloads, streaming fence completeness and regression tests.
2. `9796aca`: Core Markdown owner: validated explicit slot retention/rebinding
   across arbitrary source replacement/reordering, including retirement cleanup.
3. `ee4a760`: Canvas text editing through existing core Buffer/Editor/IME/clipboard
   primitives, shaped pointer/selection geometry, native-world focus routing,
   transaction/history safeguards and regression tests.
4. `cbd1767`: Liquid host reconciliation, retained controls/focus/drafts, controlled
   publication, nested body retention, lifecycle/resource tests; header-only
   metadata preserves literal code and slider ranges reject overflow.
5. `cfb030a`: One runnable combined host, timer-driven stream workflow, canvas
   editing/manipulation controls, integration tests and run documentation.
6. Final report-only commit: `HEAD` after committing this file. Its exact hash is
   supplied in the delivery response (a commit cannot contain its own hash).

All intended deliverables are committed. Final `git status --porcelain` is checked
empty after the report commit. Build logs/images remain outside the source tree;
no build artifacts or secrets are committed.

## Public integration contracts

### Liquid

`update_liquid_source(world, view, full_candidate)` returns `Applied`, `Incomplete`,
or a typed error. Unique explicit `key:` values retain compatible component roots
through insertions/reordering. Duplicate IDs are refused before mutation. A kind
change creates a new incarnation. Removal drops attached ECS state/RAII resources
and forgets focus, capture and queued owned intents. Host-global subscriptions
remain host-owned; store cancellation guards on the component subtree.

Keys belong in a fence's leading header, before the first blank line or `---`.
Domain `id:` fields keep their old meaning. Unkeyed identity is canonical kind plus
ordinal among unkeyed blocks of that kind; same-kind insertion/reordering may
transfer state, so editable/movable content should use keys. Constructors/reset
render duplicate-key blocks independently, without reconcilable explicit identity.

Controls maintain local drafts. Uncontrolled source values are initial defaults;
controlled values overwrite drafts on every accepted host snapshot, even an equal
snapshot. Changes emit `LiquidEvent::ControlChanged`; the host validates/acknowledges
proposals. Synchronous host acknowledgement wins. Changed controlled text clears
stale field history/selection/composition through `reset_field_draft`, keeping focus.
Incomplete fences leave the entire last valid UI untouched; the host buffers and
resends the completed full snapshot. `set_liquid_source` is still intentional reset.

Core's additive `set_markdown_with_slots` validates handles/destinations before
mutation and retains exact roots on the next sync; repeated pending calls are
latest-wins. Default source-range slot anchoring is unchanged. `LiquidDocument`
adds `ids`, and `LiquidBlock`/`LiquidEvent` add variants: exhaustive downstream
matches and document struct literals need updates. Existing constructors remain.
See `crates/rndr_liquid_ui/README.md` for the full contract.

### Canvas

Existing `view::install`/`view::update` hosts gain pointer caret/selection,
keyboard arbitrary-position editing, platform shortcuts/clipboard and transient
composition through the existing core editor. CanvasStore remains the persisted
history authority: a whole text session is one revisioned transaction, alongside
object gestures. Escape/Primary+Enter commits/exits; plain Enter inserts a newline.
Undo during editing publishes then undoes the session, without traversing a remote
transaction on publication conflict. Other focused controls retain their keys.
Toolbar hosts explicitly request canvas-slot focus after starting editing.
Read-only `text_selection`/`text_geometry` APIs expose directed byte selection and
scene-space editing quads. See `crates/rndr_canvas_ui/TEXT_EDITING.md`.

## Run the integrated example

    cargo run -p liquid_canvas_integration

Modify the three Liquid controls, then click `Stream + focus draft` (six timed
snapshots) or `Next host step`. Steps insert Markdown, publish slider value 70,
reorder keys, introduce/complete a partial fence, then remove/change types. Use
`Reset Liquid` for deliberate reset. Canvas buttons expose Edit, Commit, Zoom/Pan,
Move, Rotate, Undo and Redo; normal pointer/keyboard editing works on the canvas.
The footer exposes focus/draft/revision/edit/history state. No service is required.
Detailed acceptance walkthrough: `examples/liquid_canvas_integration/README.md`.

## Verification evidence

Before implementation, both gaps were confirmed in source: `set_liquid_source`
detached every root, and canvas `type_text` only used `push_str`/`pop`. Baseline
`cargo test -p rndr_liquid -p rndr_liquid_ui -p rndr_canvas -p rndr_canvas_ui` passed;
no existing failure was encountered in that baseline.

Final automated commands (all passed):

    cargo test -p rndr_liquid -p rndr_liquid_ui -p rndr_canvas -p rndr_canvas_ui -p liquid_canvas_integration
    cargo test -p rndr_core -p rndr_ui -p rndr_paint -p rndr_render
    cargo test -p source-session
    cargo check --workspace
    cargo check -p rndr_core -p rndr_ui -p rndr_paint -p rndr_liquid_ui -p rndr_canvas_ui --no-default-features
    cargo test -p rndr --no-default-features --test editing_compatibility
    cargo test -p rndr --no-default-features --features ui --test editing_compatibility
    cargo clippy -p rndr_core -p rndr_ui -p rndr_paint -p rndr_render -p source-session -p rndr_liquid -p rndr_liquid_ui -p rndr_canvas -p rndr_canvas_ui -p liquid_canvas_integration --lib --bins -- -D warnings
    cargo clippy -p liquid_canvas_integration --tests -- -D warnings
    RUSTDOCFLAGS='-D warnings' cargo doc -p rndr_core -p rndr_ui -p rndr_liquid -p rndr_liquid_ui -p rndr_canvas_ui --no-deps
    cargo fmt --all --check
    git diff --check

Broad core/UI/paint/render run: 1,283 passed, zero failed, six pre-existing ignored
opt-in storage/performance probes. Log: `$CARGO_TARGET_DIR/benchmark-core-tests.log`.
No tests/safeguards were weakened. Focused suites include 10 Liquid reconciliation
and 10 canvas text regressions; two combined-app tests exercise host snapshots,
focus isolation, clipboard, camera changes, text commits and shared history.
Observed RED regressions during implementation included stale reorder order,
synchronous host acknowledgements overwritten by a checkbox draft, reset-before-
sync resource retention, destroyed nested fields, literal metadata stripping and
an overflowing slider range. Each passes after its fix. An intermediate canvas
IME-anchor failure also passes in the final suite; it was introduced work, not a
baseline failure.

Rendered-image checks (not native interaction), both exported and visually inspected:

    cargo run -p liquid_canvas_integration -- --stage 3 --editing --export "$TMPDIR/liquid-canvas-integration.png"
    cargo run -p liquid_canvas_integration -- --stage 6 --export "$TMPDIR/liquid-canvas-retirement.png"

Both produced 1280x800 images. Inspection confirmed readable controls/status,
canvas selection/caret, and removal/type-change output. Initial background and
missing-slider-glyph defects were corrected and images re-exported. No independent
immutable-baseline image comparison or performance measurement was performed.

## Native boundary, limitations and remaining risks

- Actual native interaction was NOT performed: `DISPLAY` and `WAYLAND_DISPLAY`
  are unset. A suitable desktop session, real IME and external clipboard peer are
  required. Headless preedit/commit/candidate-anchor tests do not certify native
  composition, candidate-window placement, OS clipboard, OS focus or accessibility.
  Cross-platform desktop/manual acceptance remains outstanding.
- Liquid nested body identity is positional within its parent; moving across
  parent/body owners resets state. Changed legacy catalog command chrome may
  rebuild descendants; retained form controls have the stronger focus guarantee.
- Streaming admission is conservative whole-snapshot buffering for the existing
  unindented backtick grammar, not general Markdown validation. Balanced invalid
  props degrade to fenced code; inferred IDs cannot establish logical identity.
- Slider is a simple draggable text-track with canonical key/focus hooks, not a
  native accessibility Slider/SetValue widget. Canvas accessibility does not expose
  a full editable text object. Neither is claimed as native AT-certified.
- Canvas edits are plain text: byte-changing edits clear inline style spans; undo
  restores the original styled object. One session is one undo step. Preferred
  vertical-column affinity does not persist across separate input batches.
- The reused core interpreter runs in a disposable scratch App per input batch
  because its interpreter/context is private and World/App is not Send. No new
  text engine or durable undo authority was introduced, but allocation/latency
  have not been measured. No performance claim is made.
- Middle-button remains canvas pan, not primary-selection paste. Selection/caret
  overlay is not separately clipped to nested frame boundaries. Real native IME,
  clipboard, accessibility and these visual limits need platform follow-up before
  claiming unrestricted production readiness.
