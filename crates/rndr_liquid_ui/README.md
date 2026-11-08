# rndr_liquid_ui

Native rendering of [Liquid Markdown](../rndr_liquid) in rndr markdown views.

`liquid_view` parses Liquid source with `rndr_liquid`, shows the ordinary
markdown with `rndr_core`'s markdown view, and mounts each Liquid block in a
markdown host slot, built from `rndr_ui` cards, callouts, buttons and tabs.
Charts are painted by `rndr_chart` in a paint surface.

```rust
use rndr_core::*;
use rndr_liquid_ui::{dispatch, liquid_view, LiquidUiPlugin, NoHost};

let mut app = App::new();
app.add_plugin(rndr_ui::UiPlugin);
app.add_plugin(MarkdownPlugin);
app.add_plugin(LiquidUiPlugin::new(NoHost)); // after MarkdownPlugin
let view = liquid_view(app.world_mut(), "```callout\ntone: tip\nbody: Hi\n```\n");
app.add_system(Schedule::Update, |world: &mut World| {
    for request in take_action_intents(world) {
        if !dispatch(world, &request) {
            // not from a Liquid block: your own controls
        }
    }
});
```

## Host contract

The view never acts on document text. Everything that leaves it goes through
a `LiquidHost`, and every method defaults to doing nothing:

| Method | Called for |
| --- | --- |
| `event(LiquidEvent::Action)` | `actions` and recipe buttons, with the document's `intent` |
| `event(LiquidEvent::OpenLink)` | `cite` and `brief` source links |
| `event(LiquidEvent::Copy)` | the copy button of `code` blocks |
| `event(LiquidEvent::StartTimer)` | recipe steps with a duration |
| `event(LiquidEvent::RefreshFeed)` | the refresh button of `feed` blocks |
| `image(src)` | `media`, `card` and `plan` images; `None` shows alt text |
| `feed(id, datatype)` | `feed` rows, on mount and after `refresh` |
| `tile(feed, field)` | live values for feed-bound `dashboard` tiles |

Intents, URLs and image sources are untrusted document text. Host slots are
for trusted application content, so render documents you would also let
mount controls.

## Block state

Tabs, accordions, carousels, slides and checklists (`steps`, `recipe`,
`kanban`) keep their state in the view and survive `refresh`.
`set_liquid_source` rebuilds every block and resets that state. Checklist
state is not written back to the source.

## Host-driven reconciliation

Use `update_liquid_source(world, view, complete_candidate)` for an updated full
snapshot, not `set_liquid_source`. It returns `Result<LiquidUpdate,
LiquidUpdateError>`:

- `Applied`: accepted; Markdown placement and removal settle on the next frame.
  Retained controls receive their new props synchronously.
- `Incomplete`: an unclosed backtick fence or partial opener was detected. Nothing
  changes, including source, focus and drafts. Buffer in the host and resend the
  completed snapshot. This is deliberately whole-snapshot buffering, not a
  speculative partial component tree. Balanced but invalid component props still
  degrade to ordinary Markdown as in the existing parser.
- Duplicate explicit IDs are rejected before mutation; a missing view or invalid
  underlying Markdown handle is also an error. Do not mutate owned slot roots.

Put an explicit `key: customer-name` line in a fence's leading header to give the
component an ID (before the first blank line or `---`; literal body lines are
preserved). `id:` is NOT an
identity declaration; feed IDs keep their existing domain meaning. Keys must be
unique throughout the parsed document, including nested fences. Reordering keyed
components keeps their roots; changing kind under a key creates a new incarnation.
For initial construction and intentional reset, which have no error return,
duplicate-key documents render independent, non-reconcilable positional roots;
they never share state and the next valid update replaces them.

Without a key, identity is canonical kind plus ordinal among unkeyed components
of that kind. Markdown-only insertions are safe; inserting/removing/reordering
same-kind unkeyed components can transfer state to a different logical component.
This is an inference limitation, not content identity. Use keys for mutable UI.
Removing a component retires its attached subtree, clears owned focus/capture and
queued intents, and drops ECS state/resources. Application-global records and
subscriptions remain host-owned; attach RAII subscription handles to the owned
subtree to cancel them on retirement. `set_liquid_source` remains an explicit
reset, including when an update is still waiting for Markdown sync.

The `checkbox`, `input`, and `slider` fences use the existing semantic checkbox,
canonical TextField and canonical focus/key-navigation hooks. Slider is a simple
text-track control: drag its row, use arrows (1% steps), or Home/End. It is not a
native accessibility slider/SetValue implementation. Example:

````markdown
```input
key: customer-name
label: Customer name
value: Ada
controlled: false
```
````

All three controls have a local draft. `controlled: false` (default) treats source
values as mount-time defaults; accepted host snapshots update labels/ranges but
keep local values. Slider drafts clamp if the range narrows. `controlled: true`
means every accepted snapshot is authoritative, even an identical one: its value
replaces the draft. User changes are optimistic drafts until that publication,
not writes to the application's model. Switching uncontrolled to controlled
immediately publishes the host value; switching back keeps the current draft.
`LiquidEvent::ControlChanged { component, value }` reports each user proposal.
Validate it in `LiquidHost::event` before changing host data. Synchronous host
acknowledgements win over the original draft. A changed controlled text value
uses `reset_field_draft`, clearing stale typing history, caret and composition;
focus stays on the same field. Equal values do not disturb selection/composition.

`LiquidView::component(key)` exposes the retained root for an explicit key in
that view; `control_entity` exposes its canonical focus target and `control_value`
reads its draft. Nested markdown bodies retain their controls when enclosing
report/card chrome changes. Nested body identity is positional within its parent;
moving a control between different parent/body owners resets it. Old catalog
blocks retain their reader state, but prop changes may rebuild their command
chrome; focus identity guarantees apply to retained form controls, not every
legacy button in changed chrome. Unchanged catalog blocks are not rebuilt.

`LiquidDocument` adds an `ids` sidecar field and `LiquidBlock` adds three enum
variants: downstream struct literals and exhaustive enum matches require updates.
Existing construction and reset functions remain source-compatible.

## Current limits

- Icons (`{{icon:id}}`) and emoji fields are dropped; there are no icon glyphs yet.
- Slide layers, backgrounds and motion are not drawn; slides show their
  label, body and notes with previous/next.
- Styled `block` fonts, sizes and alignment are not applied yet.
- Chart tooltips are shown as a text row under the plot.

## Example

```sh
cargo run -p liquid-view
cargo run -p liquid-view -- --export liquid.png
cargo run -p liquid_canvas_integration
```
