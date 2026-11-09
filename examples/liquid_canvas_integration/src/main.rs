//! Mock host integrating update-safe Liquid and the transaction-backed canvas.
use rndr_canvas::{
    Camera, CanvasStore, Content, CreateRequest, MemoryStore, Node, Scene, Transform,
};
use rndr_canvas_ui::{
    editor::CanvasEditor,
    panels,
    view::{self, CanvasView},
};
use rndr_core::{
    set_parent, take_action_intents, App, Entity, Event, Focus, Label, Layout, LogicalSize,
    MarkdownPlugin, MarkdownView, Schedule, Size, SurfaceSlot, TextEngine, TimerHandle, Viewport,
    WindowConfig, World,
};
use rndr_liquid_ui::{
    dispatch, liquid_view, set_liquid_source, update_liquid_source, LiquidEvent, LiquidHost,
    LiquidUiPlugin,
};
use rndr_render::{encode_png, Exporter};
use rndr_ui::{
    apply_theme_bridge,
    controls::{Button, ButtonProps, HostIntent},
    Typography, UiId, UiPlugin, UiTheme,
};
use std::{sync::Arc, time::Duration};

const WIDTH: f32 = 1500.0;
const HEIGHT: f32 = 960.0;
const SLOT: u64 = 91;
const COMMAND_BASE: u64 = 900_000;
const STAGES: [&str; 8] = [
    "0 / Initial: edit the controls, then focus the input",
    "1 / Stream Markdown before and between controls",
    "2 / Host data: uncontrolled local values stay; slider becomes 80",
    "3 / Explicit IDs reordered: input, slider, checkbox",
    "4 / Incomplete fence: last valid UI is retained",
    "5 / Fence complete: stream accepted",
    "6 / Remove checkbox; slider ID becomes an input (reset)",
    "7 / Intentional replacement: all local control state resets",
];

/// Deterministic simulated host submissions. No live service is involved.
fn document(stage: usize) -> String {
    let checkbox = "```checkbox\nid: accepted\nlabel: Keep my local checkbox\nvalue: false\n```\n";
    let input = format!(
        "```input\nid: draft\nlabel: Uncontrolled draft\nvalue: {}\n```\n",
        if (2..7).contains(&stage) {
            "host attempted replacement"
        } else {
            "Edit this draft"
        }
    );
    let slider = format!("```slider\nid: volume\nlabel: Host-controlled level\nvalue: {}\nmin: 0\nmax: 100\nstep: 1\ncontrolled: true\n```\n", if (2..7).contains(&stage) {80} else {30});
    let header = "# Stateful Liquid\n\nNative controls inside an incrementally updated Markdown document.\n\n";
    let before = if (1..7).contains(&stage) {
        "**Streamed before the controls.**\n\n"
    } else {
        ""
    };
    let between = if (1..7).contains(&stage) {
        "\n_Streaming text between native controls._\n\n"
    } else {
        "\n"
    };
    let body = if stage == 6 {
        format!("{input}{between}```input\nid: volume\nlabel: Former slider: incompatible state reset\nvalue: New input state\n```\n")
    } else if (3..6).contains(&stage) {
        format!("{input}{between}{slider}{between}{checkbox}")
    } else {
        format!("{checkbox}{between}{input}{between}{slider}")
    };
    let mut source = format!("{header}{before}{body}\nCheckbox and draft are **uncontrolled**; the slider is **controlled**.\n");
    if stage == 4 {
        source.push_str("\n```callout\nid: streamed-note\ntitle: Incomplete");
    }
    if stage == 5 {
        source.push_str("\n```callout\nid: streamed-note\ntitle: Stream complete\nbody: The last valid controls survived the partial submission.\n```\n");
    }
    source
}

struct MockHost;
impl LiquidHost for MockHost {
    fn event(&mut self, _: &mut World, event: LiquidEvent) {
        println!("Liquid event: {event:?}");
    }
}

#[derive(Clone)]
struct StreamTick;
impl Event for StreamTick {}

struct Fixture {
    liquid: Entity,
    status: Entity,
    stage: usize,
    outcome: String,
    stream_timer: Option<TimerHandle>,
}

fn row(world: &mut World, parent: Entity) -> Entity {
    let entity = world.spawn((Layout {
        gap: 8.0,
        width: Size::Fill,
        ..Layout::row()
    },));
    set_parent(world, entity, parent);
    entity
}
fn column(world: &mut World, parent: Entity, width: Size) -> Entity {
    let entity = world.spawn((Layout {
        gap: 8.0,
        width,
        height: Size::Fill,
        min_height: 0.0,
        min_width: 0.0,
        ..Layout::column()
    },));
    set_parent(world, entity, parent);
    entity
}
fn button(world: &mut World, parent: Entity, index: u64, label: &str) {
    Button::mount(
        world,
        parent,
        UiId(COMMAND_BASE + index),
        ButtonProps::new(label, HostIntent::Command(COMMAND_BASE + index)),
    )
    .expect("fixture button");
}

fn store() -> Arc<dyn CanvasStore + Send + Sync> {
    let store = Arc::new(MemoryStore::new());
    store.create(CreateRequest {
        id: "integration".into(), title: "Integration canvas".into(),
        scene: Scene {
            nodes: vec![Node::new("editable", Content::Text {
                text: "Edit the middle of this canvas text.\nSelect, copy, paste, then commit.".into(),
                color: "#172b4d".into(), font_family: "Inter".into(), font_size: 24.0,
                font_weight: None, font_style: None,
            }, Transform { x: 70.0, y: 100.0, width: 430.0, height: 110.0, rotation: 0.0 })],
            ..Scene::default()
        },
    }).expect("create fixture document");
    store
}

fn apply_stage(world: &mut World, stage: usize) {
    let entity = world.resource::<Fixture>().unwrap().liquid;
    let source = document(stage);
    let outcome = if stage == 7 {
        set_liquid_source(world, entity, &source);
        "Intentional reset".into()
    } else {
        format!("{:?}", update_liquid_source(world, entity, &source))
    };
    let fixture = world.resource_mut::<Fixture>().unwrap();
    fixture.stage = stage;
    fixture.outcome = outcome;
    println!("{}: {}", STAGES[stage], fixture.outcome);
}

fn command(world: &mut World, index: u64) {
    match index {
        0 => {
            let stage = (world.resource::<Fixture>().unwrap().stage + 1).min(7);
            apply_stage(world, stage);
        }
        1 => apply_stage(world, 7),
        2 => {
            apply_stage(world, 0);
            let timer = rndr_core::every(world, Duration::from_secs(5), StreamTick);
            world.resource_mut::<Fixture>().unwrap().stream_timer = Some(timer);
        }
        3 => world.resource_mut::<Fixture>().unwrap().stream_timer = None,
        10 => {
            world
                .resource_mut::<CanvasView>()
                .unwrap()
                .editor
                .edit_text("editable");
            let entity = world
                .entities_with::<SurfaceSlot>()
                .into_iter()
                .find(|entity| {
                    world
                        .get::<SurfaceSlot>(*entity)
                        .is_some_and(|slot| slot.id == SLOT)
                });
            world.insert_resource(Focus { entity });
        }
        11 => {
            world
                .resource_mut::<CanvasView>()
                .unwrap()
                .editor
                .finish_text();
        }
        12 => {
            world.resource_mut::<CanvasView>().unwrap().editor.undo();
        }
        13 => {
            world.resource_mut::<CanvasView>().unwrap().editor.redo();
        }
        14 => {
            world
                .resource_mut::<CanvasView>()
                .unwrap()
                .editor
                .zoom_by(1.25, [700.0, 650.0]);
        }
        15 => {
            world
                .resource_mut::<CanvasView>()
                .unwrap()
                .editor
                .zoom_by(0.8, [700.0, 650.0]);
        }
        16 => {
            world
                .resource_mut::<CanvasView>()
                .unwrap()
                .editor
                .camera
                .pan_x += 30.0;
        }
        17 => {
            let editor = &mut world.resource_mut::<CanvasView>().unwrap().editor;
            editor.finish_text();
            editor.select(&["editable".into()]);
            editor.nudge(30.0, 15.0);
        }
        18 => {
            world.resource_mut::<CanvasView>().unwrap().editor.camera = Camera::default();
        }
        _ => {}
    }
}

fn refresh_status(world: &mut World) {
    let fixture = world.resource::<Fixture>().unwrap();
    let view = world.resource::<CanvasView>().unwrap();
    let editor = &view.editor;
    let text = format!("{}\n{} | auto-stream {}\nCanvas: edit {:?} | revision {} | undo {} redo {} | zoom {:.2} pan {:.0},{:.0}\nLast canvas error: {:?}",
        STAGES[fixture.stage], fixture.outcome, if fixture.stream_timer.is_some() {"ON (5s)"} else {"off"},
        editor.editing_text(), editor.mutation().document.revision, editor.mutation().can_undo, editor.mutation().can_redo,
        editor.camera.zoom, editor.camera.pan_x, editor.camera.pan_y, editor.last_error());
    let entity = fixture.status;
    if let Some(label) = world.get_mut::<Label>(entity) {
        label.text = text;
    }
}

fn app() -> App {
    app_with_theme(UiTheme::default())
}
fn app_with_theme(theme: UiTheme) -> App {
    let mut app = App::new();
    let engine: Arc<dyn TextEngine> = Arc::new(rndr_render::ParleyText::bundled());
    app.world_mut().insert_resource(engine);
    app.add_plugin(UiPlugin);
    app.add_plugin(MarkdownPlugin);
    app.add_plugin(LiquidUiPlugin::new(MockHost));
    apply_theme_bridge(app.world_mut(), theme);
    let world = app.world_mut();
    world.insert_resource(Viewport::new(WIDTH, HEIGHT));
    let root = world.spawn((Layout {
        width: Size::Fill,
        height: Size::Fill,
        gap: 12.0,
        ..Layout::column()
    },));
    // Match the native renderer's entire window to the chosen semantic palette.
    // A transparent root otherwise exposes its dark clear behind light-theme text.
    let theme = world.resource::<UiTheme>().unwrap();
    world.insert(root, theme.color(rndr_ui::Token::Canvas));
    world.get_mut::<Layout>(root).unwrap().padding = 12.0;
    rndr_ui::text(
        world,
        root,
        "Liquid + Canvas / production integration fixture",
        Typography::Title,
    )
    .unwrap();
    let controls = row(world, root);
    button(world, controls, 0, "Next host update");
    button(world, controls, 1, "Reset Liquid");
    button(world, controls, 2, "Auto stream (5s)");
    button(world, controls, 3, "Stop stream");
    let status = rndr_ui::text(world, root, "Starting fixture", Typography::Secondary).unwrap();
    let body = row(world, root);
    world.get_mut::<Layout>(body).unwrap().height = Size::Fill;
    let left = column(world, body, Size::Px(500.0));
    let liquid = liquid_view(world, &document(0));
    set_parent(world, liquid, left);
    world
        .get_mut::<MarkdownView>(liquid)
        .unwrap()
        .set_follow(false);
    if let Some(layout) = world.get_mut::<Layout>(liquid) {
        layout.height = Size::Fill;
        layout.width = Size::Fill;
        layout.min_height = 0.0;
    }
    let right = column(world, body, Size::Fill);
    rndr_ui::text(world, right, "Canvas text: double-click to edit; Escape/Commit to finish. Drag selected text as an object after committing.", Typography::Secondary).unwrap();
    let edit = row(world, right);
    for (index, label) in [
        (10, "Edit text"),
        (11, "Commit / exit"),
        (12, "Undo"),
        (13, "Redo"),
        (17, "Move object"),
    ] {
        button(world, edit, index, label);
    }
    let camera = row(world, right);
    for (index, label) in [
        (14, "Zoom +"),
        (15, "Zoom -"),
        (16, "Pan +30"),
        (18, "Reset camera"),
    ] {
        button(world, camera, index, label);
    }
    let toolbar = column(world, right, Size::Fill);
    // Toolbar deliberately has a bounded scrolling viewport: its responsive
    // fallback stacks all actions rather than hiding them in narrow panes.
    world.get_mut::<Layout>(toolbar).unwrap().height = Size::Px(96.0);
    world.insert(toolbar, rndr_core::Scroll::default());
    let canvas_body = row(world, right);
    world.get_mut::<Layout>(canvas_body).unwrap().height = Size::Fill;
    let canvas = column(world, canvas_body, Size::Fill);
    world.insert(canvas, SurfaceSlot { id: SLOT });
    let side = column(world, canvas_body, Size::Px(210.0));
    world.insert(side, rndr_core::Scroll::default());
    let editor = CanvasEditor::open(store(), "integration").unwrap();
    view::install(world, CanvasView::new(editor, SLOT));
    panels::mount(world, toolbar, side, [WIDTH - 508.0, 210.0]);
    world.insert_resource(Fixture {
        liquid,
        status,
        stage: 0,
        outcome: "Initial document".into(),
        stream_timer: None,
    });
    app.add_system_front(Schedule::Input, view::take_wheel);
    app.add_system(Schedule::Update, |world: &mut World| {
        for request in take_action_intents(world) {
            if let Some(HostIntent::Command(id)) = rndr_ui::controls::host_intent(world, &request) {
                if (COMMAND_BASE..COMMAND_BASE + 100).contains(&id) {
                    command(world, id - COMMAND_BASE);
                    continue;
                }
            }
            if !panels::dispatch(world, &request) {
                dispatch(world, &request);
            }
        }
        let due = world.resource::<Fixture>().unwrap().stream_timer.is_some()
            && !rndr_core::events::<StreamTick>(world).is_empty();
        if due {
            let stage = (world.resource::<Fixture>().unwrap().stage + 1).min(7);
            apply_stage(world, stage);
            if stage == 7 {
                world.resource_mut::<Fixture>().unwrap().stream_timer = None;
            }
        }
        view::update(world);
        panels::refresh(world);
        refresh_status(world);
    });
    app
}

fn settle(app: &mut App) {
    for _ in 0..3 {
        rndr_paint::frame(app);
    }
}
fn export(path: &str, focus: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = app();
    settle(&mut app);
    if let Some(id) = focus {
        let view = app.world().resource::<Fixture>().unwrap().liquid;
        let control = app
            .world()
            .get::<rndr_liquid_ui::LiquidView>(view)
            .and_then(|view| view.control(app.world(), id))
            .ok_or("unknown --export-focus control ID")?;
        rndr_core::request_focus(app.world_mut(), control);
        settle(&mut app);
    }
    let frame = rndr_paint::frame(&mut app);
    let image = Exporter::new()?.render_frame(
        Some(app.world()),
        &frame,
        WIDTH as u32,
        HEIGHT as u32,
        1.0,
    )?;
    std::fs::write(path, encode_png(&image)?)?;
    println!("wrote {path}: {}x{}", image.width, image.height);
    Ok(())
}
fn headless() {
    let mut app = app();
    settle(&mut app);
    for stage in 1..=7 {
        apply_stage(app.world_mut(), stage);
        settle(&mut app);
    }
    command(app.world_mut(), 10);
    assert_eq!(
        app.world()
            .resource::<CanvasView>()
            .unwrap()
            .editor
            .editing_text(),
        Some("editable")
    );
    command(app.world_mut(), 11);
    command(app.world_mut(), 17);
    command(app.world_mut(), 12);
    command(app.world_mut(), 13);
    settle(&mut app);
    assert!(app
        .world()
        .resource::<CanvasView>()
        .unwrap()
        .editor
        .last_error()
        .is_none());
    println!("Headless integrated lifecycle smoke passed (not native input verification)");
}
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--headless") {
        headless();
        return;
    }
    if let Some(path) = args
        .iter()
        .position(|arg| arg == "--export")
        .and_then(|index| args.get(index + 1))
    {
        let focus = args
            .iter()
            .position(|arg| arg == "--export-focus")
            .and_then(|index| args.get(index + 1))
            .map(String::as_str);
        if let Err(error) = export(path, focus) {
            eprintln!("export: {error}");
            std::process::exit(1);
        }
        return;
    }
    let mut app = app();
    app.world_mut().insert_resource(WindowConfig {
        title: "Liquid + Canvas integration".into(),
        size: LogicalSize {
            width: WIDTH,
            height: HEIGHT,
        },
        ..WindowConfig::default()
    });
    rndr_render::run(app);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixture_has_explicit_readable_background_and_unclipped_chrome() {
        let mut app = app();
        settle(&mut app);
        let world = app.world();
        let theme = world.resource::<UiTheme>().unwrap();
        let root = world
            .entities_with::<Layout>()
            .into_iter()
            .find(|entity| {
                !world.has::<rndr_core::Parent>(*entity)
                    && world.get::<Layout>(*entity).is_some_and(|layout| {
                        layout.width == Size::Fill && layout.height == Size::Fill
                    })
            })
            .unwrap();
        assert_eq!(
            world.get::<rndr_core::Color>(root),
            Some(&theme.color(rndr_ui::Token::Canvas))
        );
        let engine = world.resource::<Arc<dyn TextEngine>>().unwrap();
        for entity in world.entities_with::<Label>() {
            let label = world.get::<Label>(entity).unwrap();
            if label.text.starts_with("Liquid + Canvas")
                || label.text.starts_with("0 / Initial")
                || label.text.starts_with("Canvas text:")
                || label.text == "Uncontrolled draft"
                || label.text == "Host-controlled level"
            {
                let rect = world.get::<rndr_core::Rect>(entity).unwrap();
                let layout = world.get::<Layout>(entity).unwrap();
                let style = world.get::<rndr_core::TextStyle>(entity).unwrap();
                let measured =
                    engine.measure(&label.text, style, &[], rect.w - layout.padding * 2.0);
                assert!(
                    measured.height <= rect.h - layout.padding * 2.0 + 0.5,
                    "clipped {:?}: measured {} in {:?}",
                    label.text,
                    measured.height,
                    rect
                );
            }
        }
    }

    #[test]
    fn input_and_slider_are_readable_and_focus_visible_in_both_palettes() {
        use rndr_core::{Border, Color};
        use rndr_ui::{Appearance, Token};
        fn luminance(color: Color) -> f32 {
            let linear = |value: f32| {
                if value <= 0.04045 {
                    value / 12.92
                } else {
                    ((value + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b)
        }
        fn contrast(a: Color, b: Color) -> f32 {
            let a = luminance(a);
            let b = luminance(b);
            (a.max(b) + 0.05) / (a.min(b) + 0.05)
        }
        for appearance in [Appearance::Light, Appearance::Dark] {
            let theme = UiTheme {
                appearance,
                ..UiTheme::default()
            };
            let mut app = app_with_theme(theme.clone());
            settle(&mut app);
            let liquid = app.world().resource::<Fixture>().unwrap().liquid;
            for id in ["draft", "volume"] {
                let editor = app
                    .world()
                    .get::<rndr_liquid_ui::LiquidView>(liquid)
                    .unwrap()
                    .control(app.world(), id)
                    .unwrap();
                assert!(rndr_core::request_focus(app.world_mut(), editor));
                rndr_paint::frame(&mut app);
                let world = app.world();
                let fill = *world.get::<Color>(editor).unwrap();
                let label = world.get::<Label>(editor).unwrap();
                assert_eq!(fill, theme.color(Token::Content));
                assert!(contrast(label.color.unwrap(), fill) >= 4.5);
                let border = world.get::<Border>(editor).unwrap();
                assert_eq!(border.color, theme.color(Token::Focus));
                assert_eq!(border.width, theme.focus_width());
                assert!(contrast(border.color, fill) >= 3.0);
                let rect = world.get::<rndr_core::Rect>(editor).unwrap();
                let layout = world.get::<Layout>(editor).unwrap();
                let style = world.get::<rndr_core::TextStyle>(editor).unwrap();
                let engine = world.resource::<Arc<dyn TextEngine>>().unwrap();
                let measured =
                    engine.measure(&label.text, style, &[], rect.w - 2.0 * layout.padding);
                assert!(measured.height <= rect.h - 2.0 * layout.padding + 0.5);
                if id == "volume" {
                    assert!(
                        label.text.is_ascii(),
                        "slider must not rely on optional symbol glyphs"
                    );
                    assert!(label.text.ends_with("30.00"));
                }
            }
            let world = app.world();
            let status = world.resource::<Fixture>().unwrap().status;
            let color = world.get::<Label>(status).unwrap().color.unwrap();
            assert!(contrast(color, theme.color(Token::Canvas)) >= 4.5);
        }
    }

    #[test]
    fn both_slices_share_one_application() {
        headless();
    }
    #[test]
    fn liquid_acceptance_updates_preserve_local_draft_identity_and_focus() {
        use rndr_liquid_ui::{control_value, LiquidValue, LiquidView};
        let mut app = app();
        settle(&mut app);
        let view = app.world().resource::<Fixture>().unwrap().liquid;
        let control = |app: &App, id: &str| {
            app.world()
                .get::<LiquidView>(view)
                .unwrap()
                .control(app.world(), id)
        };
        let draft = control(&app, "draft").expect("draft mounted");
        let slider = control(&app, "volume").expect("slider mounted");
        // Seed a local field draft through core's supported draft owner.
        rndr_core::reset_field_draft(app.world_mut(), draft, "local draft");
        app.world_mut().insert_resource(Focus {
            entity: Some(draft),
        });
        for stage in 1..=5 {
            apply_stage(app.world_mut(), stage);
            if stage == 4 {
                assert!(app
                    .world()
                    .resource::<Fixture>()
                    .unwrap()
                    .outcome
                    .contains("Deferred"));
            }
            settle(&mut app);
            assert_eq!(control(&app, "draft"), Some(draft));
            assert_eq!(control(&app, "volume"), Some(slider));
            assert_eq!(
                control_value(app.world(), draft),
                Some(LiquidValue::Text("local draft".into()))
            );
            assert_eq!(
                app.world()
                    .resource::<Focus>()
                    .and_then(|focus| focus.entity),
                Some(draft)
            );
        }
        assert_eq!(
            control_value(app.world(), slider),
            Some(LiquidValue::Number(80.0))
        );
        apply_stage(app.world_mut(), 6);
        settle(&mut app);
        assert_eq!(control(&app, "accepted"), None);
        assert!(!app.world().is_alive(slider));
        assert_ne!(control(&app, "volume"), Some(slider));
        assert_eq!(control(&app, "draft"), Some(draft));
        apply_stage(app.world_mut(), 7);
        settle(&mut app);
        assert!(!app.world().is_alive(draft));
        let reset_draft = control(&app, "draft").unwrap();
        assert_eq!(
            control_value(app.world(), reset_draft),
            Some(LiquidValue::Text("Edit this draft".into()))
        );
    }
    #[test]
    fn canvas_edit_input_routing_and_history_operate_in_the_combined_world() {
        fn node(app: &App) -> Node {
            app.world()
                .resource::<CanvasView>()
                .unwrap()
                .editor
                .mutation()
                .document
                .scene
                .nodes[0]
                .clone()
        }
        let mut app = app();
        settle(&mut app);
        let original = node(&app);
        command(app.world_mut(), 10);
        app.world_mut().insert_resource(rndr_core::Input {
            chars: "inserted via native adapter".into(),
            ..Default::default()
        });
        rndr_paint::frame(&mut app);
        // A host update must not commit or discard the active canvas draft.
        apply_stage(app.world_mut(), 3);
        settle(&mut app);
        assert_eq!(
            app.world()
                .resource::<CanvasView>()
                .unwrap()
                .editor
                .editing_text(),
            Some("editable")
        );
        command(app.world_mut(), 11);
        let edited = node(&app);
        assert_ne!(edited.content, original.content);
        command(app.world_mut(), 17);
        let moved = node(&app);
        assert_ne!(moved.transform, edited.transform);
        command(app.world_mut(), 12);
        assert_eq!(node(&app), edited);
        command(app.world_mut(), 12);
        assert_eq!(node(&app), original);
        command(app.world_mut(), 13);
        assert_eq!(node(&app), edited);
        command(app.world_mut(), 13);
        assert_eq!(node(&app), moved);
    }
    #[test]
    fn both_workflows_have_visible_layout_and_timed_stream_uses_native_wakes() {
        let mut app = app();
        settle(&mut app);
        let frame = rndr_paint::frame(&mut app);
        let slot = frame.slots.iter().find(|slot| slot.id == SLOT).unwrap();
        assert!(
            slot.rect.w > 400.0 && slot.rect.h > 300.0,
            "canvas slot {:?}",
            slot.rect
        );
        assert!(slot.rect.x >= 500.0 && slot.rect.y >= 0.0);
        assert!(slot.rect.x + slot.rect.w <= WIDTH + 0.1);
        assert!(slot.rect.y + slot.rect.h <= HEIGHT + 0.1);
        command(app.world_mut(), 2);
        assert!(app
            .world()
            .resource::<Fixture>()
            .unwrap()
            .stream_timer
            .is_some());
        rndr_core::send(app.world_mut(), StreamTick);
        rndr_paint::frame(&mut app);
        assert_eq!(app.world().resource::<Fixture>().unwrap().stage, 1);
        command(app.world_mut(), 3);
        assert!(app
            .world()
            .resource::<Fixture>()
            .unwrap()
            .stream_timer
            .is_none());
    }
    #[test]
    fn canvas_manipulation_history_survives_liquid_updates() {
        let mut app = app();
        settle(&mut app);
        let original = app
            .world()
            .resource::<CanvasView>()
            .unwrap()
            .editor
            .mutation()
            .document
            .scene
            .nodes[0]
            .transform;
        command(app.world_mut(), 17);
        let moved = app
            .world()
            .resource::<CanvasView>()
            .unwrap()
            .editor
            .mutation()
            .document
            .scene
            .nodes[0]
            .transform;
        assert_ne!(original, moved);
        apply_stage(app.world_mut(), 3);
        settle(&mut app);
        command(app.world_mut(), 12);
        assert_eq!(
            app.world()
                .resource::<CanvasView>()
                .unwrap()
                .editor
                .mutation()
                .document
                .scene
                .nodes[0]
                .transform,
            original
        );
        command(app.world_mut(), 13);
        assert_eq!(
            app.world()
                .resource::<CanvasView>()
                .unwrap()
                .editor
                .mutation()
                .document
                .scene
                .nodes[0]
                .transform,
            moved
        );
    }
}
