//! A small mock host. Document authority lives in MemoryStore, not in UI widgets.
use rndr_canvas::{
    Camera, CanvasStore, Content, CreateRequest, MemoryStore, Node, Operation, Scene, Transform,
};
use rndr_canvas_ui::editor::CanvasEditor;
use rndr_canvas_ui::view::{self, CanvasView};
use rndr_core::*;
use rndr_liquid_ui::{
    control_entity, control_value, dispatch, liquid_view, set_liquid_source, update_liquid_source,
    LiquidEvent, LiquidHost, LiquidUiPlugin, LiquidView,
};
use rndr_ui::controls::{Button as UiButton, ButtonProps, HostIntent};
use rndr_ui::{apply_theme_bridge, text, Typography, UiId, UiPlugin, UiTheme};
use std::sync::Arc;
use std::time::Duration;

pub const WIDTH: f32 = 1280.0;
pub const HEIGHT: f32 = 800.0;
pub const STREAM: u64 = 1;
pub const NEXT: u64 = 2;
pub const RESET: u64 = 3;
pub const HOST_VALUE: u64 = 4;
pub const EDIT: u64 = 10;
pub const COMMIT: u64 = 11;
pub const MOVE: u64 = 12;
pub const UNDO: u64 = 13;
pub const REDO: u64 = 14;
pub const ZOOM: u64 = 15;
pub const ROTATE: u64 = 16;

#[derive(Clone, Copy)]
pub struct Fixture {
    pub liquid: Entity,
    pub canvas: Entity,
    status: Entity,
}
#[derive(Default)]
struct HostState {
    stage: usize,
    generation: u64,
    message: String,
    last_event: String,
}
#[derive(Clone)]
struct StreamStep {
    generation: u64,
    stage: usize,
}
impl Event for StreamStep {}
struct MockHost;
impl LiquidHost for MockHost {
    fn event(&mut self, world: &mut World, event: LiquidEvent) {
        println!("Liquid host: {event:?}");
        if let Some(state) = world.resource_mut::<HostState>() {
            state.last_event = format!("{event:?}");
        }
    }
}

/// Full host snapshots; stage 4 deliberately ends inside a Liquid fence.
pub fn liquid_snapshot(stage: usize) -> String {
    let check = "```checkbox\nkey: approved\nlabel: Approved locally\nchecked: false\n```\n";
    let input = "```input\nkey: name\nlabel: Local draft (uncontrolled)\nvalue: Ada\n```\n";
    let slider = format!("```slider\nkey: amount\nlabel: Host amount (controlled)\ncontrolled: true\nvalue: {}\nmin: 0\nmax: 100\n```\n", if stage < 2 { 20 } else { 70 });
    let prefix = match stage {
        0 => "# Stateful Liquid\n\nEdit these controls, then stream.\n\n",
        1 => "# Stateful Liquid\n\nA host streamed Markdown before the controls.\n\n",
        _ => "# Stateful Liquid\n\nStreaming kept entities, drafts and focus.\n\n",
    };
    let middle = if stage == 0 {
        "\n"
    } else {
        "\nNew Markdown between controls.\n\n"
    };
    let mut source = if stage >= 3 {
        format!("{prefix}{slider}{middle}{input}\n{check}")
    } else {
        format!("{prefix}{check}{middle}{input}\n{slider}")
    };
    if stage == 4 {
        source.push_str("\n```callout\nkey: streamed\ntitle: Partial fence");
    }
    if stage >= 5 {
        source.push_str("\n```callout\nkey: streamed\ntitle: Completed fence\nbody: Last valid controls stayed alive while this fence was incomplete.\n```\n");
    }
    if stage >= 6 {
        source = source.replace(check, "").replace(&slider, "```input\nkey: amount\nlabel: Slider changed to input (new state)\nvalue: New type\n```\n");
    }
    source
}

fn label(world: &mut World, parent: Entity, value: &str, role: Typography) -> Entity {
    text(world, parent, value, role).expect("valid label parent")
}
fn row(world: &mut World, parent: Entity) -> Entity {
    let row = world.spawn((Layout {
        width: Size::Fill,
        gap: 8.0,
        ..Layout::row()
    },));
    set_parent(world, row, parent);
    row
}
fn buttons(world: &mut World, parent: Entity, commands: &[(u64, &str)]) {
    for (id, title) in commands {
        UiButton::mount(
            world,
            parent,
            UiId(*id),
            ButtonProps::new(*title, HostIntent::Command(*id)),
        )
        .expect("mount fixture command");
    }
}

pub fn build_app() -> App {
    let mut app = App::new();
    let engine: Arc<dyn TextEngine> = Arc::new(rndr_render::ParleyText::bundled());
    app.world_mut().insert_resource(engine);
    app.add_plugin(UiPlugin);
    app.add_plugin(MarkdownPlugin);
    app.add_plugin(LiquidUiPlugin::new(MockHost));
    apply_theme_bridge(app.world_mut(), UiTheme::default());
    let world = app.world_mut();
    world.insert_resource(Viewport::new(WIDTH, HEIGHT));
    world.insert_resource(HostState::default());
    let root = world.spawn((
        Layout {
            width: Size::Fill,
            height: Size::Fill,
            padding: 16.0,
            gap: 10.0,
            ..Layout::column()
        },
        Color::rgb(0.95, 0.96, 0.98),
    ));
    label(
        world,
        root,
        "Liquid + editable canvas / host integration fixture",
        Typography::Heading,
    );
    let commands = row(world, root);
    buttons(
        world,
        commands,
        &[
            (STREAM, "Stream + focus draft"),
            (NEXT, "Next host step"),
            (HOST_VALUE, "Host value 70"),
            (RESET, "Reset Liquid"),
        ],
    );
    let commands = row(world, root);
    buttons(
        world,
        commands,
        &[
            (EDIT, "Edit text"),
            (COMMIT, "Commit / exit"),
            (MOVE, "Move +40"),
            (ROTATE, "Rotate +15°"),
            (ZOOM, "Zoom / pan"),
            (UNDO, "Undo"),
            (REDO, "Redo"),
        ],
    );
    let body = world.spawn((Layout {
        width: Size::Fill,
        height: Size::Fill,
        min_height: 0.0,
        gap: 16.0,
        ..Layout::row()
    },));
    set_parent(world, body, root);
    let liquid = liquid_view(world, &liquid_snapshot(0));
    world
        .get_mut::<MarkdownView>(liquid)
        .unwrap()
        .set_follow(false);
    if let Some(layout) = world.get_mut::<Layout>(liquid) {
        layout.width = Size::Px(430.0);
        layout.height = Size::Fill;
        layout.min_height = 0.0;
    }
    set_parent(world, liquid, body);
    let canvas_column = world.spawn((Layout {
        width: Size::Fill,
        height: Size::Fill,
        gap: 8.0,
        min_height: 0.0,
        ..Layout::column()
    },));
    set_parent(world, canvas_column, body);
    label(world, canvas_column, "Double-click text or press Edit. Drag/Shift+arrows selects; Ctrl/Cmd+C/X/V.\nEscape or Ctrl/Cmd+Enter commits. Outside editing: arrows move, Ctrl/Cmd+Z undoes.\nWheel pans; Ctrl/Cmd+wheel zooms. Buttons expose the same history.", Typography::Secondary);
    let canvas = world.spawn((
        Layout {
            width: Size::Fill,
            height: Size::Fill,
            min_height: 0.0,
            ..Layout::column()
        },
        SurfaceSlot { id: 100 },
        Focusable,
    ));
    set_parent(world, canvas, canvas_column);
    let store = Arc::new(MemoryStore::new());
    let node = Node::new(
        "text",
        Content::Text {
            text: "Edit the middle of this text".into(),
            color: "#202838".into(),
            font_family: "Inter".into(),
            font_size: 26.0,
            font_weight: None,
            font_style: None,
        },
        Transform {
            x: 60.0,
            y: 100.0,
            width: 470.0,
            height: 80.0,
            rotation: 0.0,
        },
    );
    store
        .create(CreateRequest {
            id: "fixture".into(),
            title: "Integration".into(),
            scene: Scene {
                nodes: vec![node],
                ..Scene::default()
            },
        })
        .expect("create fixture document");
    view::install(
        world,
        CanvasView::new(
            CanvasEditor::open(store, "fixture").expect("open fixture document"),
            100,
        ),
    );
    let status = label(world, root, "Ready", Typography::Secondary);
    world.insert_resource(Fixture {
        liquid,
        canvas,
        status,
    });
    app.add_system_front(Schedule::Input, view::take_wheel);
    app.add_system(Schedule::Update, update);
    app
}

fn focus_draft(world: &mut World) {
    let Some(fixture) = world.resource::<Fixture>().copied() else {
        return;
    };
    let target = world
        .get::<LiquidView>(fixture.liquid)
        .and_then(|v| v.component("name"))
        .and_then(|r| control_entity(world, r));
    if let Some(target) = target {
        request_focus(world, target);
    }
}
fn publish(world: &mut World, stage: usize) {
    let fixture = *world.resource::<Fixture>().unwrap();
    let result = update_liquid_source(world, fixture.liquid, &liquid_snapshot(stage));
    let state = world.resource_mut::<HostState>().unwrap();
    state.stage = stage;
    state.message = format!("Host step {stage}: {result:?}");
    println!("{}", state.message);
}

/// Application-local command adapter. No document text is executed as a command.
pub fn command(world: &mut World, id: u64) {
    let fixture = *world.resource::<Fixture>().unwrap();
    match id {
        STREAM => {
            let state = world.resource_mut::<HostState>().unwrap();
            state.generation += 1;
            let generation = state.generation;
            for stage in 1..=6 {
                after(
                    world,
                    Duration::from_millis(stage as u64 * 900),
                    StreamStep { generation, stage },
                )
                .detach();
            }
            focus_draft(world);
        }
        NEXT => {
            let stage = (world.resource::<HostState>().unwrap().stage + 1).min(6);
            publish(world, stage);
            focus_draft(world);
        }
        HOST_VALUE => {
            publish(world, 2);
            focus_draft(world);
        }
        RESET => {
            let state = world.resource_mut::<HostState>().unwrap();
            state.generation += 1;
            state.stage = 0;
            state.message = "Intentional reset: all Liquid state retired".into();
            set_liquid_source(world, fixture.liquid, &liquid_snapshot(0));
        }
        EDIT => {
            request_focus(world, fixture.canvas);
            let view = world.resource_mut::<CanvasView>().unwrap();
            view.editor.select(&["text".into()]);
            view.editor.edit_text("text");
        }
        COMMIT | MOVE | ROTATE | UNDO | REDO | ZOOM => {
            let view = world.resource_mut::<CanvasView>().unwrap();
            match id {
                COMMIT => {
                    view.editor.finish_text();
                }
                MOVE => {
                    view.editor.finish_text();
                    view.editor.select(&["text".into()]);
                    view.editor.nudge(40.0, 0.0);
                }
                ROTATE => {
                    view.editor.finish_text();
                    let mut node = view.editor.nodes()[0].clone();
                    node.transform.rotation += 15.0;
                    view.editor
                        .commit("Rotate text", vec![Operation::put(node)]);
                }
                UNDO => {
                    view.editor.undo();
                }
                REDO => {
                    view.editor.redo();
                }
                ZOOM => {
                    view.editor.camera = if view.editor.camera.zoom > 1.0 {
                        Camera::default()
                    } else {
                        Camera {
                            pan_x: -30.0,
                            pan_y: 20.0,
                            zoom: 1.3,
                        }
                    };
                }
                _ => unreachable!(),
            }
            request_focus(world, fixture.canvas);
        }
        _ => {}
    }
}

fn update(world: &mut World) {
    for request in take_action_intents(world) {
        if dispatch(world, &request) {
            continue;
        }
        if let Some(HostIntent::Command(id)) = rndr_ui::controls::host_intent(world, &request) {
            command(world, id);
        }
    }
    let steps = events::<StreamStep>(world).to_vec();
    for step in steps {
        if world
            .resource::<HostState>()
            .is_some_and(|s| s.generation == step.generation)
        {
            publish(world, step.stage);
        }
    }
    view::update(world);
    let fixture = *world.resource::<Fixture>().unwrap();
    let state = world.resource::<HostState>().unwrap();
    let view = world.resource::<CanvasView>().unwrap();
    let draft = world
        .get::<LiquidView>(fixture.liquid)
        .and_then(|v| v.component("name"))
        .and_then(|r| control_value(world, r));
    let status = format!("{}\nFocus: {:?} | Liquid draft: {:?}\nCanvas: revision {} | editing {:?} | undo {} / redo {} | zoom {:.2}\nLast control event: {}", state.message,
        world.resource::<Focus>().and_then(|f| f.entity), draft,
        view.editor.mutation().document.revision, view.editor.editing_text(), view.editor.mutation().can_undo, view.editor.mutation().can_redo,
        view.editor.camera.zoom, state.last_event);
    world.get_mut::<Label>(fixture.status).unwrap().text = status;
}
