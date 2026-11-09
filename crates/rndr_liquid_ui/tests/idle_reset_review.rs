//! The runner must settle a Reset event after streamed removal/type changes,
//! with no further pointer/key event and no fixed-number frame pumping.
use rndr_core::*;
use rndr_liquid_ui::*;
use rndr_ui::{apply_theme_bridge, UiPlugin, UiTheme};

struct Host;
impl LiquidHost for Host {}
const INITIAL: &str = "```input\nid: draft\nlabel: Draft\nvalue: Initial\n```\n\n```slider\nid: volume\nlabel: Level\nvalue: 30\nmin: 0\nmax: 100\n```\n";
const REPLACED: &str = "```input\nid: volume\nlabel: Former slider\nvalue: Replacement\n```\n";

fn idle(app: &mut App) {
    let mut remaining = 32; // Failure bound, not a frame budget to pump.
    while app.wants_frame() {
        assert!(app.wants_update(), "pending projection needs a full update");
        assert!(remaining > 0, "projection never returned to idle");
        remaining -= 1;
        rndr_paint::frame(app);
    }
    assert!(!app.wants_update());
    for entity in app.world().entities_with::<Label>() {
        assert!(
            !app.world()
                .get::<Label>(entity)
                .unwrap()
                .text
                .contains('\u{e001}'),
            "raw slot marker visible at the idle boundary"
        );
    }
}
fn control(app: &App, view: Entity, id: &str) -> Entity {
    app.world()
        .get::<LiquidView>(view)
        .unwrap()
        .control(app.world(), id)
        .unwrap()
}
struct Reset(Option<Entity>);

#[test]
fn reset_after_streamed_removal_and_type_replacement_settles_without_new_input() {
    let mut app = App::new();
    app.add_plugin(UiPlugin)
        .add_plugin(MarkdownPlugin)
        .add_plugin(LiquidUiPlugin::new(Host));
    app.world_mut()
        .insert_resource(Viewport::new(800.0, 1800.0));
    apply_theme_bridge(app.world_mut(), UiTheme::default());
    let view = liquid_view(app.world_mut(), INITIAL);
    idle(&mut app);
    let old_draft = control(&app, view, "draft");
    let old_slider = control(&app, view, "volume");
    assert!(request_focus(app.world_mut(), old_draft));
    update_liquid_source(app.world_mut(), view, REPLACED).unwrap();
    // One host event frame; subsequent frames require authoritative work.
    rndr_paint::frame(&mut app);
    idle(&mut app);
    assert!(!app.world().is_alive(old_draft));
    assert!(!app.world().is_alive(old_slider));
    let replacement = control(&app, view, "volume");
    assert!(app.world().has::<TextField>(replacement));
    assert!(request_focus(app.world_mut(), replacement));
    app.add_system(Schedule::Update, |world: &mut World| {
        let reset = world
            .resource_mut::<Reset>()
            .and_then(|reset| reset.0.take());
        if let Some(view) = reset {
            set_liquid_source(world, view, INITIAL);
        }
    });
    app.world_mut().insert_resource(Reset(Some(view)));
    rndr_paint::frame(&mut app); // Reset click's event frame.
    idle(&mut app); // No more simulated input events.
    assert!(!app.world().is_alive(replacement));
    assert_ne!(
        app.world().resource::<Focus>().unwrap().entity,
        Some(replacement)
    );
    let draft = control(&app, view, "draft");
    assert_eq!(app.world().entities_with::<TextField>(), vec![draft]);
    assert_eq!(
        control_value(app.world(), draft),
        Some(LiquidValue::Text("Initial".into()))
    );
    assert_eq!(
        control_value(app.world(), control(&app, view, "volume")),
        Some(LiquidValue::Number(30.0))
    );
}
