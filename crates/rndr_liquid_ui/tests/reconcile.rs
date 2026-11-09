use rndr_core::*;
use rndr_liquid_ui::*;
use rndr_ui::{apply_theme_bridge, UiPlugin, UiTheme};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

struct Host;
impl LiquidHost for Host {}
fn setup(source: &str) -> (App, Entity) {
    let mut app = App::new();
    app.add_plugin(UiPlugin);
    app.add_plugin(MarkdownPlugin);
    app.add_plugin(LiquidUiPlugin::new(Host));
    app.world_mut().insert_resource(Viewport::new(800., 1800.));
    apply_theme_bridge(app.world_mut(), UiTheme::default());
    let view = liquid_view(app.world_mut(), source);
    frames(&mut app);
    (app, view)
}
fn frames(app: &mut App) {
    rndr_paint::frame(app);
    rndr_paint::frame(app);
}
fn control(app: &App, view: Entity, id: &str) -> Entity {
    app.world()
        .get::<LiquidView>(view)
        .unwrap()
        .control(app.world(), id)
        .unwrap()
}
const CHECK: &str = "```checkbox\nid: check\nlabel: Keep me\nvalue: false\n```\n";
const INPUT: &str = "```input\nid: input\nlabel: Name\nvalue: Initial\n```\n";
const SLIDER: &str =
    "```slider\nid: slider\nlabel: Volume\nvalue: 25\nmin: 0\nmax: 100\nstep: 5\n```\n";
fn all() -> String {
    format!("{CHECK}\n{INPUT}\n{SLIDER}")
}
fn type_text(app: &mut App, entity: Entity, text: &str) {
    assert!(request_focus(app.world_mut(), entity));
    app.world_mut().resource_mut::<Input>().unwrap().chars = text.into();
    rndr_paint::frame(app);
    app.world_mut()
        .resource_mut::<Input>()
        .unwrap()
        .chars
        .clear();
}

#[test]
fn stream_and_reorder_keep_editor_identity_focus_selection_and_local_values() {
    let (mut app, view) = setup(&all());
    let check = control(&app, view, "check");
    assert_eq!(
        try_activate(app.world_mut(), check),
        ActivationResult::Accepted
    );
    let input = control(&app, view, "input");
    type_text(&mut app, input, "λ");
    let text = control_value(app.world(), input).unwrap();
    // Directed selection is part of the retained editor state, not just its text.
    app.world_mut().insert(
        input,
        Caret {
            anchor: 5,
            index: 1,
        },
    );
    let caret = app.world().get::<Caret>(input).cloned();
    let slider = control(&app, view, "slider");
    // Use the native key-navigation hook, not a second editing interpreter.
    let hook = app.world().get::<KeyNavigation>(slider).unwrap().0;
    let key = KeyEvent::named(NamedKey::ArrowRight, Modifiers::default());
    hook(app.world_mut(), slider, &key);
    assert_eq!(
        control_value(app.world(), slider),
        Some(LiquidValue::Number(30.))
    );
    assert!(request_focus(app.world_mut(), input));
    for source in [
        format!("Before\n\n{CHECK}\nBetween\n\n{INPUT}\n{SLIDER}"),
        format!("Moved\n\n{SLIDER}\n{INPUT}\n{CHECK}"),
    ] {
        assert_eq!(
            update_liquid_source(app.world_mut(), view, &source),
            Ok(UpdateStatus::Applied)
        );
        frames(&mut app);
        assert_eq!(control(&app, view, "check"), check);
        assert_eq!(
            control_value(app.world(), check),
            Some(LiquidValue::Checked(true))
        );
        assert_eq!(control(&app, view, "input"), input);
        assert_eq!(control_value(app.world(), input), Some(text.clone()));
        assert_eq!(
            app.world().get::<Caret>(input).map(|c| (c.anchor, c.index)),
            caret.as_ref().map(|c| (c.anchor, c.index))
        );
        assert_eq!(app.world().resource::<Focus>().unwrap().entity, Some(input));
        assert_eq!(control(&app, view, "slider"), slider);
        assert_eq!(
            control_value(app.world(), slider),
            Some(LiquidValue::Number(30.))
        );
    }
}

#[test]
fn slider_user_changes_respect_admission_and_controlled_host_values() {
    let (mut app, view) = setup(SLIDER);
    let slider = control(&app, view, "slider");
    let hook = app.world().get::<KeyNavigation>(slider).unwrap().0;
    let key = KeyEvent::named(NamedKey::End, Modifiers::default());
    app.world_mut().insert(slider, ControlDisabled);
    assert!(!hook(app.world_mut(), slider, &key));
    assert_eq!(
        control_value(app.world(), slider),
        Some(LiquidValue::Number(25.))
    );
    app.world_mut().remove::<ControlDisabled>(slider);
    app.world_mut()
        .get_mut::<Semantics>(slider)
        .unwrap()
        .readonly = true;
    assert!(!hook(app.world_mut(), slider, &key));
    assert_eq!(
        control_value(app.world(), slider),
        Some(LiquidValue::Number(25.))
    );
    app.world_mut()
        .get_mut::<Semantics>(slider)
        .unwrap()
        .readonly = false;
    assert!(hook(app.world_mut(), slider, &key));
    assert_eq!(
        control_value(app.world(), slider),
        Some(LiquidValue::Number(100.))
    );
    // Sends outside Input publish on the next frame, like every core event.
    rndr_paint::frame(&mut app);
    let requests = events::<LiquidControlChanged>(app.world()).len();
    assert_eq!(requests, 1);
    let controlled = SLIDER.replace("value: 25", "value: 60\ncontrolled: true");
    update_liquid_source(app.world_mut(), view, &controlled).unwrap();
    assert_eq!(
        control_value(app.world(), slider),
        Some(LiquidValue::Number(60.))
    );
    assert_eq!(events::<LiquidControlChanged>(app.world()).len(), requests);
    rndr_paint::frame(&mut app);
    assert!(events::<LiquidControlChanged>(app.world()).is_empty());
    frames(&mut app);
    assert_eq!(control(&app, view, "slider"), slider);
}

#[test]
fn controlled_update_and_explicit_reset_are_distinct() {
    let (mut app, view) = setup(INPUT);
    let input = control(&app, view, "input");
    type_text(&mut app, input, "local");
    let controlled = INPUT.replace("value: Initial", "value: Authoritative\ncontrolled: true");
    update_liquid_source(app.world_mut(), view, &controlled).unwrap();
    frames(&mut app);
    assert_eq!(control(&app, view, "input"), input);
    assert_eq!(
        control_value(app.world(), input),
        Some(LiquidValue::Text("Authoritative".into()))
    );
    set_liquid_source(app.world_mut(), view, INPUT);
    frames(&mut app);
    assert!(!app.world().is_alive(input));
    let reset = control(&app, view, "input");
    assert_ne!(reset, input);
    assert_eq!(
        control_value(app.world(), reset),
        Some(LiquidValue::Text("Initial".into()))
    );
}

#[test]
fn partial_fence_and_duplicate_ids_do_not_mutate_the_last_good_ui() {
    let (mut app, view) = setup(INPUT);
    let input = control(&app, view, "input");
    type_text(&mut app, input, "kept");
    let old = control_value(app.world(), input);
    assert_eq!(
        update_liquid_source(app.world_mut(), view, "New text\n\n```inp"),
        Ok(UpdateStatus::Deferred)
    );
    frames(&mut app);
    assert_eq!(control(&app, view, "input"), input);
    assert_eq!(control_value(app.world(), input), old);
    assert_eq!(
        update_liquid_source(app.world_mut(), view, &format!("{INPUT}\n{INPUT}")),
        Err(LiquidUpdateError::DuplicateId("input".into()))
    );
    frames(&mut app);
    assert_eq!(control(&app, view, "input"), input);
    update_liquid_source(app.world_mut(), view, &format!("Complete\n\n{INPUT}")).unwrap();
    frames(&mut app);
    assert_eq!(control(&app, view, "input"), input);
}

#[test]
fn nested_controls_survive_container_data_updates_and_move_to_top_level() {
    let nested = format!("```report\nid: report\ntitle: Before\nOpening\n{INPUT}\nClosing\n```\n");
    let (mut app, view) = setup(&nested);
    frames(&mut app);
    let input = control(&app, view, "input");
    type_text(&mut app, input, "local");
    let value = control_value(app.world(), input);
    refresh(app.world_mut());
    frames(&mut app);
    frames(&mut app);
    assert_eq!(control(&app, view, "input"), input);
    assert_eq!(control_value(app.world(), input), value);
    assert_eq!(app.world().resource::<Focus>().unwrap().entity, Some(input));
    update_liquid_source(
        app.world_mut(),
        view,
        &nested.replace("title: Before", "title: After"),
    )
    .unwrap();
    frames(&mut app);
    frames(&mut app);
    assert_eq!(control(&app, view, "input"), input);
    assert_eq!(control_value(app.world(), input), value);
    assert_eq!(app.world().resource::<Focus>().unwrap().entity, Some(input));
    update_liquid_source(app.world_mut(), view, INPUT).unwrap();
    frames(&mut app);
    assert_eq!(control(&app, view, "input"), input);
    assert_eq!(control_value(app.world(), input), value);
    update_liquid_source(app.world_mut(), view, &nested).unwrap();
    frames(&mut app);
    frames(&mut app);
    assert_eq!(control(&app, view, "input"), input);
    assert_eq!(control_value(app.world(), input), value);
}

struct Lease(Arc<AtomicUsize>);
impl Component for Lease {}
impl Drop for Lease {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}
#[test]
fn removal_kind_change_and_view_despawn_retire_owned_resources_once() {
    let (mut app, view) = setup(INPUT);
    let input = control(&app, view, "input");
    let dropped = Arc::new(AtomicUsize::new(0));
    app.world_mut().insert(input, Lease(dropped.clone()));
    request_focus(app.world_mut(), input);
    update_liquid_source(
        app.world_mut(),
        view,
        &CHECK.replace("id: check", "id: input"),
    )
    .unwrap();
    frames(&mut app);
    assert!(!app.world().is_alive(input));
    assert_ne!(app.world().resource::<Focus>().unwrap().entity, Some(input));
    assert_eq!(dropped.load(Ordering::SeqCst), 1);
    let check = control(&app, view, "input");
    app.world_mut().insert(check, Lease(dropped.clone()));
    update_liquid_source(app.world_mut(), view, "Empty").unwrap();
    frames(&mut app);
    assert!(!app.world().is_alive(check));
    assert_eq!(dropped.load(Ordering::SeqCst), 2);
    update_liquid_source(app.world_mut(), view, INPUT).unwrap();
    frames(&mut app);
    let input = control(&app, view, "input");
    app.world_mut().insert(input, Lease(dropped.clone()));
    despawn(app.world_mut(), view);
    assert!(!app.world().is_alive(input));
    assert_eq!(dropped.load(Ordering::SeqCst), 3);
}

#[test]
fn anonymous_controls_survive_markdown_insertion_but_document_the_ordinal_contract() {
    let source = INPUT.replace("id: input\n", "");
    let (mut app, view) = setup(&source);
    let input = app.world().entities_with::<TextField>()[0];
    type_text(&mut app, input, "saved");
    let old = control_value(app.world(), input);
    update_liquid_source(app.world_mut(), view, &format!("Before\n\n{source}\nAfter")).unwrap();
    frames(&mut app);
    assert!(app.world().is_alive(input));
    assert_eq!(control_value(app.world(), input), old);
}
