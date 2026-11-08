# Canvas text integration contract and evidence

## Ownership and reused infrastructure

`CanvasStore` remains the sole persisted document and undo authority. A text
session retains a core `Buffer` with directed UTF-8 byte selection and a draft
node. `nodes()` exposes that draft; `finish_text()` publishes one revisioned
transaction. Existing empty-text deletion policy is preserved. Reload drops
both the text draft and object-gesture preview. A conflict reloads and records
`last_error`; a refused text save stops an accompanying undo before traversal.

The text interpreter is **not reimplemented**. The public core `Chrome` plugin
runs its existing `Editor`/`Buffer` input path on a disposable headless `App`.
This reuses core grapheme/word/document navigation, selection replacement,
delete/insert, platform shortcuts, clipboard handling and `text_frame` IME
filtering. Core's editor pass and its internal history/vertical-goal types are
private; core `World`/`App` is not Send, while `CanvasView` is a Send+Sync world
resource. Consequently this crate retains the Buffer, not a nested App, between
input batches. Scratch history is intentionally discarded. Canvas undo is a
whole-session operation; it is never a competing source-session history.
This bridge has not been performance-benchmarked; it allocates a scratch App
for an editing batch. A future public core interpreter context could avoid
that overhead and retain preferred-column affinity across batches.

The installed view passes native key events without reducing them to the
legacy canvas shortcut enum, plus the host Platform and Clipboard resources.
Clipboard `dirty` fields return to the host world for the existing native
runner to publish. Core's `ImePreview` is retained from the projection; scene
paint includes preedit, but committed Buffer/node bytes do not. A subsequent
commit is inserted once by core's existing duplicate filtering. While preedit
is nonempty, text pointer placement is withheld and finish/history keys do not
publish the session. Explicit host finish or focus loss discards uncommitted
preedit and publishes only committed Buffer bytes.

Paint and input share `text_source`: identical font, UTF-16-to-byte styled
spans, wrap width, line height, alignment, spacing and padding. `TextLayout`
provides caret positions, selection boxes and pointer hit testing. Object and
ancestor matrices map local geometry to scene space; the camera maps scene to
view space. The view publishes a caret-only TextLayout on the slot so the
native runner can locate an axis-aligned IME candidate anchor under pan,
zoom and rotation. It is not a second visible editor or document projection.

## Host use

Keep existing `view::install` / `view::update` integration. The slot must have
its layout Rect before updating. The view claims keyboard input only while its
slot owns focus or receives a pointer press; a press outside or another focused
control ends text editing without consuming that control's keys. Input must
reach the view before application-global shortcut handlers. For strict earliest
ownership, install `view::update` at the front of Input after UI plugins, with
`take_wheel` ahead of scrolling. Do not run update twice per frame.

To start from a host toolbar: `editor.edit_text(id)`, then
`rndr_core::request_focus(world, slot_entity)`. Add `Focusable` when constructing
the slot if requesting focus before its first view update. The view adds and
removes `CapturesKeys` during text editing, allowing the native runner to enable
IME. It leaves other controls' focus and native input intact.

For headless drivers:

- `CanvasEditor::handle(&EditorInput, &TextMeasure)` accepts text, pointer,
  arrows, directed Shift-selection, deletion, session exit and canvas shortcuts.
- `set_text_engine(Arc<dyn TextEngine>)` plus a matching `EngineMeasure` permits
  the same geometry as a host painter. The default engine is core's builtin.
- `editing_text()` returns the node id; `text_selection()` returns
  `(caret, anchor)` byte offsets; `text_geometry()` returns scene-space caret
  and selection quads. Camera movement is applied separately by callers.
- To test native clipboard, IME, Home/End and full platform modifiers, insert
  core `Input`, `Platform`, `Clipboard` and `Focus` in a World with a SurfaceSlot
  and CanvasView, then call `view::update`; no window is required.
- `finish_text`, `reload`, `undo`, `redo`, `mutation`, `nodes`, `scene`, and
  camera fields remain compatible with existing hosts.

Escape and Primary+Enter finish; plain Enter inserts a newline. Primary+Z
while editing publishes and undoes the whole session, then exits. Once exited,
shortcuts return to canvas commands. Camera wheel pan/zoom does not end editing.
The first text mutation clears inline rich spans rather than pretending old
ranges remain valid; unchanged sessions preserve the original node exactly.

## Test-first evidence

Initial RED command (before replacing append-only editing):

```sh
cargo test -p rndr_canvas_ui --test text_editing arbitrary_keyboard_selection_replaces_unicode_without_canvas_commands -- --exact
```

Observed exit 101: expected `a👩‍💻Rc`, got `a👩‍💻bcR`. The same test passed after
routing text through core Buffer/Editor interpretation. Subsequent tracer
bullets were independently observed RED before implementing their behavior:

- Rotated/zoomed pointer selection expected `ab!ef`, got `abcdef!`.
- Host clipboard/IME test initially found no host Clipboard resource; preedit
  display assertion later failed before transient display was added.
- Focus test initially failed `native runner must enable IME`, then its native
  candidate-anchor assertion failed before slot caret geometry was published.
- Text overlay expected outline+caret (2 commands), got 12 object-handle commands.
- In-edit undo expected `abc`, got `abc!`.
- Queued text+Escape expected `abc確!`, got `abc確` before chronological dispatch.
- Reload expected no text session, got `Some("text")`.
- Builtin-engine pointer placement unexpectedly ended the session before the
  view's estimate fallback was replaced with the actual builtin engine.
- Stale-session undo traversed a remote transaction before conflict handling
  stopped the operation at the publication boundary.
- Wrapped vertical motion expected byte 7, got byte 1 before input width used
  the same measured text box as painting.

All tests are in `tests/text_editing.rs`, alongside the existing editor, paint,
view and panel regression suites. The double-click legacy assertion now expects
insertion at the clicked caret, rather than append-at-end behavior.

Reproducible focused checks:

```sh
cargo test -p rndr_canvas -p rndr_canvas_ui
cargo clippy -p rndr_canvas_ui --lib --tests -- -D warnings
cargo check -p rndr_canvas_ui --no-default-features
RUSTDOCFLAGS='-D warnings' cargo doc -p rndr_canvas_ui --no-deps
rustfmt --edition 2021 --check crates/rndr_canvas_ui/src/*.rs crates/rndr_canvas_ui/tests/*.rs
git diff --check -- crates/rndr_canvas_ui
```

## Verification boundary and remaining limits

The tests prove headless input, document/history transitions, host resource
routing, shaped geometry and display-list commands. They do **not** prove a
native clipboard round trip, platform IME composition, candidate-window
placement on a desktop, accessibility editing, or interaction performance.
No DISPLAY or WAYLAND_DISPLAY is available in the task environment. Native
verification is explicitly **not run**, not passed. Integrated image rendering
and inspection belong to the parent integration slice.

Required manual native follow-up: launch the integrated example on each target
platform; compose and cancel non-Latin input with a real IME; verify replacement
and one-time commit at a selected substring; inspect the candidate anchor after
pan/zoom/rotation; copy/cut/paste between the canvas and another native app;
check focus transfer to host fields and shortcut return after exit. Record the
actual input method/platform and results separately.

Additional limits: plain text editing clears inline spans on mutation; one
session is one undo step; vertical preferred-column affinity does not persist
between delivered input batches; Linux primary-selection/middle-click paste is
not added (middle button remains canvas pan); selection/caret chrome is an
overlay and is not independently clipped by nested frame boundaries. Native
accessibility does not expose a full editable canvas text object. These are not
claimed as verified features.
