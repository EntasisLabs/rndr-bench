use rndr_canvas::*;
use rndr_canvas_ui::editor::*;
use std::sync::Arc;

fn editor(text: &str) -> CanvasEditor {
    let store = Arc::new(MemoryStore::new());
    let node = Node::new(
        "text",
        Content::Text {
            text: text.into(),
            color: "#000000".into(),
            font_family: "bitmap".into(),
            font_size: 20.0,
            font_weight: None,
            font_style: None,
        },
        Transform {
            x: 30.0,
            y: 40.0,
            width: 200.0,
            height: 30.0,
            rotation: 0.0,
        },
    );
    store
        .create(CreateRequest {
            id: "doc".into(),
            title: "Text".into(),
            scene: Scene {
                nodes: vec![node],
                ..Scene::default()
            },
        })
        .unwrap();
    let mut editor = CanvasEditor::open(store, "doc").unwrap();
    assert!(editor.edit_text("text"));
    editor
}
fn key(editor: &mut CanvasEditor, key: Key, modifiers: Modifiers) {
    editor.handle(
        &EditorInput {
            keys: vec![(key, modifiers)],
            ..EditorInput::default()
        },
        &EstimateMeasure,
    );
}
fn typed(editor: &mut CanvasEditor, text: &str) {
    editor.handle(
        &EditorInput {
            text: text.into(),
            ..EditorInput::default()
        },
        &EstimateMeasure,
    );
}
fn text(editor: &CanvasEditor) -> String {
    let nodes = editor.nodes();
    let Content::Text { text, .. } = &nodes[0].content else {
        panic!()
    };
    text.clone()
}
fn view_point(editor: &CanvasEditor, byte: usize) -> [f32; 2] {
    let nodes = editor.nodes();
    let node = &nodes[0];
    let engine = rndr_core::text_engine(&rndr_core::World::new());
    let style = rndr_core::TextStyle {
        family: "bitmap".into(),
        size: 20.0,
        line_height: Some(1.3),
        ..rndr_core::TextStyle::default()
    };
    let layout = engine.layout(&text(editor), &style, &[], f32::INFINITY);
    let (x, y) = layout.caret_xy(byte);
    let local = Vec2::new(f64::from(x + 1.2), f64::from(y + 5.0));
    let scene = SceneIndex::new(&nodes).world_matrix(node).apply(local);
    let point = editor.camera.scene_to_view(scene);
    [point.x as f32, point.y as f32]
}
fn pointer(
    editor: &mut CanvasEditor,
    position: [f32; 2],
    pressed: bool,
    down: bool,
    released: bool,
) {
    editor.handle(
        &EditorInput {
            pointer: Some(PointerInput {
                position,
                pressed,
                down,
                released,
                ..PointerInput::default()
            }),
            ..EditorInput::default()
        },
        &EstimateMeasure,
    );
}
#[test]
fn pointer_caret_and_drag_selection_follow_rotated_zoomed_text() {
    let mut editor = editor("abcdef");
    editor.finish_text();
    let mut node = editor.nodes()[0].clone();
    node.transform.rotation = 32.0;
    editor.commit("Rotate", vec![Operation::put(node)]);
    assert!(editor.edit_text("text"));
    editor.camera = Camera {
        zoom: 2.3,
        pan_x: 90.0,
        pan_y: -40.0,
    };
    let from = view_point(&editor, 2);
    let to = view_point(&editor, 4);
    pointer(&mut editor, from, true, true, false);
    pointer(&mut editor, to, false, true, false);
    pointer(&mut editor, to, false, false, true);
    typed(&mut editor, "!");
    assert_eq!(text(&editor), "ab!ef");
    assert_eq!(editor.mutation().document.revision, 1);
}
#[test]
fn native_clipboard_and_ime_use_host_resources_once() {
    use rndr_canvas_ui::view::{self, CanvasView};
    use rndr_core::{
        Clipboard, Focus, Input, KeyEvent, Layout, NamedKey, Platform, Rect, SurfaceSlot, World,
    };
    let mut world = World::new();
    let slot = world.spawn((
        Layout::column(),
        SurfaceSlot { id: 9 },
        Rect {
            x: 0.0,
            y: 0.0,
            w: 800.0,
            h: 600.0,
        },
    ));
    view::install(&mut world, CanvasView::new(editor("abc"), 9));
    world.insert_resource(Focus { entity: Some(slot) });
    world.insert_resource(Platform::Mac);
    let cmd = rndr_core::Modifiers {
        super_key: true,
        ..rndr_core::Modifiers::new()
    };
    world.insert_resource(Input {
        keys: vec![KeyEvent::character('a', cmd), KeyEvent::character('c', cmd)],
        ..Input::default()
    });
    view::update(&mut world);
    assert_eq!(world.resource::<Clipboard>().unwrap().text, "abc");
    assert!(world.resource::<Clipboard>().unwrap().dirty);
    assert!(world.resource::<Input>().unwrap().keys.is_empty());
    world.insert_resource(Input {
        keys: vec![KeyEvent::character('x', cmd)],
        ..Input::default()
    });
    view::update(&mut world);
    assert_eq!(text(&world.resource::<CanvasView>().unwrap().editor), "");
    world.insert_resource(Input {
        keys: vec![KeyEvent::character('v', cmd)],
        ..Input::default()
    });
    view::update(&mut world);
    assert_eq!(text(&world.resource::<CanvasView>().unwrap().editor), "abc");
    world.insert_resource(Input {
        ime_enabled: true,
        ime_preedit: "仮".into(),
        keys: vec![KeyEvent::named(
            NamedKey::Backspace,
            rndr_core::Modifiers::new(),
        )],
        ..Input::default()
    });
    view::update(&mut world);
    assert_eq!(text(&world.resource::<CanvasView>().unwrap().editor), "abc");
    let view = world.resource::<CanvasView>().unwrap();
    assert!(
        matches!(&view.editor.scene().nodes[0].content, Content::Text { text, .. } if text == "abc仮"),
        "preedit must be visible but not committed"
    );
    assert!(view.editor.text_geometry(&EstimateMeasure).is_some());
    world.insert_resource(Input {
        ime_enabled: true,
        ime_commit: "確".into(),
        keys: vec![KeyEvent::character('x', rndr_core::Modifiers::new())],
        chars: "x".into(),
        ..Input::default()
    });
    view::update(&mut world);
    assert_eq!(
        text(&world.resource::<CanvasView>().unwrap().editor),
        "abc確"
    );
    // Finish is chronological: text before the exit key must reach CanvasStore.
    world.insert_resource(Input {
        keys: vec![
            KeyEvent::character('!', rndr_core::Modifiers::new()),
            KeyEvent::named(NamedKey::Escape, rndr_core::Modifiers::new()),
        ],
        ..Input::default()
    });
    view::update(&mut world);
    assert_eq!(
        text(&world.resource::<CanvasView>().unwrap().editor),
        "abc確!"
    );
    assert_eq!(
        world
            .resource::<CanvasView>()
            .unwrap()
            .editor
            .editing_text(),
        None
    );
    world.insert_resource(Input {
        keys: vec![KeyEvent::character('z', cmd)],
        ..Input::default()
    });
    view::update(&mut world);
    assert_eq!(text(&world.resource::<CanvasView>().unwrap().editor), "abc");
    assert!(world
        .resource_mut::<CanvasView>()
        .unwrap()
        .editor
        .edit_text("text"));
    world.insert_resource(Input {
        keys: vec![
            KeyEvent::character('!', rndr_core::Modifiers::new()),
            KeyEvent::character('z', cmd),
        ],
        ..Input::default()
    });
    view::update(&mut world);
    assert_eq!(text(&world.resource::<CanvasView>().unwrap().editor), "abc");
    assert_eq!(
        world
            .resource::<CanvasView>()
            .unwrap()
            .editor
            .editing_text(),
        None
    );
}

#[test]
fn canvas_focus_owns_native_keys_and_releases_them_to_other_controls() {
    use rndr_canvas_ui::view::{self, CanvasView};
    use rndr_core::{
        CapturesKeys, Focus, Focusable, Input, KeyEvent, Layout, Rect, SurfaceSlot, World,
    };
    let mut world = World::new();
    let slot = world.spawn((
        Layout::column(),
        SurfaceSlot { id: 9 },
        Rect {
            x: 0.0,
            y: 0.0,
            w: 800.0,
            h: 600.0,
        },
    ));
    view::install(&mut world, CanvasView::new(editor("abc"), 9));
    world.insert_resource(Focus { entity: Some(slot) });
    view::update(&mut world);
    let focused = world.resource::<Focus>().unwrap().entity.unwrap();
    assert!(
        world.has::<CapturesKeys>(focused),
        "native runner must enable IME"
    );
    let caret = world
        .get::<rndr_core::Caret>(focused)
        .expect("native IME candidate anchor");
    let layout = world
        .get::<rndr_core::TextLayout>(focused)
        .expect("transformed candidate geometry");
    let (x, y) = layout.caret_xy(caret.index);
    let geometry = world
        .resource::<CanvasView>()
        .unwrap()
        .editor
        .text_geometry(&EstimateMeasure)
        .unwrap();
    let expected = rndr_canvas::Rect::around(geometry.caret).unwrap();
    assert!((f64::from(x + rndr_core::TEXT_ORIGIN_X) - expected.x).abs() < 0.001);
    assert!((f64::from(y + rndr_core::TEXT_ORIGIN_Y) - expected.y).abs() < 0.001);
    let other = world.spawn(Focusable);
    world.insert_resource(Focus {
        entity: Some(other),
    });
    let event = KeyEvent::character('p', rndr_core::Modifiers::new());
    world.insert_resource(Input {
        keys: vec![event.clone()],
        chars: "p".into(),
        ..Input::default()
    });
    view::update(&mut world);
    assert_eq!(world.resource::<Input>().unwrap().keys, vec![event]);
    assert_eq!(text(&world.resource::<CanvasView>().unwrap().editor), "abc");
    assert_eq!(
        world
            .resource::<CanvasView>()
            .unwrap()
            .editor
            .editing_text(),
        None
    );
    assert_eq!(world.resource::<Focus>().unwrap().entity, Some(other));
}

#[test]
fn text_overlay_draws_caret_and_selection_instead_of_object_handles() {
    use rndr_canvas_ui::overlay::{paint_overlay, OverlayTheme};
    let mut editor = editor("abcdef");
    editor.select(&["text".into()]);
    let paint = |editor: &CanvasEditor| {
        let mut canvas = rndr_paint::Canvas::new();
        paint_overlay(
            &mut canvas,
            editor,
            &EstimateMeasure,
            &OverlayTheme::default(),
        );
        canvas.into_scene().commands
    };
    assert_eq!(
        paint(&editor).len(),
        2,
        "text outline and caret, not resize handles"
    );
    key(
        &mut editor,
        Key::Left,
        Modifiers {
            shift: true,
            ..Modifiers::default()
        },
    );
    let commands = paint(&editor);
    assert_eq!(
        commands.len(),
        3,
        "text outline, selection highlight and caret"
    );
    assert!(commands
        .iter()
        .any(|op| matches!(op, rndr_paint::Op::Path(p) if p.fill.is_some())));
}

#[test]
fn undo_during_editing_commits_then_undoes_the_canvas_transaction() {
    let mut editor = editor("abc");
    typed(&mut editor, "!");
    key(
        &mut editor,
        Key::Char('z'),
        Modifiers {
            primary: true,
            ..Modifiers::default()
        },
    );
    assert_eq!(text(&editor), "abc");
    assert_eq!(editor.editing_text(), None);
    assert!(editor.mutation().can_redo);
    key(
        &mut editor,
        Key::Char('z'),
        Modifiers {
            primary: true,
            shift: true,
            ..Modifiers::default()
        },
    );
    assert_eq!(text(&editor), "abc!");
    key(&mut editor, Key::Char('r'), Modifiers::default());
    assert_eq!(editor.tool(), Tool::Rectangle);
}

#[test]
fn locked_text_refuses_editing_and_reload_discards_the_draft() {
    let mut editor = editor("abc");
    typed(&mut editor, "!");
    editor.reload().unwrap();
    assert_eq!(editor.editing_text(), None);
    assert_eq!(text(&editor), "abc");
    assert!(editor.toggle_locked("text"));
    assert!(!editor.edit_text("text"));
    assert_eq!(editor.editing_text(), None);
}

#[test]
fn native_pointer_uses_the_same_builtin_engine_as_paint_without_an_installed_engine() {
    use rndr_canvas_ui::view::{self, CanvasView};
    use rndr_core::{Focus, Input, Layout, Pointer, Rect, SurfaceSlot, World};
    let editor = editor("abcdefghij");
    let point = view_point(&editor, 8);
    let mut world = World::new();
    let slot = world.spawn((
        Layout::column(),
        SurfaceSlot { id: 9 },
        Rect {
            x: 0.0,
            y: 0.0,
            w: 800.0,
            h: 600.0,
        },
    ));
    view::install(&mut world, CanvasView::new(editor, 9));
    world.insert_resource(Focus { entity: Some(slot) });
    world.insert_resource(Input {
        pointer: Pointer {
            position: point,
            pressed: true,
            down: true,
            ..Pointer::default()
        },
        ..Input::default()
    });
    view::update(&mut world);
    let view = world.resource::<CanvasView>().unwrap();
    assert_eq!(view.editor.editing_text(), Some("text"));
    assert_eq!(view.editor.text_selection(), Some((8, 8)));
}

#[test]
fn a_stale_text_undo_does_not_undo_another_editors_transaction() {
    let store = Arc::new(MemoryStore::new());
    let initial = editor("abc").mutation().document.scene.clone();
    store
        .create(CreateRequest {
            id: "doc".into(),
            title: "Doc".into(),
            scene: initial,
        })
        .unwrap();
    let mut local = CanvasEditor::open(store.clone(), "doc").unwrap();
    let mut remote = CanvasEditor::open(store, "doc").unwrap();
    assert!(local.edit_text("text"));
    typed(&mut local, "!");
    remote.select(&["text".into()]);
    assert!(remote.nudge(25.0, 0.0));
    assert!(!local.undo(), "conflict must stop before history traversal");
    assert!(matches!(local.last_error(), Some(CanvasError::Conflict(_))));
    assert_eq!(local.nodes()[0].transform.x, 55.0);
    assert_eq!(text(&local), "abc");
    assert_eq!(local.editing_text(), None);
}

#[test]
fn wrapped_vertical_motion_matches_painted_width_and_ancestor_rotation() {
    let mut editor = editor("abcdefghij");
    editor.finish_text();
    let mut node = editor.nodes()[0].clone();
    node.text_layout = Some(TextLayout {
        width: Some(70.0),
        line_height: 1.3,
        ..TextLayout::default()
    });
    node.parent_id = Some("group".into());
    let group = Node::new(
        "group",
        Content::Group,
        Transform {
            x: 80.0,
            y: 90.0,
            width: 300.0,
            height: 200.0,
            rotation: -25.0,
        },
    );
    editor.commit(
        "Wrap and parent",
        vec![Operation::put(group), Operation::put(node)],
    );
    assert!(editor.edit_text("text"));
    key(&mut editor, Key::Up, Modifiers::default());
    assert_eq!(editor.text_selection(), Some((7, 7)));
    let nodes = editor.nodes();
    let engine = rndr_core::text_engine(&rndr_core::World::new());
    let style = rndr_core::TextStyle {
        family: "bitmap".into(),
        size: 20.0,
        line_height: Some(1.3),
        ..rndr_core::TextStyle::default()
    };
    let layout = engine.layout("abcdefghij", &style, &[], 67.6);
    let (x, y) = layout.caret_xy(7);
    let expected = SceneIndex::new(&nodes)
        .world_matrix(&nodes[0])
        .apply(Vec2::new(f64::from(x + 1.2), f64::from(y + 1.2)));
    let geometry = editor.text_geometry(&EstimateMeasure).unwrap();
    assert!(geometry.caret[0].distance(expected) < 0.001);
    let (x, y) = layout.caret_xy(4);
    let scene = SceneIndex::new(&nodes)
        .world_matrix(&nodes[0])
        .apply(Vec2::new(f64::from(x + 1.2), f64::from(y + 5.0)));
    let view = editor.camera.scene_to_view(scene);
    let point = [view.x as f32, view.y as f32];
    pointer(&mut editor, point, true, true, false);
    pointer(&mut editor, point, false, false, true);
    assert_eq!(editor.text_selection(), Some((4, 4)));
}

#[test]
fn arbitrary_keyboard_selection_replaces_unicode_without_canvas_commands() {
    let mut editor = editor("a👩‍💻bc");
    key(&mut editor, Key::Left, Modifiers::default());
    key(
        &mut editor,
        Key::Left,
        Modifiers {
            shift: true,
            ..Modifiers::default()
        },
    );
    typed(&mut editor, "R");
    assert_eq!(text(&editor), "a👩‍💻Rc");
    key(&mut editor, Key::Backspace, Modifiers::default());
    key(&mut editor, Key::Backspace, Modifiers::default());
    assert_eq!(text(&editor), "ac");
    assert_eq!(editor.mutation().document.revision, 0);
    assert_eq!(editor.tool(), Tool::Select);
    assert!(editor.finish_text());
    assert_eq!(editor.mutation().document.revision, 1);
    editor.select(&["text".into()]);
    assert!(editor.nudge(10.0, 0.0));
    assert!(editor.undo());
    assert_eq!(editor.nodes()[0].transform.x, 30.0);
    assert!(editor.undo());
    assert_eq!(text(&editor), "a👩‍💻bc");
    assert!(editor.redo());
    assert_eq!(text(&editor), "ac");
    assert!(editor.redo());
    assert_eq!(editor.nodes()[0].transform.x, 40.0);
}
