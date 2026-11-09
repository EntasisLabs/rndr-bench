# Liquid + Canvas integration fixture

One native application mounts `LiquidUiPlugin`, the core Markdown plugin,
`CanvasView`, and the standard canvas toolbar/layers/inspector panels. The host
is an in-process mock: no daemon, credentials, network or application service.
Canvas changes use a `MemoryStore`; closing the application discards the board.

## Run

From the workspace root (use the assigned `CARGO_TARGET_DIR` in benchmark environments):

```sh
cargo run -p liquid_canvas_integration
cargo test -p liquid_canvas_integration
cargo run -p liquid_canvas_integration -- --headless
cargo run -p liquid_canvas_integration -- --export /tmp/liquid-canvas.png
```

The desktop opens at 1500 × 960 logical pixels. `--headless` exercises both
subsystems in one world without starting a window or GPU. `--export` uses the
real renderer offscreen and writes one PNG. Neither command proves real native
clipboard, IME, accessibility or pointer interaction correctness.

## Liquid acceptance workflow

1. Change the checkbox, edit the draft input, and drag the slider. The checkbox
   and draft are uncontrolled; the slider is controlled.
2. Click **Next host update** to insert Markdown before/between the controls.
3. Advance again: host data attempts to replace the uncontrolled draft (its local
   value should remain) and supplies slider value 80 (which should be applied).
4. Advance: explicit IDs reorder the controls.
5. Advance: an unfinished callout fence returns `Deferred`, leaving the last valid
   UI intact. Advance again to close the fence and accept the complete document.
6. Advance: checkbox is removed and the `volume` ID changes slider → input; the
   new input starts fresh. Advance once more, or use **Reset Liquid**, to replace
   the document intentionally and reset control state.

The status line displays the accepted/deferred result and stage. **Next** stops at
stage 7. **Auto stream (5s)** starts at stage 0 and advances every five seconds;
click the draft input after starting it to observe focus through host updates
without clicking a navigation button. **Stop stream** stops that timer. Manual
button clicks are ordinary focus interactions; use the timed sequence to observe
retained focus, not a claim that clicking another control should preserve focus.

Stable fence IDs are `accepted`, `draft`, and `volume`. The example uses
`update_liquid_source` for host updates and `set_liquid_source` only for the
intentional replacement stage. It never edits a Liquid view's underlying
`MarkdownView` source separately. See the public Liquid API documentation for
fallback identity, duplicate-ID refusal and controlled/uncontrolled policy.

## Canvas acceptance workflow

1. Click **Edit text**, or double-click the canvas text. Place the caret in its
   middle; Shift+arrows or pointer dragging select a substring. Type replacement
   text, use standard primary-modifier copy/cut/paste shortcuts, and navigate with
   arrows/Home/End as supported by the canvas editor.
2. Use **Zoom + / −** and **Pan +30**, then place the caret and edit again. The
   viewport and editing projection use the same canvas camera. **Reset camera**
   restores its initial camera. Native composition must be exercised on an actual
   desktop with an installed IME; headless input cannot certify it.
3. Click **Commit / exit** (or Escape as specified by the canvas editor), then
   **Move object** to create a distinct object transaction. Normal canvas tools
   and dragging also remain available in the standard panels.
4. **Undo** first undoes the move, then the committed text edit. **Redo** reapplies
   both. The status line exposes edit mode, document revision, history availability,
   zoom/pan and the last canvas error. Confirm ordinary canvas shortcuts act on
   objects again after leaving text mode.

The standard toolbar and inspector have bounded scrolling viewports. When the
responsive toolbar cannot fit every action in one row, it stacks them; scroll
inside that toolbar rather than letting it consume the canvas. The fixture tests
assert the canvas retains a visible rectangle larger than 400 × 300 logical
pixels inside the initial window.

The example does not implement a second text engine or mutate store documents
behind the canvas editor. The visible edit/commit/move buttons delegate to the
same `CanvasEditor` APIs used by native interaction. The fixture tests establish
that Liquid updates do not replace the canvas world or its manipulation history;
subsystem regression suites establish detailed editing/reconciliation contracts.

## Evidence boundaries

Record automated commands, image generation/inspection, and actual desktop
interaction separately. A generated PNG is not proof of interaction; a headless
lifecycle smoke test is not proof of native composition. Platform clipboard and
IME behavior require an available desktop session and must not be inferred from
the smoke test. No native interaction or IME result is asserted by this README.
