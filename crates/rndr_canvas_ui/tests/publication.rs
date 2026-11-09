use rndr_canvas::*;
use rndr_canvas_ui::editor::{CanvasEditor, EditorInput, Key, Modifiers};
use rndr_core::{Clipboard, Input, KeyEvent, NamedKey, Platform};
use std::sync::Arc;

fn rig() -> (Arc<MemoryStore>, CanvasEditor, CanvasEditor) {
    let store = Arc::new(MemoryStore::new());
    store
        .create(CreateRequest {
            id: "doc".into(),
            title: "Doc".into(),
            scene: Scene {
                nodes: vec![Node::new(
                    "text",
                    Content::Text {
                        text: "abc".into(),
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
                )],
                ..Scene::default()
            },
        })
        .unwrap();
    let local = CanvasEditor::open(store.clone(), "doc").unwrap();
    let other = CanvasEditor::open(store.clone(), "doc").unwrap();
    (store, local, other)
}

fn draft(editor: &mut CanvasEditor) {
    assert!(editor.edit_text("text"));
    editor.handle(
        &EditorInput {
            text: "x".into(),
            ..EditorInput::default()
        },
        &EstimateMeasure,
    );
}

fn text(editor: &CanvasEditor) -> String {
    let Content::Text { text, .. } = &editor.nodes()[0].content else {
        panic!()
    };
    text.clone()
}

#[test]
fn refused_publication_stops_undo_and_redo_on_every_adapter() {
    for redo in [false, true] {
        for route in ["direct", "compact", "native"] {
            let (store, mut local, mut other) = rig();
            draft(&mut local);
            let mut moved = other.nodes()[0].clone();
            moved.transform.x = 200.0;
            assert!(other.commit("Other move", vec![Operation::put(moved)]));
            if redo {
                assert!(other.undo());
            }
            let before = store.get("doc").unwrap();
            match route {
                "direct" => {
                    assert!(!if redo { local.redo() } else { local.undo() });
                }
                "compact" => {
                    local.handle(
                        &EditorInput {
                            keys: vec![(
                                Key::Char('z'),
                                Modifiers {
                                    primary: true,
                                    shift: redo,
                                    ..Modifiers::default()
                                },
                            )],
                            ..EditorInput::default()
                        },
                        &EstimateMeasure,
                    );
                }
                "native" => {
                    assert!(local.handle_native_text(
                        &Input {
                            keys: vec![KeyEvent::character(
                                'z',
                                rndr_core::Modifiers {
                                    ctrl: true,
                                    shift: redo,
                                    ..Default::default()
                                }
                            )],
                            ..Input::default()
                        },
                        Platform::X11,
                        &mut Clipboard::default()
                    ));
                }
                _ => unreachable!(),
            }
            let after = store.get("doc").unwrap();
            assert_eq!(after.document, before.document, "{route}, redo={redo}");
            assert_eq!(
                (after.can_undo, after.can_redo),
                (before.can_undo, before.can_redo)
            );
            assert!(
                matches!(local.last_error(), Some(CanvasError::Conflict(_))),
                "{route}, redo={redo}"
            );
            assert!(local.editing_text().is_none());
            assert_eq!(text(&local), "abc");
            // Conflict reloads the current snapshot. A separate, deliberate
            // history request may now traverse that snapshot's history.
            assert!(if redo { local.redo() } else { local.undo() });
            assert!(local.last_error().is_none());
            assert_eq!(
                local.nodes()[0].transform.x,
                if redo { 200.0 } else { 100.0 }
            );
        }
    }
}

#[test]
fn unchanged_draft_is_not_a_refusal_even_after_an_earlier_conflict() {
    let (_, mut local, mut other) = rig();
    draft(&mut local);
    let mut moved = other.nodes()[0].clone();
    moved.transform.x = 200.0;
    assert!(other.commit("Other move", vec![Operation::put(moved)]));
    assert!(!local.undo());
    assert!(matches!(local.last_error(), Some(CanvasError::Conflict(_))));
    assert!(local.edit_text("text"));
    assert!(
        local.undo(),
        "an unchanged draft does not re-raise a stale refusal"
    );
    assert_eq!(local.nodes()[0].transform.x, 100.0);
    assert_eq!(text(&local), "abc");
    assert!(local.last_error().is_none());
}

#[test]
fn committed_ime_text_precedes_exit_and_history_in_the_same_batch() {
    for exit in ["escape", "legacy_escape", "enter", "undo", "redo"] {
        let (_, mut editor, _) = rig();
        assert!(editor.edit_text("text"));
        let keys = match exit {
            "escape" => vec![KeyEvent::named(NamedKey::Escape, Default::default())],
            "legacy_escape" => vec![],
            "enter" => vec![KeyEvent::named(
                NamedKey::Enter,
                rndr_core::Modifiers {
                    ctrl: true,
                    ..Default::default()
                },
            )],
            _ => vec![KeyEvent::character(
                'z',
                rndr_core::Modifiers {
                    ctrl: true,
                    shift: exit == "redo",
                    ..Default::default()
                },
            )],
        };
        assert!(editor.handle_native_text(
            &Input {
                ime_enabled: true,
                ime_commit: "確".into(),
                keys,
                key: (exit == "legacy_escape").then_some(rndr_core::Key::Escape),
                ..Input::default()
            },
            Platform::X11,
            &mut Clipboard::default()
        ));
        assert!(editor.editing_text().is_none(), "{exit}");
        assert_eq!(
            text(&editor),
            if exit == "undo" { "abc" } else { "abc確" },
            "{exit}"
        );
        if exit == "undo" {
            assert!(editor.redo());
        } else {
            assert!(editor.undo());
            assert!(editor.redo());
        }
        assert_eq!(
            text(&editor),
            "abc確",
            "commit was retained exactly once: {exit}"
        );
    }
}

#[test]
fn tab_keeps_committed_ime_text_on_the_canvas_and_leaves_traversal_unconsumed() {
    use rndr_canvas_ui::view::{self, CanvasView};
    use rndr_core::{Layout, Rect, SurfaceSlot, World};
    let (_, editor, _) = rig();
    let mut world = World::new();
    let slot = world.spawn((Layout::column(), SurfaceSlot { id: 7 }));
    world.insert(
        slot,
        Rect {
            x: 0.0,
            y: 0.0,
            w: 600.0,
            h: 400.0,
        },
    );
    view::install(&mut world, CanvasView::new(editor, 7));
    assert!(view::focus(&mut world));
    world
        .resource_mut::<CanvasView>()
        .unwrap()
        .editor
        .edit_text("text");
    world.insert_resource(Input {
        ime_enabled: true,
        ime_commit: "確".into(),
        key: Some(rndr_core::Key::Tab),
        keys: vec![KeyEvent::named(NamedKey::Tab, Default::default())],
        ..Input::default()
    });
    view::take_keyboard(&mut world);
    let editor = &world.resource::<CanvasView>().unwrap().editor;
    assert_eq!(text(editor), "abc確");
    assert!(editor.editing_text().is_none());
    assert_eq!(editor.mutation().document.revision, 1);
    let input = world.resource::<Input>().unwrap();
    assert_eq!(input.keys.len(), 1, "Tab remains for traversal");
    assert!(
        input.ime_commit.is_empty(),
        "accepted commit cannot replay into the next field"
    );
}

#[test]
fn preedit_is_not_published_but_a_simultaneous_commit_is_retained() {
    let (_, mut editor, _) = rig();
    assert!(editor.edit_text("text"));
    let mut clipboard = Clipboard::default();
    assert!(editor.handle_native_text(
        &Input {
            ime_enabled: true,
            ime_preedit: "未".into(),
            ime_commit: "確".into(),
            keys: vec![KeyEvent::named(NamedKey::Escape, Default::default())],
            ..Input::default()
        },
        Platform::X11,
        &mut clipboard
    ));
    assert_eq!(text(&editor), "abc確");
    assert!(
        editor.editing_text().is_some(),
        "IME owns Escape during preedit"
    );
    assert_eq!(editor.mutation().document.revision, 0);
    assert!(editor.handle_native_text(
        &Input {
            keys: vec![KeyEvent::named(NamedKey::Escape, Default::default())],
            ..Input::default()
        },
        Platform::X11,
        &mut clipboard
    ));
    assert_eq!(text(&editor), "abc確");
    assert_eq!(editor.mutation().document.revision, 1);
}
