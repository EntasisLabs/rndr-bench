# rndr_liquid

Liquid Markdown for rndr: paste-first UI blocks written as ordinary fenced
code. A fence whose language names a component (` ```card `, ` ```chart `,
` ```plan `, ` ```tabs `, …) and whose body is forgiving `key: value` text,
`---`-separated items, or a GFM table becomes a typed block. Anything that
does not parse stays a code fence, so documents always degrade to readable
Markdown.

This crate is the headless grammar only: no Markdown engine, renderer,
navigation or network access. Intents, feed ids, image paths and URLs are
carried as data for the host to act on.

```rust
use rndr_liquid::{LiquidBlock, LiquidDocument, Segment};

let doc = LiquidDocument::parse("```callout\ntone: tip\nbody: Ship it\n```\n");
for segment in doc.segments() {
    match segment {
        Segment::Markdown(md) => { /* hand to the Markdown renderer */ }
        Segment::Block(i) => match &doc.blocks[i] {
            LiquidBlock::Callout(c) => assert_eq!(c.body, "Ship it"),
            other => unreachable!("{}", other.kind()),
        },
    }
}
```

## Fences

`card` `carousel` `actions` (`action_row`) `callout` `section` `block`
`chips` (`chip_group`) `media` `feed` `cite` `compare` `plan` `timeline`
`shortlist` `decision` `brief` `dashboard` `chart` `report` `slides` `tabs`
`steps` `recipe` (`procedure`) `accordion` `code` `tree` `kanban`
`checkbox` `input` `slider`

Charts cover `bar`, `line`, `area`, `pie`, `donut`, `radar`, `radial`,
`scatter`, `combo` and `heatmap`. Inline `{{icon:name}}` shortcodes are
limited to [`ICON_ALLOWLIST`](src/icons.rs) and canonicalized.

## Document model

`LiquidDocument::parse` resolves fences innermost-first, so a ` ```chart `
inside a ` ```report ` becomes its own block and the report body holds a
reference to it. Each block is replaced by a line `\u{E001}N\u{E001}`;
`segments` splits Markdown (including block bodies) into text and block
references. Non-Liquid documentation fences are left untouched, so
` ```markdown ` examples of Liquid stay literal.

Every block serializes with serde as `{"kind": …, "props": {…}}` using the
same camelCase property names as the reference TypeScript implementation.
`tests/medousa_golden.rs` checks parity against that implementation's output.

## Explicit component identity

Every Liquid fence accepts an unindented, case-sensitive `key: ...` metadata
line. `key:` is reserved universally, including on controls; **`id:` is not
component metadata**. Existing domain IDs (feed IDs, styled-block anchors,
accordion item IDs) retain their original meanings.

````text
```input
key: customer-name
label: Name
value: Ada
controlled: false
```
````

`LiquidDocument::ids: Vec<Option<String>>` is aligned with `blocks`, including
nested blocks in innermost-first creation order. Reserved `key:` lines are
removed before parsing a block's props, not displayed as body content. Values
are trimmed but otherwise opaque; the first nonempty key in a fence wins, and
an empty key means no explicit identity. Duplicate keys across fences are
preserved, **not made unique**: the host reconciliation layer must reject or
otherwise handle duplicates rather than silently aliasing state. Invalid
fences remain literal Markdown, including their metadata. Documentation fences
remain literal too. `parse_fence` also strips reserved keys but returns only
props; use `LiquidDocument::parse` to obtain component identity.

Adding `ids` requires callers constructing `LiquidDocument` with a struct literal
to supply that field (or use `..Default::default()`). New `LiquidBlock` variants
also require updates to exhaustive matches.

## Controls and host values

The new fences use ordinary `label:`, `checked:` / `value:`, and `controlled:`
fields, producing public root-exported payloads:

- `CheckboxProps { label: String, checked: bool, controlled: bool }`
- `InputProps { label: String, value: String, controlled: bool }`
- `SliderProps { label: String, value: f64, min: f64, max: f64, controlled: bool }`

Labels default to empty, checkbox values to false, and input values to empty.
Slider defaults are `min: 0`, `max: 100`, `value: 0`. Slider parsing refuses
non-numeric/nonfinite bounds or values, and bounds must satisfy `min < max`.
Finite out-of-range values (including the default value) are clamped. Refused
blocks remain ordinary fenced code. These checks apply to fence parsing;
direct Rust construction and serde decoding do not validate slider ranges.

`controlled` defaults to false. It declares the renderer's update policy:
uncontrolled values initialize newly mounted state without replacing retained
local edits; controlled values are supplied by the host on each admitted
update. This headless crate only parses that policy; it does not own control
state or reconciliation. Canonical `kind()` names are `checkbox`, `input`, and
`slider`, also used as serde tags.

## Streaming admission

`LiquidDocument::is_complete(source: &str) -> bool` conservatively checks fence
closure, independently of prop validity. It understands same-delimiter nested
Liquid fences and longer documentation fences shielding shorter example
fences. An unclosed fence (including ordinary/documentation fences) or trailing
partial opener such as two backticks or ` ```inp ` returns false. This is not a
general Markdown validator: it follows the grammar's unindented backtick-fence
syntax, not tilde or indented fences.

A host can buffer the **whole candidate document** while false, retaining the
last admitted interactive UI, then parse and reconcile once true. Explicit
replacement/reset may bypass admission: `parse` remains permissive and does
not call `is_complete` itself.
