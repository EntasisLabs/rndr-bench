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

## Host update metadata and controls

`LiquidDocument::parse_with_metadata(source)` returns a `ParsedLiquidDocument`:
`document` retains the existing `LiquidDocument` representation; `metadata` has
one `BlockMetadata { explicit_id: Option<String> }` per block in the exact same
innermost-first order; `incomplete` identifies unfinished Liquid syntax. Hosts
can keep the last accepted UI while `incomplete` is true. The flag is syntactic:
a closed fence with invalid props remains ordinary Markdown, not an incomplete
component. Ambiguous open fences and partial Liquid language names are treated
conservatively as incomplete. Ordinary backtick and tilde documentation/code
fences, including nested code examples, do not introduce live controls or IDs.

Use an unindented `id: my-control` line in the fence header, before the first
`---`. IDs are trimmed, case-sensitive strings; empty IDs are absent. These
lines are removed before props parsing, except `feed` and styled `block`, where
`id` already has public domain meaning and remains in props too. Item IDs after
`---` retain their existing parser semantics. Duplicate block IDs are exposed
unchanged: the host must reject duplicates globally rather than aliasing state.
This grammar does not choose fallback identity, reconcile entities, or own focus.

The `checkbox`, `input`, and `slider` fences carry typed `label`, `value`, and
`controlled` fields. `controlled` defaults to `false`: a renderer should treat
`value` as initial data for an uncontrolled component and as authoritative data
for a controlled component. Checkbox values are `true`/`false`; input values are
strings. Slider values are finite numbers with defaults `min: 0`, `max: 100`,
`step: 1`, and `value: min`. The parser requires `min < max` and positive `step`,
rejects invalid/nonfinite numeric input, and clamps the value into the range.
The renderer owns interaction, step snapping, and controlled/uncontrolled policy.

Existing `LiquidDocument` fields are unchanged. Adding the three `LiquidBlock`
variants requires downstream exhaustive matches to handle the new controls.

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
