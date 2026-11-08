# Liquid + Canvas integration fixture (partial)

```sh
cargo run -p liquid_canvas_integration
cargo test -p liquid_canvas_integration
cargo run -p liquid_canvas_integration -- --export /tmp/liquid-canvas.png
```

No external service is required; the default canvas store is in memory.

- Open **Open me**, then click **Append host chunk** repeatedly. Each click
  simulates a host streaming snapshot; the accordion entities and state survive.
- **Reset Liquid document** deliberately retires the old interactive subtree.
- Double-click the canvas title to edit, or use Text to create another object.
  Left/Right navigate; Shift+Left/Right select; typing replaces the selection.
  Backspace/Delete remove graphemes; primary+A selects all. Escape or
  primary+Enter commits. Select and drag an object, then use Undo/Redo.
- Existing canvas toolbar zoom/pan and inspector remain available. Scroll the
  right panel to reach the remaining inspector controls.

## Contracts and limits

`append_liquid_source` takes the complete accumulated snapshot. Parsed blocks
and Markdown must retain their exact published prefixes, otherwise it returns
`LiquidAppendError::ChangedPrefix` without mutation. Buffer refused chunks and
retry, or deliberately reset with `set_liquid_source`. Existing reader state is
uncontrolled; changing published props requires reset. This is **not** general
keyed reconciliation: no explicit IDs, reorder, insertion or removal support.
Incomplete syntax is not guaranteed to be withheld, although refused updates
leave the previous UI intact.

Canvas editing reuses `rndr_core::edit_string` for grapheme navigation and
selection replacement. `CanvasEditor::text_selection()` exposes directed UTF-8
byte offsets. Draft edits still commit through the existing canvas transaction
history, not a separate source history. There is no new native clipboard or IME
adapter, pointer caret hit testing, caret/selection drawing, vertical navigation,
or text-local undo. The example demonstrates a partial integration, **not** the
complete benchmark acceptance workflows. Native focus/IME are unverified.
