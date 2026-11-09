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

## Incremental host updates

Use `update_liquid_source(world, view, &whole_source_snapshot)` for ordinary
streaming and host updates. `Applied` accepts the source; `Deferred` conservatively
holds the entire last valid UI when a Liquid fence/opener is unfinished. Submit
more of the complete snapshot later. Duplicate explicit IDs return
`LiquidUpdateError::DuplicateId` before mutation. `MissingView` indicates a stale
or wrong target. Complete but malformed props retain the parser's existing
fallback behavior; this is not a full source validation service.

An unindented fence-header `id:` establishes case-sensitive, document-scoped
identity, including nested blocks. The same ID and kind retain owned content
across Markdown insertions and component reordering. Removal or a kind change
retires incompatible content. Initial/reset constructors remain infallible for
compatibility. Duplicate occurrences render independently, but only the first
occurrence owns the public ID and can retain state on a later unique-ID update.
Later duplicates retire through the normal cleanup boundary (including focus)
rather than aliasing the surviving control. Incremental duplicate submissions
still refuse atomically; validate initial data with
`LiquidDocument::parse_with_metadata` when rejection is desired. Legacy
feed/styled-block IDs remain props as well as metadata.

Anonymous blocks match by kind and ordinal *among anonymous blocks of that kind*.
Markdown and other-kind insertions are safe; insertion/removal/reordering of
same-kind anonymous siblings can transfer local state to the ordinal occupant.
Use explicit IDs for dynamic collections. Changing a non-control payload rebuilds
its presentation while keeping compatible block state; this does not invent
stable application IDs for internal rows or panels.

Liquid owns the underlying Markdown projection: do not independently mutate its
`MarkdownView`. Core MarkdownSlot roots are detached/recreated, never reparented;
only host-owned content moves into new slots. Nested views share the outer block
table and claim retained descendants through the same slot lifecycle. Normal
Markdown frame synchronization settles nested mounts. Pending source, slot bindings
and post-layout host measurements keep `App::wants_frame` and `App::wants_update`
true until published, so event-driven runners settle resets and nested content
without another input event or a fixed number of extra frames. Removal/kind change clears
canonical interaction targets and despawns the owned subtree. Component-owned
RAII subscriptions/resources drop with it; externally retained host resources
remain the host's responsibility. Drop-counted regression leases test retirement.
`set_liquid_source` remains the deliberate whole-document reset API.

### Stateful controls

The new `checkbox`, `input` and `slider` fences accept `id`, `label`, `value`,
and `controlled` (default **false**). For example:

````text
```input
id: draft
label: Name
value: Initial text
```

```checkbox
id: accepted
label: Accepted
value: false
```

```slider
id: volume
label: Volume
min: 0
max: 100
step: 1
value: 30
controlled: true
```
````

Uncontrolled `value` initializes local state only. Host updates change labels and
bounds without replacing user drafts; slider bounds always constrain its value.
In controlled mode every accepted snapshot applies host `value`. User drafts stay
visible between submissions, so the host can accept/reject/transform requests.
Identical controlled text preserves selection/composition; a differing host value
resets incompatible text history/caret. Host submissions emit no user change event.

Resolve IDs with `LiquidView::control(world, id)` and read `control_value` for the
current `LiquidValue`. `LiquidControlChanged` is a normal typed rndr event carrying
the canonical editor entity and the user draft. Retained entities remain stable
through reordering; entity IDs are not application identities after retirement.
`LiquidView::block` exposes top-level content roots, not core MarkdownSlot roots.

Text input uses core TextField editing, selection, clipboard and composition.
Checkbox uses `rndr_ui::selection::Checkbox`; slider uses canonical admission,
focus/key-navigation, pointer dragging, arrows, Home and End. The slider currently
has a named button-like accessibility surface, **not** a certified native range
role/SetValue contract. Native IME, clipboard and assistive technology require
platform testing; headless tests establish only their state/protocol behavior.

The integrated fixture exercises both update-safe Liquid and interactive canvas
text in one host: `cargo run -p liquid_canvas_integration`. See its
[README](../../examples/liquid_canvas_integration/README.md) for headless/export
commands and a separate native acceptance checklist.

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
```
