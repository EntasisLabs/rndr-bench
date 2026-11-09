//! Event-driven Markdown projection work must settle without new input.
use rndr_core::*;

fn drain_idle(app: &mut App) {
    for _ in 0..16 {
        if !app.wants_frame() {
            assert!(!app.wants_update());
            return;
        }
        assert!(app.wants_update());
        app.update();
    }
    panic!("Markdown did not return to idle");
}

#[test]
fn host_mount_measure_resize_and_detach_request_full_updates_until_published() {
    let mut app = App::new();
    app.add_plugin(MarkdownPlugin);
    app.world_mut().insert_resource(Viewport::new(400., 300.));
    let view = markdown_view(app.world_mut(), "Fallback");
    assert!(app.wants_update());
    drain_idle(&mut app);
    let expected = app.world().get::<MarkdownView>(view).unwrap().blocks()[0].clone();
    let slot = mount_markdown_slot(app.world_mut(), view, 0, &expected, MarkdownSlotId(1)).unwrap();
    let child = app.world_mut().spawn((Layout {
        width: Size::Fill,
        height: Size::Px(32.),
        ..Layout::column()
    },));
    set_parent(app.world_mut(), child, slot.root);
    assert!(app.wants_update());
    drain_idle(&mut app);
    assert_eq!(children_of(app.world(), view), vec![slot.root]);
    assert!(app.world().get::<Rect>(slot.root).unwrap().h >= 32.);

    // The event frame measures the previous Rect; only its layout sees 96.
    app.world_mut().get_mut::<Layout>(child).unwrap().height = Size::Px(96.);
    app.update();
    assert!(
        app.wants_update(),
        "new post-layout height needs publication"
    );
    drain_idle(&mut app);
    assert!(app.world().get::<Rect>(slot.root).unwrap().h >= 96.);

    // Fixed-height hosts must not spin on a deliberately different child size.
    slot.set_height(app.world_mut(), 48.).unwrap();
    drain_idle(&mut app);
    assert!(!app.wants_frame());
    slot.detach(app.world_mut()).unwrap();
    assert!(app.wants_update());
    drain_idle(&mut app);
    let fallback = children_of(app.world(), view)[0];
    assert_eq!(app.world().get::<Label>(fallback).unwrap().text, "Fallback");
    assert!(!app.world().is_alive(child));
}
