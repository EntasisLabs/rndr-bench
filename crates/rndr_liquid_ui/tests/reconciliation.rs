use rndr_core::*;
use rndr_liquid_ui::*;

fn setup(source: &str) -> (App, Entity) {
    let mut app = App::new();
    app.add_plugin(MarkdownPlugin);
    app.add_plugin(LiquidUiPlugin::new(NoHost));
    app.world_mut().insert_resource(Viewport::new(720., 1000.));
    let view = liquid_view(app.world_mut(), source);
    rndr_paint::frame(&mut app);
    rndr_paint::frame(&mut app);
    (app, view)
}

fn descendants(world: &World, root: Entity) -> Vec<Entity> {
    let mut out = vec![];
    for child in children_of(world, root) {
        out.push(child);
        out.extend(descendants(world, child));
    }
    out
}

#[test]
fn markdown_insertion_retains_component_entities_and_focus() {
    let source = "```accordion\ntitle: FAQ\n---\nlabel: One\nbody: Answer\n```\n";
    let (mut app, view) = setup(source);
    let target = descendants(app.world(), view)
        .into_iter()
        .find(|e| app.world().has::<Button>(*e))
        .unwrap();
    assert!(request_focus(app.world_mut(), target));
    assert_eq!(
        update_liquid_source(app.world_mut(), view, &format!("Intro\n\n{source}")),
        Ok(LiquidUpdate::Applied)
    );
    rndr_paint::frame(&mut app);
    rndr_paint::frame(&mut app);
    assert!(app.world().is_alive(target));
    assert!(descendants(app.world(), view).contains(&target));
    assert_eq!(
        app.world().resource::<Focus>().unwrap().entity,
        Some(target)
    );
}

#[test]
fn mounted_order_is_document_order_in_the_first_reorder_frame() {
    let (mut app, view) = setup(&source("initial", false, false));
    update_liquid_source(app.world_mut(), view, &source("initial", false, true)).unwrap();
    rndr_paint::frame(&mut app);
    assert_eq!(
        app.world().get::<LiquidView>(view).unwrap().mounted(),
        vec![0, 1, 2]
    );
}

fn source(input: &str, controlled: bool, reverse: bool) -> String {
    let check = "```checkbox\nkey: check\nlabel: Check\nchecked: false\n```\n";
    let input = format!(
        "```input\nkey: input\nlabel: Name\nvalue: {input}\ncontrolled: {controlled}\n```\n"
    );
    let slider = "```slider\nkey: range\nlabel: Amount\nvalue: 20\nmin: 0\nmax: 100\n```\n";
    if reverse {
        format!("{slider}\nBetween\n\n{input}\n{check}")
    } else {
        format!("Before\n\n{check}\n{input}\n{slider}")
    }
}

#[test]
fn nested_control_keeps_identity_when_its_parent_markdown_changes() {
    let source = |title: &str, value: &str| {
        format!("```report\nkey: report\ntitle: {title}\n\nIntro\n\n```input\nkey: nested\nlabel: Nested draft\nvalue: {value}\n```\n\nTail\n```\n")
    };
    let (mut app, view) = setup(&source("Old heading", "initial"));
    for _ in 0..3 {
        rndr_paint::frame(&mut app);
    }
    let input = descendants(app.world(), view)
        .into_iter()
        .find(|e| app.world().has::<TextField>(*e))
        .unwrap();
    assert!(request_focus(app.world_mut(), input));
    app.world_mut().get_mut::<Label>(input).unwrap().text = "local".into();
    update_liquid_source(app.world_mut(), view, &source("New heading", "host")).unwrap();
    for _ in 0..4 {
        rndr_paint::frame(&mut app);
    }
    assert!(app.world().is_alive(input));
    assert_eq!(app.world().get::<Label>(input).unwrap().text, "local");
    assert_eq!(app.world().resource::<Focus>().unwrap().entity, Some(input));
}

fn root(app: &App, view: Entity, id: &str) -> Entity {
    app.world()
        .get::<LiquidView>(view)
        .unwrap()
        .component(id)
        .unwrap()
}

#[test]
fn reset_before_sync_retires_components_omitted_by_a_pending_update() {
    let initial = source("initial", false, false);
    let (mut app, view) = setup(&initial);
    let old_input = root(&app, view, "input");
    let target = control_entity(app.world(), old_input).unwrap();
    assert!(request_focus(app.world_mut(), target));
    update_liquid_source(app.world_mut(), view, "No controls").unwrap();
    set_liquid_source(app.world_mut(), view, &initial);
    assert!(!app.world().is_alive(old_input));
    assert_eq!(app.world().resource::<Focus>().unwrap().entity, None);
    for _ in 0..3 {
        rndr_paint::frame(&mut app);
    }
    assert_ne!(root(&app, view, "input"), old_input);
}

#[test]
fn explicit_controls_retain_drafts_and_focus_across_reordering_and_host_publication() {
    let (mut app, view) = setup(&source("initial", false, false));
    let input_root = root(&app, view, "input");
    let input = control_entity(app.world(), input_root).unwrap();
    assert!(request_focus(app.world_mut(), input));
    app.world_mut().get_mut::<Label>(input).unwrap().text = "local".into();
    app.world_mut().insert(
        input,
        Caret {
            index: 2,
            anchor: 1,
        },
    );
    assert_eq!(
        update_liquid_source(app.world_mut(), view, &source("host", false, true)),
        Ok(LiquidUpdate::Applied)
    );
    rndr_paint::frame(&mut app);
    rndr_paint::frame(&mut app);
    assert_eq!(root(&app, view, "input"), input_root);
    assert_eq!(app.world().get::<Label>(input).unwrap().text, "local");
    assert_eq!(app.world().get::<Caret>(input).unwrap().index, 2);
    assert_eq!(app.world().resource::<Focus>().unwrap().entity, Some(input));
    assert_eq!(
        update_liquid_source(app.world_mut(), view, &source("host", true, false)),
        Ok(LiquidUpdate::Applied)
    );
    rndr_paint::frame(&mut app);
    assert_eq!(app.world().get::<Label>(input).unwrap().text, "host");
    assert_eq!(app.world().resource::<Focus>().unwrap().entity, Some(input));
}

#[test]
fn streaming_and_duplicate_ids_leave_last_valid_ui_intact() {
    let complete = source("initial", false, false);
    let (mut app, view) = setup(&complete);
    let input_root = root(&app, view, "input");
    let input = control_entity(app.world(), input_root).unwrap();
    assert!(request_focus(app.world_mut(), input));
    let incomplete = format!("{complete}\n```input\nkey: new\nvalue: stream");
    assert_eq!(
        update_liquid_source(app.world_mut(), view, &incomplete),
        Ok(LiquidUpdate::Incomplete)
    );
    rndr_paint::frame(&mut app);
    assert_eq!(root(&app, view, "input"), input_root);
    assert_eq!(app.world().resource::<Focus>().unwrap().entity, Some(input));
    let duplicate = format!("{complete}\n```input\nkey: input\nvalue: other\n```\n");
    assert_eq!(
        update_liquid_source(app.world_mut(), view, &duplicate),
        Err(LiquidUpdateError::DuplicateId("input".into()))
    );
    assert_eq!(root(&app, view, "input"), input_root);
    assert_eq!(
        update_liquid_source(app.world_mut(), view, &format!("{incomplete}\n```\n")),
        Ok(LiquidUpdate::Applied)
    );
    rndr_paint::frame(&mut app);
    rndr_paint::frame(&mut app);
    assert_eq!(root(&app, view, "input"), input_root);
    assert!(app
        .world()
        .get::<LiquidView>(view)
        .unwrap()
        .component("new")
        .is_some());
}

struct OwnedResource(std::sync::Arc<std::sync::atomic::AtomicUsize>);
impl Component for OwnedResource {}
impl Drop for OwnedResource {
    fn drop(&mut self) {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

#[test]
fn reset_releases_focus_synchronously() {
    let initial = source("initial", false, false);
    let (mut app, view) = setup(&initial);
    let input = control_entity(app.world(), root(&app, view, "input")).unwrap();
    assert!(request_focus(app.world_mut(), input));
    set_liquid_source(app.world_mut(), view, &initial);
    assert_eq!(app.world().resource::<Focus>().unwrap().entity, None);
}

#[test]
fn synchronous_host_acknowledgement_wins_over_checkbox_draft() {
    struct Ack;
    const DOC: &str =
        "```checkbox\nkey: c\nlabel: Controlled\nchecked: false\ncontrolled: true\n```\n";
    impl LiquidHost for Ack {
        fn event(&mut self, world: &mut World, event: LiquidEvent) {
            if let LiquidEvent::ControlChanged { component, .. } = event {
                let view = world.get::<Parent>(component).unwrap().entity;
                update_liquid_source(world, view, DOC).unwrap();
            }
        }
    }
    let mut app = App::new();
    app.add_plugin(MarkdownPlugin);
    app.add_plugin(LiquidUiPlugin::new(Ack));
    app.world_mut()
        .insert_resource(Viewport::new(720.0, 1000.0));
    let view = liquid_view(app.world_mut(), DOC);
    rndr_paint::frame(&mut app);
    rndr_paint::frame(&mut app);
    let block = root(&app, view, "c");
    let target = control_entity(app.world(), block).unwrap();
    assert_eq!(
        try_activate(app.world_mut(), target),
        ActivationResult::Accepted
    );
    assert_eq!(
        control_value(app.world(), block),
        Some(ControlValue::Checked(false))
    );
    assert_eq!(access_details(app.world(), target).checked, Some(false));
}

#[test]
fn removed_and_incompatible_controls_release_owned_resources_and_reset_is_explicit() {
    let (mut app, view) = setup(&source("initial", false, false));
    let input_root = root(&app, view, "input");
    let input = control_entity(app.world(), input_root).unwrap();
    let check = root(&app, view, "check");
    let range = root(&app, view, "range");
    let drops = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    app.world_mut().insert(input, OwnedResource(drops.clone()));
    app.world_mut().insert(check, OwnedResource(drops.clone()));
    assert!(request_focus(app.world_mut(), input));
    let next = "```checkbox\nkey: input\nlabel: New type\n```\n\n```slider\nkey: range\nlabel: Amount\nvalue: 10\n```\n";
    assert_eq!(
        update_liquid_source(app.world_mut(), view, next),
        Ok(LiquidUpdate::Applied)
    );
    rndr_paint::frame(&mut app);
    rndr_paint::frame(&mut app);
    assert!(!app.world().is_alive(input_root));
    assert!(!app.world().is_alive(check));
    assert_ne!(root(&app, view, "input"), input_root);
    assert_eq!(root(&app, view, "range"), range);
    assert_eq!(drops.load(std::sync::atomic::Ordering::SeqCst), 2);
    assert_eq!(app.world().resource::<Focus>().unwrap().entity, None);
    set_liquid_source(app.world_mut(), view, next);
    rndr_paint::frame(&mut app);
    rndr_paint::frame(&mut app);
    assert!(!app.world().is_alive(range));
    assert_ne!(root(&app, view, "range"), range);
}

#[test]
fn checkbox_and_slider_drafts_survive_and_controlled_publication_is_repeatable() {
    let initial = source("initial", false, false);
    let (mut app, view) = setup(&initial);
    let check = root(&app, view, "check");
    let checkbox = control_entity(app.world(), check).unwrap();
    assert_eq!(
        try_activate(app.world_mut(), checkbox),
        ActivationResult::Accepted
    );
    let range = root(&app, view, "range");
    let slider = control_entity(app.world(), range).unwrap();
    assert!(request_focus(app.world_mut(), slider));
    app.world_mut()
        .resource_mut::<Input>()
        .unwrap()
        .keys
        .push(KeyEvent::named(NamedKey::ArrowRight, Modifiers::default()));
    rndr_paint::frame(&mut app);
    assert_eq!(
        control_value(app.world(), check),
        Some(ControlValue::Checked(true))
    );
    assert_eq!(
        control_value(app.world(), range),
        Some(ControlValue::Number(21.0))
    );
    update_liquid_source(app.world_mut(), view, &source("other", false, true)).unwrap();
    rndr_paint::frame(&mut app);
    assert_eq!(
        control_value(app.world(), check),
        Some(ControlValue::Checked(true))
    );
    assert_eq!(
        control_value(app.world(), range),
        Some(ControlValue::Number(21.0))
    );
    let controlled = initial
        .replace("checked: false", "checked: false\ncontrolled: true")
        .replace("value: 20", "value: 20\ncontrolled: true");
    update_liquid_source(app.world_mut(), view, &controlled).unwrap();
    rndr_paint::frame(&mut app);
    assert_eq!(
        control_value(app.world(), check),
        Some(ControlValue::Checked(false))
    );
    assert_eq!(
        control_value(app.world(), range),
        Some(ControlValue::Number(20.0))
    );
    assert_eq!(
        try_activate(app.world_mut(), checkbox),
        ActivationResult::Accepted
    );
    update_liquid_source(app.world_mut(), view, &controlled).unwrap();
    rndr_paint::frame(&mut app);
    assert_eq!(
        control_value(app.world(), check),
        Some(ControlValue::Checked(false))
    );
}
