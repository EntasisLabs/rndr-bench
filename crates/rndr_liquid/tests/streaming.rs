use rndr_liquid::LiquidDocument;

#[test]
fn documentation_examples_are_literal_and_nested_liquid_closes_independently() {
    for source in [
        "````markdown\n```input\nvalue: Sample\n```\n````",
        "````markdown\n```input\nAn intentionally unclosed example.\n````",
        "```markdown\n```checkbox\nchecked: true\n```\n```",
        "```report\ntitle: Nested\n```section\ntitle: Controls\n```input\n```\n```\n```",
        "```input\r\nvalue: CRLF\r\n```\r\n",
        "```input\n```\n\n```slider\n```",
    ] {
        assert!(LiquidDocument::is_complete(source), "{source:?}");
    }
    for source in [
        "````markdown\n```input\n```",
        "```markdown\n```checkbox\nchecked: true\n```",
        "```report\ntitle: Nested\n```section\ntitle: Controls\n```input\n```\n```",
        "```input\n```\n\n```slider\nvalue: 10",
    ] {
        assert!(!LiquidDocument::is_complete(source), "{source:?}");
    }
}

#[test]
fn completeness_is_not_prop_validation_or_a_restriction_on_parse() {
    assert!(LiquidDocument::is_complete("```slider\nmin: NaN\n```"));
    let source = "```input\nvalue: retained\n```\n```slider\nvalue: 10";
    assert!(!LiquidDocument::is_complete(source));
    // Explicit reset may parse a partial candidate without admission checks.
    assert_eq!(LiquidDocument::parse(source).blocks.len(), 1);
}

#[test]
fn every_recognized_language_requires_its_closing_fence() {
    for lang in rndr_liquid::FENCE_LANGS {
        let candidate = format!("```{lang}\n");
        assert!(!LiquidDocument::is_complete(&candidate), "{lang}");
        assert!(
            LiquidDocument::is_complete(&format!("{candidate}```")),
            "{lang}"
        );
    }
}

#[test]
fn streaming_waits_for_fence_closure_and_partial_tail_openers() {
    for source in [
        "Intro\n``",
        "Intro\n```inp",
        "```input\nkey: name\nvalue: Ada",
        "```checkbox\nchecked: true\n``",
        "```slider\nvalue: 10\n",
        "```report\ntitle: Settings\n```input\nvalue: Ada\n```",
    ] {
        assert!(!LiquidDocument::is_complete(source), "{source:?}");
    }
    for source in [
        "",
        "Ordinary **Markdown** and `inline code`.",
        "```input\nkey: name\nvalue: Ada\n```",
        "```checkbox\nchecked: true\n```\n",
        "```slider\nvalue: 10\n```\nMore prose",
        "```report\ntitle: Settings\n```input\nvalue: Ada\n```\n```",
    ] {
        assert!(LiquidDocument::is_complete(source), "{source:?}");
    }
}
