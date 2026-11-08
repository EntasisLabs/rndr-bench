use liquid_canvas_integration::{
    build_app, command, liquid_snapshot, Fixture, EDIT, MOVE, REDO, UNDO,
};
use rndr_canvas::Content;
use rndr_canvas_ui::view::CanvasView;
use rndr_core::*;
use rndr_liquid_ui::*;

fn frames(app: &mut App) {
    for _ in 0..3 {
        rndr_paint::frame(app);
    }
}
fn canvas_text(app: &App) -> String {
    let nodes = app.world().resource::<CanvasView>().unwrap().editor.nodes();
    match &nodes[0].content {
        Content::Text { text, .. } => text.clone(),
        _ => panic!(),
    }
}
fn keys(app: &mut App, keys: Vec<KeyEvent>) {
    app.world_mut().resource_mut::<Input>().unwrap().keys = keys;
    rndr_paint::frame(app);
}

#[test]
fn clipboard_camera_edit_and_focus_transfer_use_the_combined_frame_path() {
    let mut app = build_app();
    app.world_mut().insert_resource(Platform::X11);
    frames(&mut app);
    command(app.world_mut(), EDIT);
    frames(&mut app);
    let shift = Modifiers {
        shift: true,
        ..Modifiers::new()
    };
    let ctrl = Modifiers {
        ctrl: true,
        ..Modifiers::new()
    };
    keys(
        &mut app,
        vec![
            KeyEvent::named(NamedKey::Home, Modifiers::new()),
            KeyEvent::named(NamedKey::ArrowRight, shift),
            KeyEvent::named(NamedKey::ArrowRight, shift),
            KeyEvent::character('c', ctrl),
        ],
    );
    assert_eq!(app.world().resource::<Clipboard>().unwrap().text, "Ed");
    keys(
        &mut app,
        vec![
            KeyEvent::named(NamedKey::End, Modifiers::new()),
            KeyEvent::character('v', ctrl),
        ],
    );
    assert_eq!(canvas_text(&app), "Edit the middle of this textEd");
    command(app.world_mut(), liquid_canvas_integration::ZOOM);
    frames(&mut app);
    assert_eq!(
        app.world()
            .resource::<CanvasView>()
            .unwrap()
            .editor
            .camera
            .zoom,
        1.3
    );
    keys(
        &mut app,
        vec![
            KeyEvent::named(NamedKey::Home, Modifiers::new()),
            KeyEvent::character('!', Modifiers::new()),
        ],
    );
    assert_eq!(canvas_text(&app), "!Edit the middle of this textEd");
    let fixture = *app.world().resource::<Fixture>().unwrap();
    let root = app
        .world()
        .get::<LiquidView>(fixture.liquid)
        .unwrap()
        .component("name")
        .unwrap();
    let input = control_entity(app.world(), root).unwrap();
    request_focus(app.world_mut(), input);
    keys(&mut app, vec![KeyEvent::character('q', Modifiers::new())]);
    assert_eq!(canvas_text(&app), "!Edit the middle of this textEd");
    assert_eq!(
        app.world()
            .resource::<CanvasView>()
            .unwrap()
            .editor
            .editing_text(),
        None
    );
    assert_eq!(
        app.world()
            .resource::<CanvasView>()
            .unwrap()
            .editor
            .mutation()
            .document
            .revision,
        1
    );
    assert_eq!(app.world().get::<Label>(input).unwrap().text, "Adaq");
    assert_eq!(app.world().resource::<Focus>().unwrap().entity, Some(input));
}

#[test]
fn one_host_routes_liquid_drafts_and_canvas_edits_into_shared_document_history() {
    let mut app = build_app();
    frames(&mut app);
    let fixture = *app.world().resource::<Fixture>().unwrap();
    let root = app
        .world()
        .get::<LiquidView>(fixture.liquid)
        .unwrap()
        .component("name")
        .unwrap();
    let input = control_entity(app.world(), root).unwrap();
    assert!(request_focus(app.world_mut(), input));
    keys(&mut app, vec![KeyEvent::character('!', Modifiers::new())]);
    let draft = app.world().get::<Label>(input).unwrap().text.clone();
    for stage in 1..=5 {
        let expected = if stage == 4 {
            LiquidUpdate::Incomplete
        } else {
            LiquidUpdate::Applied
        };
        assert_eq!(
            update_liquid_source(app.world_mut(), fixture.liquid, &liquid_snapshot(stage)),
            Ok(expected)
        );
        frames(&mut app);
        assert_eq!(
            app.world()
                .get::<LiquidView>(fixture.liquid)
                .unwrap()
                .component("name"),
            Some(root)
        );
        assert_eq!(app.world().resource::<Focus>().unwrap().entity, Some(input));
        assert_eq!(app.world().get::<Label>(input).unwrap().text, draft);
        assert_eq!(canvas_text(&app), "Edit the middle of this text");
    }
    command(app.world_mut(), EDIT);
    frames(&mut app);
    // The existing core editor receives a host key queue, not a fixture-only edit API.
    let shift = Modifiers {
        shift: true,
        ..Modifiers::new()
    };
    keys(
        &mut app,
        vec![
            KeyEvent::named(NamedKey::Home, Modifiers::new()),
            KeyEvent::named(NamedKey::ArrowRight, shift),
            KeyEvent::character('R', Modifiers::new()),
        ],
    );
    assert_eq!(canvas_text(&app), "Rdit the middle of this text");
    keys(
        &mut app,
        vec![KeyEvent::named(NamedKey::Escape, Modifiers::new())],
    );
    let revision = app
        .world()
        .resource::<CanvasView>()
        .unwrap()
        .editor
        .mutation()
        .document
        .revision;
    assert_eq!(revision, 1);
    command(app.world_mut(), MOVE);
    assert_eq!(
        app.world().resource::<CanvasView>().unwrap().editor.nodes()[0]
            .transform
            .x,
        100.0
    );
    command(app.world_mut(), UNDO);
    assert_eq!(canvas_text(&app), "Rdit the middle of this text");
    command(app.world_mut(), UNDO);
    assert_eq!(canvas_text(&app), "Edit the middle of this text");
    command(app.world_mut(), REDO);
    command(app.world_mut(), REDO);
    assert_eq!(canvas_text(&app), "Rdit the middle of this text");
    assert_eq!(
        app.world().resource::<CanvasView>().unwrap().editor.nodes()[0]
            .transform
            .x,
        100.0
    );
    assert_eq!(app.world().get::<Label>(input).unwrap().text, draft);
}
