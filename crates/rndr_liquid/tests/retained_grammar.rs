use rndr_liquid::{parse_fence, CheckboxProps, InputProps, LiquidBlock, SliderProps};

#[test]
fn single_fence_parser_strips_reserved_keys() {
    let Some(LiquidBlock::Actions(actions)) = parse_fence("actions", "key: save\nSave") else {
        panic!()
    };
    assert_eq!(actions.actions.len(), 1);
    assert_eq!(actions.actions[0].label, "Save");
}

#[test]
fn metadata_does_not_remove_literal_key_lines_after_the_header() {
    let Some(LiquidBlock::Code(code)) = parse_fence(
        "code",
        "key: code-id\nlang: yaml\n---\nkey: literal-value\n",
    ) else {
        panic!()
    };
    assert_eq!(code.source, "key: literal-value");
}

#[test]
fn metadata_preserves_domain_ids_and_literal_or_invalid_fences() {
    let source = "```feed\nkey: component\nid: live-data\ndatatype: text\n```\n```block\nkey: styled\nid: anchor\n---\nHello\n```\n```card\nkey: invalid\n```\n````markdown\n```actions\nkey: literal\nApply\n```\n````";
    let doc = rndr_liquid::LiquidDocument::parse(source);
    assert_eq!(doc.ids, [Some("component".into()), Some("styled".into())]);
    assert_eq!(doc.ids.len(), doc.blocks.len());
    let LiquidBlock::Feed(feed) = &doc.blocks[0] else {
        panic!()
    };
    assert_eq!(feed.feed_id, "live-data");
    let LiquidBlock::Block(styled) = &doc.blocks[1] else {
        panic!()
    };
    assert_eq!(styled.id.as_deref(), Some("anchor"));
    assert!(doc.markdown.contains("key: invalid"));
    assert!(doc.markdown.contains("key: literal"));
    let doc = rndr_liquid::LiquidDocument::parse("```checkbox\nkey: \nkey: first\nkey: ignored\n```\n```input\nkey: first\n```\n```slider\nkey: \n```");
    assert_eq!(doc.ids, [Some("first".into()), Some("first".into()), None]);
}

#[test]
fn component_keys_align_with_innermost_first_blocks_and_do_not_leak_into_body() {
    let doc = rndr_liquid::LiquidDocument::parse(
        "```report\nkey: outer\ntitle: Settings\n\n```actions\nkey: inner\nApply | intent: apply\n```\n```\n\n```checkbox\nlabel: Unkeyed\n```",
    );
    assert_eq!(doc.blocks.len(), 3);
    assert_eq!(doc.ids, [Some("inner".into()), Some("outer".into()), None]);
    let LiquidBlock::Actions(actions) = &doc.blocks[0] else {
        panic!()
    };
    assert_eq!(actions.actions.len(), 1);
    assert_eq!(actions.actions[0].label, "Apply");
    let LiquidBlock::Report(report) = &doc.blocks[1] else {
        panic!()
    };
    assert!(!report.body.contains("key:"));
    assert!(report.body.contains(&rndr_liquid::block_token(0)));
}

#[test]
fn slider_defaults_and_host_values_have_valid_ranges() {
    assert_eq!(
        parse_fence("slider", ""),
        Some(LiquidBlock::Slider(SliderProps {
            label: String::new(),
            value: 0.0,
            min: 0.0,
            max: 100.0,
            controlled: false,
        }))
    );
    assert_eq!(
        parse_fence("slider", ""),
        Some(LiquidBlock::Slider(SliderProps::default()))
    );
    let block = parse_fence(
        "slider",
        "label: Level\nmin: -5\nmax: 10\nvalue: 3.5\ncontrolled: true",
    )
    .unwrap();
    assert_eq!(block.kind(), "slider");
    assert_eq!(
        block,
        LiquidBlock::Slider(SliderProps {
            label: "Level".into(),
            value: 3.5,
            min: -5.0,
            max: 10.0,
            controlled: true,
        })
    );
    assert_eq!(
        serde_json::from_str::<LiquidBlock>(&serde_json::to_string(&block).unwrap()).unwrap(),
        block
    );
    for (body, value) in [("value: 200", 100.0), ("value: -5", 0.0), ("min: 5", 5.0)] {
        let Some(LiquidBlock::Slider(props)) = parse_fence("slider", body) else {
            panic!("{body}");
        };
        assert_eq!(props.value, value);
    }
    for body in [
        "min: 10\nmax: 10",
        "min: 101",
        "min: NaN",
        "max: inf",
        "value: -inf",
        "value: NaN",
        "value: nope",
        "min: -1e308\nmax: 1e308",
    ] {
        assert_eq!(parse_fence("slider", body), None, "{body}");
    }
}

#[test]
fn input_has_uncontrolled_defaults_and_parses_host_values() {
    assert_eq!(
        parse_fence("input", ""),
        Some(LiquidBlock::Input(InputProps::default()))
    );
    let block = parse_fence("input", "label: Name\nvalue: Ada\ncontrolled: true").unwrap();
    assert_eq!(block.kind(), "input");
    assert_eq!(
        block,
        LiquidBlock::Input(InputProps {
            label: "Name".into(),
            value: "Ada".into(),
            controlled: true,
        })
    );
    assert_eq!(
        serde_json::from_str::<LiquidBlock>(&serde_json::to_string(&block).unwrap()).unwrap(),
        block
    );
}

#[test]
fn checkbox_has_uncontrolled_defaults_and_parses_host_values() {
    assert_eq!(
        parse_fence("checkbox", ""),
        Some(LiquidBlock::Checkbox(CheckboxProps::default()))
    );
    let block = parse_fence(
        "CHECKBOX",
        "label: Remember\nchecked: true\ncontrolled: true",
    )
    .unwrap();
    assert_eq!(block.kind(), "checkbox");
    assert_eq!(
        block,
        LiquidBlock::Checkbox(CheckboxProps {
            label: "Remember".into(),
            checked: true,
            controlled: true,
        })
    );
    assert_eq!(
        serde_json::from_str::<LiquidBlock>(&serde_json::to_string(&block).unwrap()).unwrap(),
        block
    );
}
