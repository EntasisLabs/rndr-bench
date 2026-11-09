# rndr_canvas_ui

Paint and edit [`rndr_canvas`](../rndr_canvas) documents with `rndr_paint`,
`rndr_core` input and `rndr_ui` controls.

| Module | What it does |
| --- | --- |
| `paint` | `paint_scene` draws a scene into an `rndr_paint::Canvas`: frames with rounded clipping, shadows, shapes, dashed lines with arrowheads, pressure-shaped ink, styled text and cropped images. `visual_bounds` gives the painted extent. |
| `editor` | `CanvasEditor`, a headless editor over a `CanvasStore`. It takes one `EditorInput` per frame and needs no window, so it can be driven from tests. |
| `overlay` | `paint_overlay` draws the selection box and handles, hover outline, marquee, lasso, eraser trail and snap guides. |
| `view` | Runs a `CanvasView` in an `rndr_core` world: reads mouse, touch, pen, wheel and keys over a `SurfaceSlot` and paints the scene and overlay into it. |
| `panels` | Toolbar, arrange buttons, stroke/fill swatches, widths, text sizes and a layers list (select, hide, lock), built from `rndr_ui` buttons. |

Text is measured with the world's `TextEngine` (`EngineMeasure`), so
picking text lines up with what is painted. Without an explicitly installed
engine, the view uses core's builtin engine, just like text painting.

## Editing model

While a gesture is in progress the editor keeps a draft copy of the nodes,
which is what gets painted. Releasing commits one `Transaction` to the
store, so each gesture is one undo step. A conflict reloads the document and
is reported through `CanvasEditor::last_error`. Camera, selection, tool and
style live in the editor, never in the document.

Tools: select (`V`), hand (`H`), pen (`P`), eraser (`E`), rectangle (`R`),
ellipse (`O`), line (`L`), arrow (`A`), text (`T`) and frame (`F`).

- Select: click, shift-click, marquee, alt-drag lasso, double-click into a
  group or to edit text. Drag to move with grid and smart-guide snapping;
  drag handles to resize, rotate (shift snaps to 15°) or move line ends.
- Shapes, lines and frames: drag to place; shift keeps squares, circles and
  45° lines. A new node is placed inside the frame under the pointer.
- Pen: records pressure, tilt and time from the pointer's samples. A pen's
  eraser end, or the eraser tool, removes whole strokes or cuts ink at the
  points it touches.
- Pan with the hand tool, the middle button or the wheel; zoom with the
  primary modifier (Ctrl, or Cmd on macOS) and the wheel.
- Keys: undo `Mod+Z`, redo `Mod+Shift+Z` (also `Ctrl+Y` on Windows/Linux), duplicate `Mod+D`,
  group `Mod+G`, ungroup `Mod+Shift+G`, select all `Mod+A`, delete
  `Delete`/`Backspace`, layer order `]`/`[`, nudge with arrows (shift for
  10), `Escape` cancels a gesture, then clears the selection, then leaves
  the group.

## Using it

```rust,ignore
let editor = CanvasEditor::open(store, "board")?;
view::install(world, CanvasView::new(editor, SLOT));      // world has a SurfaceSlot { id: SLOT }
panels::mount(world, toolbar, side, [toolbar_width, side_width]);
app.add_system_front(Schedule::Input, view::take_wheel);   // ahead of apply_scroll
app.add_system(Schedule::Update, |world: &mut World| {
    for request in take_action_intents(world) {
        panels::dispatch(world, &request);
    }
    view::update(world);
    panels::refresh(world);
});
```

`examples/canvas-editor` is a complete app; `--export out.png` writes a frame
without opening a window.

## Interactive text integration

Use the text tool to create a text object, double-click an existing text object,
or call `CanvasEditor::edit_text(id)`. Programmatic entry from a toolbar should
also call `view::focus(world)` to transfer keyboard ownership to the canvas.
Locked, hidden, and otherwise non-editable nodes are refused by `edit_text`.

Click to position a caret and drag to select; Shift-click extends the selection.
Left/Right and Shift+arrows navigate/select by core grapheme boundaries, Up/Down
use shaped visual lines, Home/End select line boundaries, and primary+A selects
all. Typing replaces selections. Backspace/Delete edit around the caret; Enter
inserts a newline. Primary+C/X/V use the existing world `Clipboard`, including
its dirty flag for platform publication. Shortcut matching uses the core
platform table, including physical letter keys on non-Latin keyboard layouts.
Escape or primary+Enter commits and exits editing. Clicking outside the edited
object, leaving canvas focus, or Tab commits the draft as well.

`CanvasEditor::text_selection()` and `set_text_selection(anchor, caret)` expose
UTF-8 byte offsets for host adapters and test automation. Invalid UTF-8 boundaries
are refused. Pointer hit testing, visual-line navigation, selection and caret
painting share the paint path's text style, spans, wrapping width and `TextEngine`.
Camera pan/zoom and scene parent/object matrices map pointer and editing chrome
between coordinate spaces. `view::update` supplies the world's shaping engine;
headless callers can use `set_text_engine` explicitly.

A draft remains outside the saved scene until `finish_text()`. A changed draft
is one existing `CanvasStore` transaction; an unchanged draft is no transaction.
Document undo/redo therefore crosses text edits and object gestures. Primary+Z
while editing commits and undoes that complete session, then exits editing;
there is no separate per-keystroke undo history. Store failures remain observable
through `last_error`; conflicts follow the existing reload policy. Both Undo and
Redo stop if publishing the pending text draft is refused: the triggering request
never traverses history on the newly reloaded snapshot. The disposable draft is
retired, the exact conflict remains in `last_error`, and the other writer's
transaction and history flags remain intact. A subsequent deliberate history
request may operate on the refreshed snapshot. Unchanged drafts are not refusals.
All input routes (direct methods, `EditorInput`, native adapter) share this gate.

An IME commit is accepted text, not cancellable preedit: the adapter inserts it
exactly once before processing exit or history actions in the same input batch.
Escape/primary+Enter therefore save that commit; Undo can undo it as part of the
session and Redo can restore it. While preedit is nonempty, the IME retains key
ownership and only committed text changes the draft; preedit itself is never saved.

`view::take_wheel` now reserves the focused canvas keyboard queue too; keep it
at the front of `Schedule::Input`, before Chrome/global keymaps. Alternatively,
register `view::take_keyboard` there when no wheel interception is needed.
`view::update` consumes the reserved queue once, never stealing input when a
foreign control has focus. Tab is left for native focus traversal after the text
draft is committed. Both reservation and direct/pre-Chrome consumption require
current canonical focus eligibility (`access_details(...).focus`), not just a
stale `Focus` entity. Losing eligibility commits the previously accepted draft
once and releases editing/capture; queued input is discarded rather than replayed
when re-enabled. A refused direct queue remains available to its owner. Native
Undo/Redo use core's platform shortcut resolver even after text editing exits,
including physical Z/Y with non-Latin logical characters. Released/synthetic
keys and history chords rejected by core are not reinterpreted as history by the
compact editor. The low-level `EditorInput` API remains platform-neutral.
While editing, the surface carries `CapturesKeys`, which
also enables the desktop runner's existing IME route. Direct headless/native
adapters can call `handle_native_text(&Input, Platform, &mut Clipboard)` instead
of replaying those same keys through `handle`.

Regression command: `cargo test -p rndr_canvas_ui`. The `text_editing` suite covers
substring replacement, core grapheme deletion, pointer drag under rotated parent
and object transforms plus camera pan/zoom, platform copy/cut/paste, native frame
composition filtering, focus ownership/traversal, no-op admission, and combined
text/manipulation history. These are headless tests, not native interaction or
pixel-inspection evidence.

## Limits

- Canvas text editing is plain-text editing: changing content clears inline
  UTF-16 style spans rather than applying stale offsets to new text. Object-level
  text style and layout remain intact. Word-wise and page-wise navigation are
  not currently exposed by the canvas adapter.
- Composition uses core's `text_frame` filtering and commit deduplication;
  preedit is transient overlay text, not a saved document mutation. Native IME
  sessions still require platform verification. The current overlay does not
  reflow surrounding text around preedit, and does not publish a transformed
  candidate-window rectangle to the runner.
- The inspector applies its swatches and sizes to the selection and to new
  nodes; it does not read the selected node's current style, and offers
  fixed swatches rather than a color picker.
- Image assets are drawn from texture ids the app supplies in
  `CanvasView::images`; the editor does not import images yet.
