//! Retained Liquid form controls on the canonical interaction/field paths.
use crate::view::emit;
use crate::LiquidEvent;
use rndr_core::*;
use rndr_liquid::LiquidBlock;
use rndr_ui::selection::{ChangeHandler, Checkbox, SelectionProps, SelectionValue};
use rndr_ui::{text, Typography, UiId, UiStyle};

/// A control draft reported to the host. This is data, not mutation authority.
#[derive(Clone, Debug, PartialEq)]
pub enum ControlValue {
    /// Two-state checkbox value.
    Checked(bool),
    /// Single-line text input value.
    Text(String),
    /// Finite slider value within its declared range.
    Number(f64),
}

#[derive(Clone)]
struct Control {
    target: Entity,
    caption: Option<Entity>,
    value: ControlValue,
}
impl Component for Control {}
#[derive(Clone, Copy)]
struct Owner(Entity);
impl Component for Owner {}
#[derive(Clone, Copy)]
struct Range {
    min: f64,
    max: f64,
}
impl Component for Range {}

/// Canonical focus/input entity owned by a Liquid component root.
/// Use this for focus requests and observation, not reparenting or destruction.
pub fn control_entity(world: &World, root: Entity) -> Option<Entity> {
    world
        .get::<Control>(root)
        .map(|c| c.target)
        .filter(|e| world.is_alive(*e))
}

/// Read the current local draft, including text not yet acknowledged by a host.
pub fn control_value(world: &World, root: Entity) -> Option<ControlValue> {
    let control = world.get::<Control>(root)?;
    match &control.value {
        ControlValue::Text(_) => Some(ControlValue::Text(
            world.get::<Label>(control.target)?.text.clone(),
        )),
        value => Some(value.clone()),
    }
}

fn changed(world: &mut World, target: Entity, value: ControlValue) {
    let Some(Owner(root)) = world.get::<Owner>(target).copied() else {
        return;
    };
    if let Some(control) = world.get_mut::<Control>(root) {
        control.value = value.clone();
    }
    emit(
        world,
        LiquidEvent::ControlChanged {
            component: root,
            value,
        },
    );
}

fn checkbox_changed(world: &mut World, target: Entity, value: &SelectionValue) {
    let SelectionValue::Checked(checked) = value else {
        return;
    };
    let Some(Owner(root)) = world.get::<Owner>(target).copied() else {
        return;
    };
    let Some(block) = world.get::<crate::view::BlockRoot>(root).cloned() else {
        return;
    };
    if let LiquidBlock::Checkbox(props) = &block.blocks[block.index] {
        let _ = Checkbox::mount(
            world,
            root,
            UiId(1),
            SelectionProps::new(&props.label),
            *checked,
        );
    }
    // Complete local projection before crossing the host publication boundary.
    // A synchronous acknowledgement may replace this draft and must win.
    changed(world, target, ControlValue::Checked(*checked));
}

fn field_changed(world: &mut World, target: Entity, transaction: FieldTransaction) {
    if let FieldTransaction::Changed(value) = transaction {
        changed(world, target, ControlValue::Text(value));
    }
}

fn range_label(world: &mut World, root: Entity) {
    let Some(control) = world.get::<Control>(root).cloned() else {
        return;
    };
    let ControlValue::Number(value) = control.value else {
        return;
    };
    let Some(range) = world.get::<Range>(control.target).copied() else {
        return;
    };
    let fraction = ((value - range.min) / (range.max - range.min)).clamp(0.0, 1.0);
    let at = (fraction * 20.0).round() as usize;
    // Keep the fallback track readable with the bundled text engine too.
    let track: String = (0..=20).map(|i| if i == at { 'O' } else { '-' }).collect();
    if let Some(label) = world.get_mut::<Label>(control.target) {
        label.text = format!("[{track}]  {value:.1}");
    }
}

fn range_key(world: &mut World, target: Entity, event: &KeyEvent) -> bool {
    let Some(range) = world.get::<Range>(target).copied() else {
        return false;
    };
    let Some(Owner(root)) = world.get::<Owner>(target).copied() else {
        return false;
    };
    let Some(ControlValue::Number(value)) = control_value(world, root) else {
        return false;
    };
    let step = (range.max - range.min) / 100.0;
    let next = match event.logical {
        LogicalKey::Named(NamedKey::ArrowLeft | NamedKey::ArrowDown) => value - step,
        LogicalKey::Named(NamedKey::ArrowRight | NamedKey::ArrowUp) => value + step,
        LogicalKey::Named(NamedKey::Home) => range.min,
        LogicalKey::Named(NamedKey::End) => range.max,
        _ => return false,
    }
    .clamp(range.min, range.max);
    if next != value {
        changed(world, target, ControlValue::Number(next));
        range_label(world, root);
    }
    true
}

/// Pointer drag follows canonical hit/focus admission. Keyboard uses KeyNavigation.
pub(crate) fn pointer(world: &mut World) {
    let Some(input) = world.resource::<Input>() else {
        return;
    };
    let pointer = input.pointer;
    if pointer.button != PointerButton::Left || (!pointer.down && !pointer.pressed) {
        return;
    }
    let Some(target) = world
        .resource::<Focus>()
        .and_then(|f| f.entity)
        .filter(|e| world.has::<Range>(*e) && interaction_visible(world, *e))
    else {
        return;
    };
    let Some(rect) = world.get::<Rect>(target).copied() else {
        return;
    };
    if pointer.pressed && hit_test(world, pointer.position[0], pointer.position[1]) != Some(target)
    {
        return;
    }
    let Some(range) = world.get::<Range>(target).copied() else {
        return;
    };
    let Some(Owner(root)) = world.get::<Owner>(target).copied() else {
        return;
    };
    let fraction = f64::from(((pointer.position[0] - rect.x) / rect.w.max(1.0)).clamp(0.0, 1.0));
    let next = ControlValue::Number(range.min + fraction * (range.max - range.min));
    if control_value(world, root).as_ref() != Some(&next) {
        changed(world, target, next);
        range_label(world, root);
    }
}

/// Returns false for the existing non-form block catalog.
pub(crate) fn reconcile(world: &mut World, root: Entity, block: &LiquidBlock) -> bool {
    let (label, initial, controlled) = match block {
        LiquidBlock::Checkbox(p) => (&p.label, ControlValue::Checked(p.checked), p.controlled),
        LiquidBlock::Input(p) => (&p.label, ControlValue::Text(p.value.clone()), p.controlled),
        LiquidBlock::Slider(p) => (&p.label, ControlValue::Number(p.value), p.controlled),
        _ => return false,
    };
    let old = world.get::<Control>(root).cloned();
    let mut value = if controlled {
        initial.clone()
    } else {
        control_value(world, root).unwrap_or_else(|| initial.clone())
    };
    if let (LiquidBlock::Slider(p), ControlValue::Number(number)) = (block, &mut value) {
        *number = number.clamp(p.min, p.max);
    }
    let caption = old.as_ref().and_then(|c| c.caption).or_else(|| {
        (!matches!(block, LiquidBlock::Checkbox(_)))
            .then(|| text(world, root, label, Typography::Label).ok())
            .flatten()
    });
    if let Some(caption) = caption {
        if let Some(text) = world.get_mut::<Label>(caption) {
            text.text = label.clone();
        }
    }
    let target = match block {
        LiquidBlock::Checkbox(_) => {
            let ControlValue::Checked(checked) = value else {
                unreachable!()
            };
            let Ok(handle) =
                Checkbox::mount(world, root, UiId(1), SelectionProps::new(label), checked)
            else {
                return true;
            };
            world.insert(handle.root, ChangeHandler(checkbox_changed));
            handle.root
        }
        LiquidBlock::Input(_) => {
            let target = old.as_ref().map(|c| c.target).unwrap_or_else(|| {
                let entity = world.spawn((
                    Layout {
                        width: Size::Fill,
                        height: Size::Px(36.0),
                        padding: 4.0,
                        ..Layout::column()
                    },
                    Label::new(""),
                    TextField,
                    Focusable,
                    CapturesKeys,
                ));
                world.insert(entity, FieldTransactions(field_changed));
                world.insert(entity, UiStyle::editor());
                set_parent(world, entity, root);
                entity
            });
            if let ControlValue::Text(text) = &value {
                if world.get::<Label>(target).is_some_and(|l| l.text != *text) {
                    reset_field_draft(world, target, text);
                }
            }
            target
        }
        LiquidBlock::Slider(p) => {
            let target = old.as_ref().map(|c| c.target).unwrap_or_else(|| {
                let entity = world.spawn((
                    Layout {
                        width: Size::Fill,
                        height: Size::Px(36.0),
                        padding: 4.0,
                        ..Layout::column()
                    },
                    Label::new(""),
                    Focusable,
                    KeyNavigation(range_key),
                ));
                set_parent(world, entity, root);
                entity
            });
            world.insert(
                target,
                Range {
                    min: p.min,
                    max: p.max,
                },
            );
            target
        }
        _ => unreachable!(),
    };
    world.insert(
        target,
        Semantics {
            name: Some(label.clone()),
            ..Default::default()
        },
    );
    world.insert(target, Owner(root));
    world.insert(
        root,
        Control {
            target,
            caption,
            value,
        },
    );
    range_label(world, root);
    true
}
