# Liquid + canvas host integration fixture

Run from the workspace root:

    cargo run -p liquid_canvas_integration

No services, files, or daemon are required. The host uses a fresh MemoryStore.
A desktop session and the repository's normal graphics/font prerequisites are
needed for the window. All builds use the inherited CARGO_TARGET_DIR.

## Liquid workflow

1. Change the checkbox, type in the local draft field and drag the slider.
2. Click `Stream + focus draft`. It returns focus to the text field and schedules
   six host snapshots, 900 ms apart, through rndr's timer/event infrastructure.
   Keep typing while the snapshots arrive, or click `Next host step` manually.
3. Step 1 inserts Markdown before and between controls; step 2 publishes the
   controlled slider value 70; step 3 reorders explicit keys; step 4 sends an
   incomplete fence and reports `Incomplete`; step 5 completes it; step 6 removes
   the checkbox and changes the slider's kind to input. The uncontrolled text
   draft remains, including its entity and focus. The slider is controlled, so
   each accepted source snapshot republishes the host value.
4. `Host value 70` publishes step 2 directly. `Reset Liquid` intentionally retires
   every Liquid incarnation and returns to the initial document. Reset invalidates
   outstanding stream generations. Starting another stream invalidates the old one.

The footer shows focus, the local Liquid draft, the last control proposal, canvas
editing state, revision and history availability. The host logs transitions to
stdout. The snapshots are full candidates, not append-only source commands.

## Canvas workflow

1. Click `Edit text`, or double-click the canvas text.
2. Click in the middle, drag a selection, or use Shift+arrows. Type to replace.
   Home/End, word movement, Delete and Backspace use the existing core editor.
3. Use Ctrl/Cmd+C, X and V through rndr's Clipboard abstraction. Native OS clipboard
   behavior still needs separate platform verification.
4. Wheel pans; Ctrl/Cmd+wheel zooms. `Zoom / pan` toggles a repeatable camera change.
   Edit again under that camera; `Rotate +15°` exercises object rotation.
5. `Commit / exit`, Escape, Ctrl/Cmd+Enter, or leaving the canvas commits the draft
   through CanvasStore. Enter without the command modifier inserts a newline.
6. `Move +40` makes a separate object transaction. Undo twice restores movement,
   then text. Redo twice restores both. Text edit-mode Ctrl/Cmd+Z commits the
   draft then uses the authoritative canvas transaction history, not a rival
   durable scratch history. Outside text editing, letter/tool and arrow/movement
   shortcuts belong to the canvas again.

Toolbar commands are host adapters. The actual text editing, geometry, focus,
clipboard and transactions live in rndr, not in the example.

## Automated and rendered verification

    cargo test -p liquid_canvas_integration
    cargo test -p rndr_liquid -p rndr_liquid_ui -p rndr_canvas -p rndr_canvas_ui
    cargo run -p liquid_canvas_integration -- --export "$TMPDIR/integration.png"
    cargo run -p liquid_canvas_integration -- --stage 3 --editing --export "$TMPDIR/integration-editing.png"

`--stage N` publishes one snapshot before opening/exporting; `--editing` enters
editing and selects the first word using the native-style core key queue.
The integration test uses the real combined app/frame/input path to assert that
Liquid drafts/focus do not leak keys to canvas tools and that text and movement
share authoritative undo/redo. Subsystem tests cover pointer transforms,
clipboard/preedit simulation, retirement and reconciliation edge cases.

Offscreen images establish rendered appearance only, not native clipboard,
composition, focus, accessibility, or OS presentation certification. See
BENCHMARK_REPORT.md and the two subsystem READMEs for evidence and limits.
