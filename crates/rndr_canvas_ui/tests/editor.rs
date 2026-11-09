use rndr_canvas::*;
use rndr_canvas_ui::editor::*;
use std::sync::Arc;

struct Rig {
    editor: CanvasEditor,
    time: f64,
}

impl Rig {
    fn new() -> Self {
        let store = Arc::new(MemoryStore::new());
        store
            .create(CreateRequest {
                id: "doc".into(),
                title: "Doc".into(),
                scene: Scene::default(),
            })
            .unwrap();
        Self {
            editor: CanvasEditor::open(store, "doc").unwrap(),
            time: 0.0,
        }
    }

    fn frame(
        &mut self,
        pointer: Option<PointerInput>,
        keys: Vec<(Key, Modifiers)>,
        mods: Modifiers,
    ) {
        self.time += 16.0;
        let input = EditorInput {
            pointer,
            modifiers: mods,
            keys,
            viewport: [800.0, 600.0],
            time_ms: self.time,
            ..EditorInput::default()
        };
        self.editor.handle(&input, &EstimateMeasure);
    }

    fn drag(&mut self, from: [f32; 2], to: [f32; 2], mods: Modifiers) {
        let p = |position, pressed, down, released| PointerInput {
            position,
            pressed,
            down,
            released,
            ..PointerInput::default()
        };
        self.frame(Some(p(from, true, true, false)), vec![], mods);
        let mid = [(from[0] + to[0]) / 2.0, (from[1] + to[1]) / 2.0];
        self.frame(Some(p(mid, false, true, false)), vec![], mods);
        self.frame(Some(p(to, false, true, false)), vec![], mods);
        self.frame(Some(p(to, false, false, true)), vec![], mods);
        self.time += 1000.0;
    }

    fn click(&mut self, at: [f32; 2], mods: Modifiers) {
        self.frame(
            Some(PointerInput {
                position: at,
                pressed: true,
                down: true,
                ..PointerInput::default()
            }),
            vec![],
            mods,
        );
        self.frame(
            Some(PointerInput {
                position: at,
                released: true,
                ..PointerInput::default()
            }),
            vec![],
            mods,
        );
    }

    fn key(&mut self, key: Key, mods: Modifiers) {
        self.frame(None, vec![(key, mods)], mods);
    }

    fn nodes(&self) -> Vec<Node> {
        self.editor.mutation().document.scene.nodes.clone()
    }
}

const NONE: Modifiers = Modifiers {
    shift: false,
    alt: false,
    primary: false,
};
const CMD: Modifiers = Modifiers {
    shift: false,
    alt: false,
    primary: true,
};
const SHIFT: Modifiers = Modifiers {
    shift: true,
    alt: false,
    primary: false,
};

#[test]
fn drawing_a_rectangle_is_one_undoable_step() {
    let mut rig = Rig::new();
    rig.editor.set_tool(Tool::Rectangle);
    rig.drag([10.0, 10.0], [110.0, 60.0], NONE);
    let nodes = rig.nodes();
    assert_eq!(nodes.len(), 1);
    let t = nodes[0].transform;
    assert_eq!((t.x, t.y, t.width, t.height), (10.0, 10.0, 100.0, 50.0));
    assert_eq!(rig.editor.selection(), [nodes[0].id.clone()]);
    assert_eq!(rig.editor.tool(), Tool::Select);
    assert_eq!(rig.editor.mutation().document.revision, 1);
    rig.key(Key::Char('z'), CMD);
    assert!(rig.nodes().is_empty());
    rig.key(Key::Char('z'), Modifiers { shift: true, ..CMD });
    assert_eq!(rig.nodes().len(), 1);
}

#[test]
fn select_move_with_preview_then_commit() {
    let mut rig = Rig::new();
    rig.editor.style.fill = "#ff0000".into();
    rig.editor.set_tool(Tool::Rectangle);
    rig.drag([10.0, 10.0], [60.0, 60.0], NONE);
    rig.key(Key::Escape, NONE);
    assert!(rig.editor.selection().is_empty());
    let p = |position, pressed, down, released| PointerInput {
        position,
        pressed,
        down,
        released,
        ..PointerInput::default()
    };
    rig.frame(Some(p([30.0, 30.0], true, true, false)), vec![], NONE);
    rig.frame(Some(p([130.0, 30.0], false, true, false)), vec![], NONE);
    // Mid-gesture the document is unchanged; the preview moved.
    assert_eq!(rig.nodes()[0].transform.x, 10.0);
    assert_eq!(rig.editor.nodes()[0].transform.x, 110.0);
    rig.frame(Some(p([130.0, 30.0], false, false, true)), vec![], NONE);
    assert_eq!(rig.nodes()[0].transform.x, 110.0);
    assert_eq!(rig.editor.mutation().undo_label.as_deref(), Some("Move"));
}

#[test]
fn marquee_group_duplicate_and_delete() {
    let mut rig = Rig::new();
    for x in [10.0, 100.0] {
        rig.editor.set_tool(Tool::Ellipse);
        rig.drag([x, 10.0], [x + 40.0, 50.0], NONE);
    }
    rig.key(Key::Escape, NONE);
    rig.drag([0.0, 0.0], [200.0, 100.0], NONE);
    assert_eq!(rig.editor.selection().len(), 2);
    rig.key(Key::Char('g'), CMD);
    let groups = rig
        .nodes()
        .iter()
        .filter(|n| matches!(n.content, Content::Group))
        .count();
    assert_eq!(groups, 1);
    assert_eq!(rig.editor.selection().len(), 1);
    rig.key(Key::Char('d'), CMD);
    assert_eq!(rig.nodes().len(), 6);
    rig.key(Key::Delete, NONE);
    assert_eq!(rig.nodes().len(), 3);
    let group = rig
        .nodes()
        .iter()
        .find(|n| matches!(n.content, Content::Group))
        .unwrap()
        .id
        .clone();
    rig.editor.select(&[group]);
    rig.key(Key::Char('g'), Modifiers { shift: true, ..CMD });
    assert_eq!(rig.nodes().len(), 2);
}

#[test]
fn pen_strokes_record_pressure_and_erasing_splits_them() {
    let mut rig = Rig::new();
    rig.editor.set_tool(Tool::Pen);
    let samples: Vec<Sample> = (0..=20)
        .map(|i| Sample {
            position: [10.0 + i as f32 * 10.0, 50.0],
            pressure: Some(0.7),
            tilt: None,
            time_ms: i as f64 * 8.0,
        })
        .collect();
    let pen = |pressed, down, released, samples: Vec<Sample>| PointerInput {
        position: samples.last().map_or([210.0, 50.0], |s| s.position),
        pressed,
        down,
        released,
        kind: PointerKind::Pen,
        samples,
        ..PointerInput::default()
    };
    rig.frame(
        Some(pen(true, true, false, samples[..1].to_vec())),
        vec![],
        NONE,
    );
    rig.frame(
        Some(pen(false, true, false, samples[1..].to_vec())),
        vec![],
        NONE,
    );
    rig.frame(Some(pen(false, false, true, vec![])), vec![], NONE);
    let nodes = rig.nodes();
    let Content::Ink { points, input, .. } = &nodes[0].content else {
        panic!()
    };
    assert_eq!(*input, InputKind::Pen);
    assert_eq!(points.len(), 21);
    assert_eq!(points[3].pressure, Some(0.699999988079071));
    rig.editor.set_tool(Tool::Eraser);
    rig.drag([110.0, 40.0], [110.0, 60.0], NONE);
    let inks = rig
        .nodes()
        .iter()
        .filter(|n| matches!(n.content, Content::Ink { .. }))
        .count();
    assert_eq!(inks, 2);
}

#[test]
fn typing_creates_text_and_double_click_edits_it() {
    let mut rig = Rig::new();
    rig.editor.set_tool(Tool::Text);
    rig.click([50.0, 50.0], NONE);
    assert!(rig.editor.editing_text().is_some());
    rig.editor.handle(
        &EditorInput {
            text: "Hi".into(),
            viewport: [800.0, 600.0],
            ..EditorInput::default()
        },
        &EstimateMeasure,
    );
    assert!(rig.nodes().is_empty(), "nothing is saved while typing");
    rig.key(Key::Escape, NONE);
    let nodes = rig.nodes();
    assert!(matches!(&nodes[0].content, Content::Text { text, .. } if text == "Hi"));
    rig.editor.set_tool(Tool::Select);
    let at = [
        nodes[0].transform.x as f32 + 5.0,
        nodes[0].transform.y as f32 + 10.0,
    ];
    rig.click(at, NONE);
    rig.click(at, NONE);
    assert!(rig.editor.editing_text().is_some());
    assert_eq!(rig.editor.text_selection(), Some((0, 0)));
    rig.editor.handle(
        &EditorInput {
            text: "!".into(),
            viewport: [800.0, 600.0],
            ..EditorInput::default()
        },
        &EstimateMeasure,
    );
    rig.key(Key::Escape, NONE);
    assert!(matches!(&rig.nodes()[0].content, Content::Text { text, .. } if text == "!Hi"));
}

#[test]
fn resize_handle_and_arrows_and_wheel_zoom() {
    let mut rig = Rig::new();
    rig.editor.set_tool(Tool::Rectangle);
    rig.drag([10.0, 10.0], [110.0, 110.0], NONE);
    rig.drag([110.0, 110.0], [210.0, 160.0], NONE);
    let t = rig.nodes()[0].transform;
    assert_eq!((t.width, t.height), (200.0, 150.0));
    rig.key(Key::Right, SHIFT);
    assert_eq!(rig.nodes()[0].transform.x, 20.0);
    rig.editor.set_tool(Tool::Arrow);
    rig.drag([300.0, 300.0], [400.0, 310.0], SHIFT);
    let arrow = rig.nodes().last().cloned().unwrap();
    let line = LineGeometry::of(&arrow).unwrap();
    assert!(
        line.end_arrow && (line.end.y - line.start.y).abs() < 1e-9,
        "snapped flat: {line:?}"
    );
    let before = rig.editor.camera;
    rig.editor.handle(
        &EditorInput {
            pointer: Some(PointerInput {
                position: [100.0, 100.0],
                ..PointerInput::default()
            }),
            scroll: [0.0, 50.0],
            modifiers: CMD,
            viewport: [800.0, 600.0],
            ..EditorInput::default()
        },
        &EstimateMeasure,
    );
    assert!(rig.editor.camera.zoom > before.zoom);
    let still = rig.editor.camera.view_to_scene(Vec2::new(100.0, 100.0));
    assert!((still.x - 100.0).abs() < 1e-9 && (still.y - 100.0).abs() < 1e-9);
}

#[test]
fn shapes_drawn_inside_a_frame_become_its_children() {
    let mut rig = Rig::new();
    rig.editor.set_tool(Tool::Frame);
    rig.drag([0.0, 0.0], [300.0, 300.0], NONE);
    rig.editor.set_tool(Tool::Rectangle);
    rig.drag([50.0, 50.0], [100.0, 100.0], NONE);
    let nodes = rig.nodes();
    let frame = nodes
        .iter()
        .find(|n| matches!(n.content, Content::Frame { .. }))
        .unwrap();
    let rect = nodes
        .iter()
        .find(|n| matches!(n.content, Content::Shape { .. }))
        .unwrap();
    assert_eq!(rect.parent_id.as_ref(), Some(&frame.id));
    rig.editor.toggle_locked(&frame.id);
    rig.editor.set_tool(Tool::Select);
    rig.click([75.0, 75.0], NONE);
    assert!(
        rig.editor.selection().is_empty(),
        "locked frame's child is not editable"
    );
}

#[test]
fn line_arrow_and_frame_preview_follow_the_drag() {
    for tool in [Tool::Rectangle, Tool::Line, Tool::Arrow, Tool::Frame] {
        let mut rig = Rig::new();
        rig.editor.set_tool(tool);
        let p = |position: [f32; 2], pressed| PointerInput {
            position,
            pressed,
            down: true,
            ..PointerInput::default()
        };
        rig.frame(Some(p([10.0, 10.0], true)), vec![], NONE);
        rig.frame(Some(p([60.0, 30.0], false)), vec![], NONE);
        rig.frame(Some(p([210.0, 110.0], false)), vec![], NONE);
        let draft = rig.editor.nodes();
        assert_eq!(draft.len(), 1, "{tool:?}");
        let bounds = SceneIndex::new(&draft).bounds(&draft[0]);
        assert!(bounds.width >= 199.0, "{tool:?}: {bounds:?}");
    }
}

#[test]
fn text_selection_replacement_graphemes_and_transaction_history() {
    let mut rig = Rig::new();
    rig.editor.set_tool(Tool::Text);
    rig.click([50.0, 50.0], NONE);
    let type_into = |rig: &mut Rig, text: &str| {
        rig.editor.handle(
            &EditorInput {
                text: text.into(),
                ..EditorInput::default()
            },
            &EstimateMeasure,
        );
    };
    type_into(&mut rig, "ab🇫🇷cd");
    rig.key(Key::Escape, NONE);
    let id = rig.nodes()[0].id.clone();
    assert!(rig.editor.edit_text(&id));
    rig.key(Key::Delete, NONE); // Forward delete at end is a consumed no-op.
    rig.key(Key::Left, NONE);
    rig.key(Key::Left, NONE);
    rig.key(Key::Left, SHIFT);
    assert_eq!(rig.editor.text_selection(), Some((2, 10)));
    type_into(&mut rig, "X");
    rig.key(Key::Delete, NONE);
    rig.key(Key::Escape, NONE);
    assert!(matches!(&rig.nodes()[0].content, Content::Text { text, .. } if text == "abXd"));
    assert!(rig.editor.undo());
    assert!(matches!(&rig.nodes()[0].content, Content::Text { text, .. } if text == "ab🇫🇷cd"));
    assert!(rig.editor.redo());
    assert!(matches!(&rig.nodes()[0].content, Content::Text { text, .. } if text == "abXd"));
}

#[test]
fn pointer_selection_tracks_rotated_text_through_camera_and_commits_once() {
    let mut rig = Rig::new();
    let node = Node::new(
        "text",
        Content::Text {
            text: "abcdef".into(),
            color: "#000".into(),
            font_family: "Inter".into(),
            font_size: 20.0,
            font_weight: None,
            font_style: None,
        },
        Transform {
            x: 100.0,
            y: 100.0,
            width: 100.0,
            height: 30.0,
            rotation: 30.0,
        },
    );
    assert!(rig
        .editor
        .commit("fixture", vec![Operation::put(node.clone())]));
    rig.editor.camera.zoom = 2.0;
    let point = |column: f64, editor: &CanvasEditor| {
        let nodes = editor.nodes();
        let index = SceneIndex::with_measure(&nodes, &EstimateMeasure);
        let node = index.get("text").unwrap();
        let scene = index
            .world_matrix(node)
            .apply(Vec2::new(1.2 + column * 12.0, 8.0));
        let view = editor.camera.scene_to_view(scene);
        [view.x as f32, view.y as f32]
    };
    let at = point(2.0, &rig.editor);
    rig.click(at, NONE);
    rig.click(at, NONE);
    assert_eq!(rig.editor.text_selection(), Some((2, 2)));
    let revision = rig.editor.mutation().document.revision;
    let end = point(4.0, &rig.editor);
    rig.drag(at, end, NONE);
    assert_eq!(rig.editor.text_selection(), Some((4, 2)));
    assert_eq!(rig.editor.mutation().document.revision, revision);
    rig.editor.handle(
        &EditorInput {
            text: "X".into(),
            ..EditorInput::default()
        },
        &EstimateMeasure,
    );
    rig.key(Key::Escape, NONE);
    assert!(matches!(&rig.nodes()[0].content, Content::Text { text, .. } if text == "abXef"));
    assert!(rig.editor.undo());
    assert!(matches!(&rig.nodes()[0].content, Content::Text { text, .. } if text == "abcdef"));
    assert!(rig.editor.redo());
    assert!(matches!(&rig.nodes()[0].content, Content::Text { text, .. } if text == "abXef"));
}
