use rndr_canvas::text::{index_at_node, EstimateMeasure};
use rndr_canvas::*;

fn text_node(text: &str) -> Node {
    Node::new(
        "text",
        Content::Text {
            text: text.into(),
            color: "#000".into(),
            font_family: "Inter".into(),
            font_size: 20.0,
            font_weight: None,
            font_style: None,
        },
        Transform::default(),
    )
}

#[test]
fn pointer_offsets_are_bytes_not_utf16_units() {
    let node = text_node("a😀b");
    assert_eq!(
        index_at_node(&node, &EstimateMeasure, 1.2 + 24.0, 5.0),
        Some(5)
    );
}

#[test]
fn normalized_newlines_map_back_to_original_bytes() {
    let node = text_node("a\r\nb\rc");
    assert_eq!(index_at_node(&node, &EstimateMeasure, 1.2, 28.0), Some(3));
    assert_eq!(index_at_node(&node, &EstimateMeasure, 1.2, 54.0), Some(5));
}

#[test]
fn wrapped_line_hit_testing_uses_layout_starts() {
    let mut node = text_node("abcdef");
    node.text_layout = Some(TextLayout {
        width: Some(38.4),
        ..TextLayout::default()
    });
    assert_eq!(index_at_node(&node, &EstimateMeasure, 13.2, 28.0), Some(4));
}
