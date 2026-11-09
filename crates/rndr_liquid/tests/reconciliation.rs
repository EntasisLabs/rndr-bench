use rndr_liquid::{parse_fence, LiquidBlock, LiquidDocument};

#[test]
fn metadata_tracks_nested_block_indices_and_removes_ids_from_content() {
    let parsed = LiquidDocument::parse_with_metadata(
        "```report\nid: outer\ntitle: Status\nOpening\n```input\nid: inner\nlabel: Name\nvalue: Ada\n```\nClosing\n```",
    );
    assert!(!parsed.incomplete);
    assert_eq!(parsed.document.blocks.len(), 2);
    assert_eq!(parsed.metadata.len(), 2);
    assert_eq!(parsed.metadata[0].explicit_id.as_deref(), Some("inner"));
    assert_eq!(parsed.metadata[1].explicit_id.as_deref(), Some("outer"));
    let LiquidBlock::Input(input) = &parsed.document.blocks[0] else {
        panic!()
    };
    assert_eq!(input.value, "Ada");
    let LiquidBlock::Report(report) = &parsed.document.blocks[1] else {
        panic!()
    };
    assert!(!report.body.contains("id:"));
}

#[test]
fn ids_survive_markdown_insertion_and_reordering_without_silent_deduplication() {
    let a = "```checkbox\nid: a\nlabel: Check\n```";
    let b = "```slider\nid: b\nvalue: 25\n```";
    for (source, expected) in [
        (format!("{a}\n{b}"), vec!["a", "b"]),
        (
            format!("Streaming intro\n{b}\nBetween\n{a}"),
            vec!["b", "a"],
        ),
        (format!("{a}\n{a}"), vec!["a", "a"]),
    ] {
        let parsed = LiquidDocument::parse_with_metadata(&source);
        assert!(!parsed.incomplete);
        assert_eq!(
            parsed
                .metadata
                .iter()
                .map(|m| m.explicit_id.as_deref().unwrap())
                .collect::<Vec<_>>(),
            expected
        );
    }
    let no_id = LiquidDocument::parse_with_metadata("```input\nlabel: Name\n```");
    assert_eq!(no_id.metadata[0].explicit_id, None);
}

#[test]
fn incomplete_nested_fences_are_reported_until_outer_close() {
    let open = "```report\nid: parent\ntitle: Status\n```checkbox\nid: child\nvalue: true\n```";
    assert!(LiquidDocument::parse_with_metadata(open).incomplete);
    assert!(!LiquidDocument::parse_with_metadata(&format!("{open}\n```")).incomplete);
    assert!(LiquidDocument::parse_with_metadata("```input\nid: typing\nvalue: partial").incomplete);
    assert!(LiquidDocument::parse_with_metadata("```check").incomplete);
    assert!(LiquidDocument::parse_with_metadata("```").incomplete);
    assert!(LiquidDocument::parse_with_metadata("```check\nid: not_finished").incomplete);
    assert!(LiquidDocument::parse_with_metadata("Some Markdown\n``").incomplete);
}

#[test]
fn documentation_code_is_opaque_to_streaming_and_identity() {
    for source in [
        "````markdown\n```input\nid: example\nvalue: unfinished\n````",
        "```rust\n// ```input is documentation\n```",
        "````markdown\n```checkbox\nid: example\n```\n````",
        "~~~markdown\n```input\nid: example\nvalue: unfinished\n~~~",
        "~~~markdown\n```checkbox\nid: example\n```\n~~~",
        "```rust\n```input\nid: hidden\nvalue: text\n```",
        "````rust\n```input\nid: hidden\nvalue: text\n```",
        "~~~markdown\n``",
    ] {
        let parsed = LiquidDocument::parse_with_metadata(source);
        assert!(!parsed.incomplete, "{source}");
        assert!(parsed.metadata.is_empty(), "{source}");
        assert!(parsed.document.blocks.is_empty(), "{source}");
    }
}

#[test]
fn nested_ordinary_code_stays_opaque_and_legacy_domain_ids_survive() {
    let source = "```report\nid: outer\ntitle: Demo\n````rust\n```input\nid: hidden\nvalue: text\n```\n````\n```";
    let parsed = LiquidDocument::parse_with_metadata(source);
    assert!(!parsed.incomplete);
    assert_eq!(parsed.document.blocks.len(), 1);
    assert_eq!(parsed.metadata[0].explicit_id.as_deref(), Some("outer"));
    let LiquidBlock::Report(report) = &parsed.document.blocks[0] else {
        panic!()
    };
    assert!(report.body.contains("id: hidden"));
    let feed = LiquidDocument::parse_with_metadata("```feed\nid: weekly\ndatatype: md\n```");
    assert_eq!(feed.metadata[0].explicit_id.as_deref(), Some("weekly"));
    assert!(matches!(&feed.document.blocks[0], LiquidBlock::Feed(_)));
    let tabs = LiquidDocument::parse_with_metadata(
        "```tabs\nid: tabs\n---\nid: one\nlabel: First\nbody: content\n---\nlabel: Second\nbody: more\n```",
    );
    assert_eq!(tabs.metadata[0].explicit_id.as_deref(), Some("tabs"));
    let LiquidBlock::Tabs(tabs) = &tabs.document.blocks[0] else {
        panic!()
    };
    // Tab panels retain their existing inferred identity policy.
    assert_eq!(tabs.panels[0].id, "tab-first");
    assert_eq!(tabs.panels.len(), 2);
}

#[test]
fn typed_control_values_defaults_validation_and_serde() {
    let Some(LiquidBlock::Checkbox(c)) = parse_fence("checkbox", "label: Ready\nvalue: true")
    else {
        panic!()
    };
    assert!(c.value);
    assert!(!c.controlled);
    let Some(LiquidBlock::Input(i)) =
        parse_fence("input", "label: Name\nvalue: Ada\ncontrolled: true")
    else {
        panic!()
    };
    assert_eq!(i.value, "Ada");
    assert!(i.controlled);
    let Some(LiquidBlock::Slider(s)) =
        parse_fence("slider", "min: 10\nmax: 20\nvalue: 50\nstep: 0.5")
    else {
        panic!()
    };
    assert_eq!((s.min, s.max, s.step, s.value), (10.0, 20.0, 0.5, 20.0));
    assert!(!s.controlled);
    for body in [
        "min: 20\nmax: 10",
        "step: 0",
        "value: NaN",
        "min: -inf",
        "step: nope",
    ] {
        assert!(parse_fence("slider", body).is_none(), "{body}");
    }
    assert!(parse_fence("checkbox", "value: maybe").is_none());
    assert!(parse_fence("input", "controlled: maybe").is_none());
    for source in [
        "```checkbox\nvalue: true\n```",
        "```input\nvalue: text\n```",
        "```slider\nvalue: 12\n```",
    ] {
        let block = LiquidDocument::parse(source).blocks.remove(0);
        let json = serde_json::to_string(&block).unwrap();
        assert_eq!(serde_json::from_str::<LiquidBlock>(&json).unwrap(), block);
    }
}
