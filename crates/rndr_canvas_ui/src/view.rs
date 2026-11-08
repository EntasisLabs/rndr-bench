//! Runs a [`CanvasEditor`] inside an `rndr_core` world: reads pointer, pen,
//! wheel and key input over a [`SurfaceSlot`], and paints the scene and
//! overlay into that slot.

use crate::editor::{
    self, CanvasEditor, CursorHint, EditorInput, Key, Modifiers, PointerInput, Sample,
};
use crate::overlay::{paint_overlay, OverlayTheme};
use crate::paint::{affine, paint_scene, EngineMeasure, ImageIds};
use rndr_canvas::text::TextMeasure;
use rndr_canvas::Vec2;
use rndr_core::{
    platform_of, Color, CursorStyle, Input, LogicalKey, MotionWatch, NamedKey, PointerButton,
    PointerKind as CoreKind, Pointers, Rect, SurfaceSlot, World,
};
use rndr_paint::{Canvas, CanvasPainter, CanvasRegistry, FillRule, Path};

/// The editor and what painting it needs. Insert it as a resource with
/// [`install`].
pub struct CanvasView {
    pub editor: CanvasEditor,
    /// Texture ids for the document's image assets.
    pub images: ImageIds,
    pub overlay: OverlayTheme,
    /// Painted under the scene across the whole slot, so a transparent scene
    /// background still reads as a canvas. Transparent shows the window.
    pub backdrop: Color,
    /// The [`SurfaceSlot`] id the canvas is laid out in.
    pub slot: u64,
    captured: bool,
    wheel: [f32; 2],
}

impl CanvasView {
    pub fn new(editor: CanvasEditor, slot: u64) -> Self {
        Self {
            editor,
            images: ImageIds::new(),
            overlay: OverlayTheme::default(),
            backdrop: Color::rgb(0.976, 0.98, 0.984),
            slot,
            captured: false,
            wheel: [0.0, 0.0],
        }
    }
}

/// Inserts `view` and registers its painter for `view.slot`. Spawn a
/// [`SurfaceSlot`] with that id where the canvas belongs, run
/// [`take_wheel`] at the front of the Input schedule and [`update`] each
/// frame.
pub fn install(world: &mut World, view: CanvasView) {
    let slot = view.slot;
    world.insert_resource(view);
    if world.resource::<CanvasRegistry>().is_none() {
        world.insert_resource(CanvasRegistry::new());
    }
    if let Some(registry) = world.resource_mut::<CanvasRegistry>() {
        registry.insert(slot, ViewPainter);
    }
    world.insert_resource(MotionWatch { active: true });
}

fn measure(world: &World) -> Box<dyn TextMeasure> {
    Box::new(EngineMeasure(rndr_core::text_engine(world)))
}

pub(crate) fn slot_rect(world: &World, slot: u64) -> Option<Rect> {
    world
        .entities_with::<SurfaceSlot>()
        .into_iter()
        .find(|e| world.get::<SurfaceSlot>(*e).is_some_and(|s| s.id == slot))
        .and_then(|e| world.get::<Rect>(e).copied())
}

pub(crate) fn key(logical: &LogicalKey) -> Option<Key> {
    Some(match logical {
        LogicalKey::Named(NamedKey::Escape) => Key::Escape,
        LogicalKey::Named(NamedKey::Enter) => Key::Enter,
        LogicalKey::Named(NamedKey::Backspace) => Key::Backspace,
        LogicalKey::Named(NamedKey::Delete) => Key::Delete,
        LogicalKey::Named(NamedKey::ArrowLeft) => Key::Left,
        LogicalKey::Named(NamedKey::ArrowRight) => Key::Right,
        LogicalKey::Named(NamedKey::ArrowUp) => Key::Up,
        LogicalKey::Named(NamedKey::ArrowDown) => Key::Down,
        LogicalKey::Character(c) => Key::Char(c.to_ascii_lowercase()),
        _ => return None,
    })
}

/// Builds this frame's [`EditorInput`] from the world. The pointer counts
/// while it is over `rect` or while a press that began there is held.
pub fn editor_input(world: &World, rect: Rect, captured: bool) -> EditorInput {
    let input = world.resource::<Input>().cloned().unwrap_or_default();
    let platform = platform_of(world);
    let origin = [rect.x, rect.y];
    let local = |p: [f32; 2]| [p[0] - origin[0], p[1] - origin[1]];
    let inside = |p: [f32; 2]| {
        p[0] >= rect.x && p[1] >= rect.y && p[0] < rect.x + rect.w && p[1] < rect.y + rect.h
    };
    let pen = world.resource::<Pointers>().and_then(|pointers| {
        pointers
            .iter()
            .find(|c| matches!(c.kind, CoreKind::Pen | CoreKind::Eraser) && (c.down || c.released))
            .cloned()
    });
    let pointer = match pen {
        Some(contact) => Some(PointerInput {
            position: local(contact.position),
            pressed: contact.pressed,
            down: contact.down,
            released: contact.released,
            button: editor::Button::Primary,
            kind: if contact.kind == CoreKind::Eraser {
                editor::PointerKind::Eraser
            } else {
                editor::PointerKind::Pen
            },
            samples: contact
                .samples
                .iter()
                .map(|s| Sample {
                    position: local(s.position),
                    pressure: s.pressure,
                    tilt: s.tilt,
                    time_ms: s.time.as_secs_f64() * 1000.0,
                })
                .collect(),
        }),
        None => {
            let p = input.pointer;
            let touch = world
                .resource::<Pointers>()
                .and_then(|ps| ps.get(p.id).map(|c| c.kind == CoreKind::Touch))
                .unwrap_or(false);
            let samples = world
                .resource::<Pointers>()
                .and_then(|ps| ps.get(p.id))
                .map(|c| {
                    c.samples
                        .iter()
                        .map(|s| Sample {
                            position: local(s.position),
                            pressure: s.pressure,
                            tilt: s.tilt,
                            time_ms: s.time.as_secs_f64() * 1000.0,
                        })
                        .collect()
                })
                .unwrap_or_default();
            Some(PointerInput {
                position: local(p.position),
                pressed: p.pressed,
                down: p.down,
                released: p.released,
                button: match p.button {
                    PointerButton::Middle => editor::Button::Middle,
                    PointerButton::Right => editor::Button::Secondary,
                    _ => editor::Button::Primary,
                },
                kind: if touch {
                    editor::PointerKind::Touch
                } else {
                    editor::PointerKind::Mouse
                },
                samples,
            })
        }
    };
    let pointer = pointer.filter(|p| {
        let window = [p.position[0] + origin[0], p.position[1] + origin[1]];
        if p.pressed {
            inside(window)
        } else {
            captured || inside(window)
        }
    });
    let over = pointer.is_some();
    let modifiers = Modifiers {
        shift: input.shift,
        alt: input.alt,
        primary: if platform.primary_is_super() {
            input.super_key
        } else {
            input.ctrl
        },
    };
    EditorInput {
        pointer,
        scroll: if over { input.scroll } else { [0.0, 0.0] },
        modifiers,
        keys: input
            .keys
            .iter()
            .filter(|e| !e.released)
            .filter_map(|e| {
                key(&e.logical).map(|k| {
                    (
                        k,
                        Modifiers {
                            shift: e.modifiers.shift,
                            alt: e.modifiers.alt,
                            primary: e.modifiers.primary(platform),
                        },
                    )
                })
            })
            .collect(),
        text: if modifiers.primary {
            String::new()
        } else {
            input.chars.clone()
        },
        viewport: [rect.w, rect.h],
        time_ms: world
            .resource::<Pointers>()
            .and_then(|ps| ps.primary().map(|c| c.time.as_secs_f64() * 1000.0))
            .unwrap_or(0.0),
    }
}

/// Claims the wheel while the pointer is over the canvas (or a press that
/// began there is held), before layout's scroll pass consumes it for the
/// surrounding scrollports. `rndr_ui` runs [`rndr_core::apply_scroll`] at the
/// front of [`rndr_core::Schedule::Input`], so add this with
/// `add_system_front` after the plugins. The next [`update`] pans or zooms
/// with it.
pub fn take_wheel(world: &mut World) {
    let Some((slot, captured)) = world.resource::<CanvasView>().map(|v| (v.slot, v.captured))
    else {
        return;
    };
    let Some(rect) = slot_rect(world, slot) else {
        return;
    };
    let Some(input) = world.resource_mut::<Input>() else {
        return;
    };
    let [x, y] = input.pointer.position;
    let inside = x >= rect.x && y >= rect.y && x < rect.x + rect.w && y < rect.y + rect.h;
    if input.scroll == [0.0, 0.0] || !(inside || captured) {
        return;
    }
    let scroll = std::mem::take(&mut input.scroll);
    if let Some(view) = world.resource_mut::<CanvasView>() {
        view.wheel[0] += scroll[0];
        view.wheel[1] += scroll[1];
    }
}

/// Feeds this frame's input to the installed [`CanvasView`]. Keyboard input is
/// claimed only while its slot owns focus or receives a press. Run before
/// host-global shortcuts; Input-front registration gives earliest ownership.
/// A host toolbar that starts editing should request focus for the slot.
pub fn update(world: &mut World) {
    let Some(slot) = world.resource::<CanvasView>().map(|v| v.slot) else {
        return;
    };
    let Some(rect) = slot_rect(world, slot) else {
        return;
    };
    let captured = world.resource::<CanvasView>().is_some_and(|v| v.captured);
    let mut input = editor_input(world, rect, captured);
    let Some(entity) = world
        .entities_with::<SurfaceSlot>()
        .into_iter()
        .find(|e| world.get::<SurfaceSlot>(*e).is_some_and(|s| s.id == slot))
    else {
        return;
    };
    world.insert(entity, rndr_core::Focusable);
    let native = world.resource::<Input>().cloned().unwrap_or_default();
    let focused = world.resource::<rndr_core::Focus>().and_then(|f| f.entity) == Some(entity);
    let pressed_here = input.pointer.as_ref().is_some_and(|p| p.pressed);
    let pressed_elsewhere = native.pointer.pressed && !pressed_here;
    let owns_keys = (focused || pressed_here) && !pressed_elsewhere;
    if pressed_here {
        rndr_core::request_focus(world, entity);
    }
    if !owns_keys {
        input.keys.clear();
        input.text.clear();
        if let Some(view) = world.resource_mut::<CanvasView>() {
            view.editor.finish_text();
        }
    }
    let measure = measure(world);
    let native = if owns_keys { native } else { Input::default() };
    let platform = platform_of(world);
    let engine = rndr_core::text_engine(world);
    let mut clipboard = world
        .resource::<rndr_core::Clipboard>()
        .cloned()
        .unwrap_or_default();
    let Some(view) = world.resource_mut::<CanvasView>() else {
        return;
    };
    let wheel = std::mem::take(&mut view.wheel);
    input.scroll[0] += wheel[0];
    input.scroll[1] += wheel[1];
    view.editor.set_text_engine(engine);
    let response =
        view.editor
            .handle_native(&input, &native, platform, &mut clipboard, measure.as_ref());
    if let Some(p) = &input.pointer {
        view.captured = p.down && !p.released;
    }
    let editing = view.editor.editing_text().is_some();
    let candidate = view
        .editor
        .text_geometry(measure.as_ref())
        .and_then(|geometry| {
            rndr_canvas::Rect::around(geometry.caret.map(|p| view.editor.camera.scene_to_view(p)))
        });
    if editing {
        world.insert(entity, rndr_core::CapturesKeys);
        if let Some(candidate) = candidate {
            // This slot has no Label: the layout is solely the native runner's
            // window-coordinate IME anchor, never another text projection.
            let mut layout = rndr_core::text_engine(world).layout(
                "",
                &rndr_core::TextStyle::default(),
                &[],
                f32::INFINITY,
            );
            if let Some(line) = layout.lines.first_mut() {
                line.y = candidate.y as f32 - rndr_core::TEXT_ORIGIN_Y;
                line.h = candidate.height as f32;
                line.edges = vec![(0, candidate.x as f32 - rndr_core::TEXT_ORIGIN_X)];
            }
            world.insert(entity, layout);
            world.insert(
                entity,
                rndr_core::Caret {
                    index: 0,
                    anchor: 0,
                },
            );
        }
    } else {
        world.remove::<rndr_core::CapturesKeys>(entity);
        world.remove::<rndr_core::Caret>(entity);
        world.remove::<rndr_core::TextLayout>(entity);
    }
    world.insert_resource(clipboard);
    if let Some(native) = world.resource_mut::<Input>().filter(|_| owns_keys) {
        native.keys.clear();
        native.key = None;
        native.chars.clear();
        native.ime_commit.clear();
    }
    if input.pointer.is_some() {
        world.insert_resource(match response.cursor {
            CursorHint::Text => CursorStyle::Text,
            CursorHint::Grab => CursorStyle::Grab,
            CursorHint::Move => CursorStyle::Move,
            CursorHint::Crosshair => CursorStyle::Crosshair,
            CursorHint::Default => CursorStyle::Arrow,
        });
    }
}

struct ViewPainter;

impl CanvasPainter for ViewPainter {
    fn paint(&self, world: &World, canvas: &mut Canvas) {
        let Some(view) = world.resource::<CanvasView>() else {
            return;
        };
        let measure = measure(world);
        // Surfaces paint in window coordinates; the editor works relative
        // to the slot.
        let Some(rect) = slot_rect(world, view.slot) else {
            return;
        };
        let to_window = rndr_canvas::Matrix::translate(f64::from(rect.x), f64::from(rect.y));
        let camera = view.editor.camera;
        canvas.set_camera(affine(to_window.then(camera.matrix())));
        if view.backdrop.a > 0.0 {
            let corner = |x: f32, y: f32| {
                let p = camera.view_to_scene(Vec2::new(f64::from(x), f64::from(y)));
                [p.x as f32, p.y as f32]
            };
            canvas.fill(
                Path::polygon(&[
                    corner(0.0, 0.0),
                    corner(rect.w, 0.0),
                    corner(rect.w, rect.h),
                    corner(0.0, rect.h),
                ]),
                view.backdrop,
                FillRule::NonZero,
            );
        }
        paint_scene(
            canvas,
            world,
            &view.editor.scene(),
            &view.images,
            measure.as_ref(),
        );
        paint_overlay(canvas, &view.editor, measure.as_ref(), &view.overlay);
    }
}
