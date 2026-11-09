use rndr_canvas::*;
use rndr_canvas_ui::editor::{CanvasEditor, EditorInput, Key, Modifiers, PointerInput};
use rndr_canvas_ui::paint::EngineMeasure;
use rndr_canvas_ui::view::{self, CanvasView};
use rndr_core::{
    BuiltinText, Clipboard, Focus, Input, KeyCode, KeyEvent, Layout, NamedKey, Platform,
    Rect as UiRect, SurfaceSlot, TextEngine, TextStyle, World,
};
use std::sync::Arc;

fn rig(text: &str) -> (CanvasEditor, Arc<dyn TextEngine>) {
    let store = Arc::new(MemoryStore::new());
    let node = Node::new(
        "text",
        Content::Text {
            text: text.into(),
            color: "#111111".into(),
            font_family: "sans".into(),
            font_size: 24.0,
            font_weight: None,
            font_style: None,
        },
        Transform {
            x: 100.0,
            y: 80.0,
            width: 300.0,
            height: 60.0,
            rotation: 0.0,
        },
    );
    store
        .create(CreateRequest {
            id: "doc".into(),
            title: "Doc".into(),
            scene: Scene {
                nodes: vec![node],
                ..Scene::default()
            },
        })
        .unwrap();
    let mut editor = CanvasEditor::open(store, "doc").unwrap();
    let engine: Arc<dyn TextEngine> = Arc::new(BuiltinText::new());
    editor.set_text_engine(engine.clone());
    (editor, engine)
}

fn text(editor: &CanvasEditor) -> String {
    let nodes = editor.nodes();
    let Content::Text { text, .. } = &nodes.iter().find(|n| n.id == "text").unwrap().content else {
        panic!()
    };
    text.clone()
}

fn frame(editor: &mut CanvasEditor, key: Key, shift: bool) {
    editor.handle(
        &EditorInput {
            keys: vec![(
                key,
                Modifiers {
                    shift,
                    ..Modifiers::default()
                },
            )],
            ..EditorInput::default()
        },
        &EstimateMeasure,
    );
}

fn native(editor: &mut CanvasEditor, keys: Vec<KeyEvent>, clipboard: &mut Clipboard) {
    assert!(editor.handle_native_text(
        &Input {
            keys,
            ..Input::default()
        },
        Platform::X11,
        clipboard
    ));
}

#[test]
fn selection_replacement_and_grapheme_deletion_use_core_semantics() {
    let (mut editor, _) = rig("alpha beta");
    assert!(editor.edit_text("text"));
    assert!(editor.set_text_selection(0, 0));
    for _ in 0..6 {
        frame(&mut editor, Key::Right, false);
    }
    for _ in 0..4 {
        frame(&mut editor, Key::Right, true);
    }
    assert_eq!(editor.text_selection(), Some((6, 10)));
    editor.handle(
        &EditorInput {
            text: "new".into(),
            ..EditorInput::default()
        },
        &EstimateMeasure,
    );
    assert_eq!(text(&editor), "alpha new");
    frame(&mut editor, Key::Home, false);
    assert_eq!(editor.text_selection(), Some((0, 0)));
    frame(&mut editor, Key::End, true);
    assert_eq!(editor.text_selection(), Some((0, 9)));
    editor.finish_text();
    assert_eq!(editor.mutation().document.revision, 1);
    assert_eq!(editor.mutation().undo_label.as_deref(), Some("Edit text"));

    let (mut editor, _) = rig("Ae\u{301}😀Z");
    editor.edit_text("text");
    assert!(!editor.set_text_selection(2, 3), "inside a UTF-8 codepoint");
    editor.set_text_selection(1, 1);
    frame(&mut editor, Key::Delete, false);
    assert_eq!(text(&editor), "A😀Z");
    frame(&mut editor, Key::Right, false);
    assert_eq!(editor.text_selection(), Some((5, 5)));
    frame(&mut editor, Key::Backspace, false);
    assert_eq!(text(&editor), "AZ");
    editor.set_text_selection(2, 2);
    frame(&mut editor, Key::Delete, false);
    assert_eq!(
        text(&editor),
        "AZ",
        "forward delete at end is consumed no-op"
    );
}

#[test]
fn clipboard_uses_native_platform_shortcut_table_and_preserves_lines() {
    let (mut editor, _) = rig("alpha beta");
    editor.edit_text("text");
    editor.set_text_selection(10, 6);
    let ctrl = rndr_core::Modifiers {
        ctrl: true,
        ..rndr_core::Modifiers::default()
    };
    let mut clipboard = Clipboard::default();
    let physical_copy = KeyEvent::character('ж', ctrl).with_code(KeyCode::letter('c').unwrap());
    native(&mut editor, vec![physical_copy], &mut clipboard);
    assert_eq!(clipboard.text, "beta");
    assert!(clipboard.dirty);
    native(
        &mut editor,
        vec![KeyEvent::character('x', ctrl)],
        &mut clipboard,
    );
    assert_eq!(text(&editor), "alpha ");
    native(
        &mut editor,
        vec![KeyEvent::character('v', ctrl)],
        &mut clipboard,
    );
    assert_eq!(text(&editor), "alpha beta");
    clipboard.text = "one\r\ntwo".into();
    native(
        &mut editor,
        vec![
            KeyEvent::character('a', ctrl),
            KeyEvent::character('v', ctrl),
        ],
        &mut clipboard,
    );
    assert_eq!(text(&editor), "one\ntwo");
    assert_eq!(editor.text_selection(), Some((7, 7)));
    // Released/synthetic keys never edit or duplicate clipboard operations.
    native(
        &mut editor,
        vec![KeyEvent::character('q', rndr_core::Modifiers::default()).released()],
        &mut clipboard,
    );
    assert_eq!(text(&editor), "one\ntwo");
}

fn point(editor: &CanvasEditor, engine: &dyn TextEngine, byte: usize) -> [f32; 2] {
    let nodes = editor.nodes();
    let node = nodes.iter().find(|n| n.id == "text").unwrap();
    let Content::Text { text, .. } = &node.content else {
        panic!()
    };
    let layout = engine.layout(
        text,
        &TextStyle {
            family: "sans".into(),
            size: 24.0,
            line_height: Some(1.3),
            ..TextStyle::default()
        },
        &[],
        f32::INFINITY,
    );
    let (x, y) = layout.caret_xy(byte);
    let world = SceneIndex::new(&nodes)
        .world_matrix(node)
        .apply(Vec2::new(f64::from(x + 1.44), f64::from(y + 10.0)));
    let view = editor.camera.matrix().apply(world);
    [view.x as f32, view.y as f32]
}

fn pointer(
    editor: &mut CanvasEditor,
    at: [f32; 2],
    pressed: bool,
    down: bool,
    released: bool,
    engine: &Arc<dyn TextEngine>,
) {
    editor.handle(
        &EditorInput {
            pointer: Some(PointerInput {
                position: at,
                pressed,
                down,
                released,
                ..PointerInput::default()
            }),
            ..EditorInput::default()
        },
        &EngineMeasure(engine.clone()),
    );
}

#[test]
fn pointer_caret_and_drag_follow_rotated_object_camera_and_parent() {
    let (mut editor, engine) = rig("abcdefghij");
    let mut node = editor.nodes()[0].clone();
    node.transform.rotation = 31.0;
    node.parent_id = Some("parent".into());
    let parent = Node::new(
        "parent",
        Content::Group,
        Transform {
            x: 60.0,
            y: 40.0,
            width: 500.0,
            height: 300.0,
            rotation: -17.0,
        },
    );
    assert!(editor.commit(
        "Parent and rotate",
        vec![Operation::put(parent), Operation::put(node)]
    ));
    editor.camera.zoom = 2.25;
    editor.camera.pan_x = -95.0;
    editor.camera.pan_y = 160.0;
    editor.edit_text("text");
    let at = point(&editor, engine.as_ref(), 3);
    pointer(&mut editor, at, true, true, false, &engine);
    assert_eq!(editor.text_selection(), Some((3, 3)));
    let at = point(&editor, engine.as_ref(), 7);
    pointer(&mut editor, at, false, true, false, &engine);
    pointer(&mut editor, at, false, false, true, &engine);
    assert_eq!(editor.text_selection(), Some((3, 7)));
    editor.handle(
        &EditorInput {
            text: "!".into(),
            ..EditorInput::default()
        },
        &EngineMeasure(engine),
    );
    assert_eq!(text(&editor), "abc!hij");
    assert_eq!(
        editor.mutation().document.revision,
        1,
        "draft selection and text aren't transactions"
    );
}

#[test]
fn text_and_manipulation_share_document_undo_redo() {
    let (mut editor, _) = rig("before");
    editor.edit_text("text");
    editor.set_text_selection(0, 6);
    editor.handle(
        &EditorInput {
            text: "after".into(),
            ..EditorInput::default()
        },
        &EstimateMeasure,
    );
    frame(&mut editor, Key::Escape, false);
    assert!(editor.editing_text().is_none());
    assert_eq!(text(&editor), "after");
    frame(&mut editor, Key::Right, false);
    assert_eq!(
        editor.nodes()[0].transform.x,
        101.0,
        "arrows return to canvas nudge"
    );
    assert!(editor.undo());
    assert_eq!(editor.nodes()[0].transform.x, 100.0);
    assert_eq!(text(&editor), "after");
    assert!(editor.undo());
    assert_eq!(text(&editor), "before");
    assert!(editor.redo());
    assert_eq!(text(&editor), "after");
    assert!(editor.redo());
    assert_eq!(editor.nodes()[0].transform.x, 101.0);
}

#[test]
fn composition_filters_keys_and_deduplicates_commit_without_saving_preedit() {
    let (mut editor, _) = rig("abc");
    editor.edit_text("text");
    editor.set_text_selection(1, 2);
    let mut clipboard = Clipboard::default();
    editor.handle_native_text(
        &Input {
            ime_enabled: true,
            ime_preedit: "あ".into(),
            keys: vec![KeyEvent::character('a', rndr_core::Modifiers::default())],
            ..Input::default()
        },
        Platform::X11,
        &mut clipboard,
    );
    assert_eq!(text(&editor), "abc");
    assert_eq!(editor.mutation().document.revision, 0);
    editor.handle_native_text(
        &Input {
            ime_enabled: true,
            ime_commit: "あ".into(),
            keys: vec![KeyEvent::character('a', rndr_core::Modifiers::default())],
            ..Input::default()
        },
        Platform::X11,
        &mut clipboard,
    );
    assert_eq!(text(&editor), "aあc");
    editor.finish_text();
    assert_eq!(editor.mutation().document.revision, 1);
}

#[test]
fn focused_view_routes_text_clipboard_and_releases_canvas_shortcuts() {
    let (editor, engine) = rig("hello");
    let mut world = World::new();
    let slot = world.spawn((Layout::column(), SurfaceSlot { id: 7 }));
    world.insert(
        slot,
        UiRect {
            x: 20.0,
            y: 30.0,
            w: 600.0,
            h: 400.0,
        },
    );
    world.insert_resource(engine);
    view::install(&mut world, CanvasView::new(editor, 7));
    world
        .resource_mut::<CanvasView>()
        .unwrap()
        .editor
        .edit_text("text");
    assert!(view::focus(&mut world));
    world.insert_resource(Input {
        keys: vec![KeyEvent::character('!', rndr_core::Modifiers::default())],
        ..Input::default()
    });
    view::update(&mut world);
    assert_eq!(
        text(&world.resource::<CanvasView>().unwrap().editor),
        "hello!"
    );
    assert!(
        world.has::<rndr_core::CapturesKeys>(slot),
        "native runner enables IME on editing surface"
    );
    assert!(
        world.resource::<Input>().unwrap().keys.is_empty(),
        "do not replay canvas keys into another interpreter"
    );
    let other = world.spawn(Layout::column());
    world.insert_resource(Focus {
        entity: Some(other),
    });
    world.insert_resource(Input {
        keys: vec![KeyEvent::named(
            NamedKey::Delete,
            rndr_core::Modifiers::default(),
        )],
        chars: "wrong target".into(),
        ..Input::default()
    });
    view::update(&mut world);
    let editor = &world.resource::<CanvasView>().unwrap().editor;
    assert!(editor.editing_text().is_none());
    assert_eq!(text(editor), "hello!");
    assert_eq!(
        editor.mutation().document.revision,
        1,
        "focus loss commits the draft once"
    );
    assert!(!world.has::<rndr_core::CapturesKeys>(slot));
    assert_eq!(
        world.resource::<Input>().unwrap().keys.len(),
        1,
        "foreign focus retains its own input"
    );
}

#[test]
fn early_keyboard_claim_precedes_chrome_and_tab_returns_to_focus_traversal() {
    let (editor, _) = rig("hello");
    let mut app = rndr_core::App::new();
    app.add_plugin(rndr_core::Chrome)
        .add_plugin(rndr_core::GenericInteraction);
    let world = app.world_mut();
    let slot = world.spawn((Layout::column(), SurfaceSlot { id: 7 }));
    world.insert(
        slot,
        UiRect {
            x: 0.0,
            y: 0.0,
            w: 600.0,
            h: 400.0,
        },
    );
    let other = world.spawn((Layout::column(), rndr_core::Focusable));
    world.insert(
        other,
        UiRect {
            x: 0.0,
            y: 450.0,
            w: 100.0,
            h: 40.0,
        },
    );
    view::install(world, CanvasView::new(editor, 7));
    world
        .resource_mut::<CanvasView>()
        .unwrap()
        .editor
        .edit_text("text");
    assert!(view::focus(world));
    world.insert_resource(Input {
        keys: vec![KeyEvent::character('!', rndr_core::Modifiers::default())],
        chars: "!".into(),
        ..Input::default()
    });
    view::take_wheel(world);
    assert!(world.resource::<Input>().unwrap().keys.is_empty());
    assert!(world.resource::<Input>().unwrap().chars.is_empty());
    // Chrome runs with no canvas-owned queue left to reinterpret.
    app.update();
    view::update(app.world_mut());
    assert_eq!(
        text(&app.world().resource::<CanvasView>().unwrap().editor),
        "hello!"
    );
    let world = app.world_mut();
    // Restore explicit test rectangles after the headless layout pass.
    world.insert(
        slot,
        UiRect {
            x: 0.0,
            y: 0.0,
            w: 600.0,
            h: 400.0,
        },
    );
    world.insert(
        other,
        UiRect {
            x: 0.0,
            y: 450.0,
            w: 100.0,
            h: 40.0,
        },
    );
    world.insert_resource(Input {
        key: Some(rndr_core::Key::Tab),
        keys: vec![KeyEvent::named(
            NamedKey::Tab,
            rndr_core::Modifiers::default(),
        )],
        ..Input::default()
    });
    view::take_keyboard(world);
    assert_eq!(
        world.resource::<Input>().unwrap().keys.len(),
        1,
        "Tab stays available to generic focus"
    );
    assert!(!world.has::<rndr_core::CapturesKeys>(slot));
    assert!(world
        .resource::<CanvasView>()
        .unwrap()
        .editor
        .editing_text()
        .is_none());
    app.update();
    assert_eq!(app.world().resource::<Focus>().unwrap().entity, Some(other));
}

fn view_rig() -> (World, rndr_core::Entity) {
    let (editor, engine) = rig("abc");
    let mut world = World::new();
    let slot = world.spawn((Layout::column(), SurfaceSlot { id: 7 }));
    world.insert(
        slot,
        UiRect {
            x: 0.0,
            y: 0.0,
            w: 600.0,
            h: 400.0,
        },
    );
    world.insert_resource(engine);
    view::install(&mut world, CanvasView::new(editor, 7));
    world
        .resource_mut::<CanvasView>()
        .unwrap()
        .editor
        .edit_text("text");
    assert!(view::focus(&mut world));
    (world, slot)
}

#[test]
fn disabled_view_refuses_direct_and_reserved_input_even_with_stale_focus() {
    for (reserve, observe_focus) in [(false, false), (true, false), (false, true), (true, true)] {
        let (mut world, slot) = view_rig();
        if observe_focus {
            view::update(&mut world);
        }
        world.insert_resource(Input {
            keys: vec![KeyEvent::character('x', rndr_core::Modifiers::default())],
            chars: "x".into(),
            ..Input::default()
        });
        if reserve {
            view::take_keyboard(&mut world);
            assert!(world.resource::<Input>().unwrap().keys.is_empty());
        }
        world.insert(slot, rndr_core::ControlDisabled);
        assert!(!rndr_core::access_details(&world, slot).focus);
        assert_eq!(world.resource::<Focus>().unwrap().entity, Some(slot));
        view::take_keyboard(&mut world);
        view::update(&mut world);
        let editor = &world.resource::<CanvasView>().unwrap().editor;
        assert_eq!(text(editor), "abc");
        assert_eq!(editor.mutation().document.revision, 0);
        assert!(editor.editing_text().is_none());
        assert!(!world.has::<rndr_core::CapturesKeys>(slot));
        if !reserve {
            assert_eq!(world.resource::<Input>().unwrap().chars, "x");
            assert_eq!(world.resource::<Input>().unwrap().keys.len(), 1);
        }
        // Shortcut input must also remain unconsumed and cannot manipulate objects.
        world.insert_resource(Input {
            keys: vec![KeyEvent::named(
                NamedKey::Delete,
                rndr_core::Modifiers::default(),
            )],
            ..Input::default()
        });
        view::take_keyboard(&mut world);
        view::update(&mut world);
        assert_eq!(text(&world.resource::<CanvasView>().unwrap().editor), "abc");
        assert_eq!(world.resource::<Input>().unwrap().keys.len(), 1);
        world.remove::<rndr_core::ControlDisabled>(slot);
        world.insert_resource(Input::default());
        view::update(&mut world);
        assert_eq!(
            text(&world.resource::<CanvasView>().unwrap().editor),
            "abc",
            "reserved input is not replayed after re-enabling"
        );
    }
}

#[test]
fn physical_history_chords_use_platform_contract_in_and_after_text_editing() {
    for platform in [
        Platform::Mac,
        Platform::Windows,
        Platform::X11,
        Platform::Wayland,
    ] {
        let (mut world, _) = view_rig();
        world.insert_resource(platform);
        world.insert_resource(Input {
            keys: vec![KeyEvent::character('x', rndr_core::Modifiers::default())],
            ..Input::default()
        });
        view::update(&mut world);
        assert_eq!(
            text(&world.resource::<CanvasView>().unwrap().editor),
            "abcx"
        );
        let mods = rndr_core::Modifiers {
            super_key: platform.primary_is_super(),
            ctrl: !platform.primary_is_super(),
            ..rndr_core::Modifiers::default()
        };
        let undo = KeyEvent::character('я', mods).with_code(KeyCode::letter('z').unwrap());
        assert_eq!(
            rndr_core::standard_action(platform, &undo),
            Some(rndr_core::EditAction::Undo)
        );
        world.insert_resource(Input {
            keys: vec![undo.clone()],
            ..Input::default()
        });
        view::take_keyboard(&mut world);
        view::update(&mut world);
        assert_eq!(text(&world.resource::<CanvasView>().unwrap().editor), "abc");
        assert!(world
            .resource::<CanvasView>()
            .unwrap()
            .editor
            .editing_text()
            .is_none());
        let redo_z = KeyEvent::character(
            'я',
            rndr_core::Modifiers {
                shift: true,
                ..mods
            },
        )
        .with_code(KeyCode::letter('z').unwrap());
        let mut redos = vec![redo_z];
        if !platform.primary_is_super() {
            redos.push(KeyEvent::character('н', mods).with_code(KeyCode::letter('y').unwrap()));
        }
        let revision = world
            .resource::<CanvasView>()
            .unwrap()
            .editor
            .mutation()
            .document
            .revision;
        let mut refused = vec![
            KeyEvent::character('z', rndr_core::Modifiers { alt: true, ..mods }),
            KeyEvent::character(
                'z',
                rndr_core::Modifiers {
                    ctrl: true,
                    super_key: true,
                    ..mods
                },
            ),
            undo.clone().released(),
            undo.clone().synthetic(),
            KeyEvent::character(
                'z',
                rndr_core::Modifiers {
                    shift: true,
                    ..mods
                },
            )
            .synthetic(),
        ];
        if platform.primary_is_super() {
            refused.push(KeyEvent::character('y', mods));
        }
        for event in refused {
            world.insert_resource(Input {
                keys: vec![event],
                ..Input::default()
            });
            view::update(&mut world);
            let editor = &world.resource::<CanvasView>().unwrap().editor;
            assert_eq!(text(editor), "abc");
            assert_eq!(
                editor.mutation().document.revision,
                revision,
                "refused/released/synthetic history must not execute"
            );
        }
        for redo in redos {
            assert_eq!(
                rndr_core::standard_action(platform, &redo),
                Some(rndr_core::EditAction::Redo)
            );
            world.insert_resource(Input {
                keys: vec![redo],
                ..Input::default()
            });
            view::update(&mut world);
            assert_eq!(
                text(&world.resource::<CanvasView>().unwrap().editor),
                "abcx",
                "physical redo after leaving text editing on {platform:?}"
            );
            world.insert_resource(Input {
                keys: vec![undo.clone()],
                ..Input::default()
            });
            view::update(&mut world);
            assert_eq!(text(&world.resource::<CanvasView>().unwrap().editor), "abc");
        }
    }
}

#[test]
fn unchanged_and_locked_text_respect_existing_document_authority() {
    let (mut editor, _) = rig("hello");
    assert!(editor.edit_text("text"));
    assert!(
        !editor.finish_text(),
        "no content change means no transaction"
    );
    assert_eq!(editor.mutation().document.revision, 0);
    let mut node = editor.nodes()[0].clone();
    node.locked = true;
    editor.commit("Lock", vec![Operation::put(node)]);
    assert!(!editor.edit_text("text"));
    assert!(editor.editing_text().is_none());
}
