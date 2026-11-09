//! Stateful control adapters; field editing remains owned by rndr_core.
use rndr_core::*;
use rndr_liquid::LiquidBlock;
use rndr_ui::selection::{ChangeHandler, Checkbox, SelectionProps, SelectionValue};
use rndr_ui::{Token, Typography, UiId, UiRecipe, UiStyle, UiTheme};

/// Current local value of a Liquid control.
#[derive(Clone, Debug, PartialEq)]
pub enum LiquidValue {
    /// Checkbox state.
    Checked(bool),
    /// Text input draft.
    Text(String),
    /// Slider value.
    Number(f64),
}

/// User request emitted by a Liquid control. Host submissions never emit this.
#[derive(Clone, Debug, PartialEq)]
pub struct LiquidControlChanged {
    /// Canonical interactive entity; resolve IDs using `LiquidView::control`.
    pub control: Entity,
    /// New local draft. A controlled host may accept or replace it on submission.
    pub value: LiquidValue,
}
impl Event for LiquidControlChanged {}

#[derive(Clone)]
struct Control {
    editor: Entity,
    label: Entity,
    value: LiquidValue,
    slider: Option<rndr_liquid::SliderProps>,
    dragging: bool,
}
impl Component for Control {}
#[derive(Clone, Copy)]
struct Owner(Entity);
impl Component for Owner {}

/// Read a live control's current local draft; accepts its block or editor root.
pub fn control_value(world: &World, entity: Entity) -> Option<LiquidValue> {
    let root = world.get::<Owner>(entity).map_or(entity, |owner| owner.0);
    let control = world.get::<Control>(root)?;
    if matches!(control.value, LiquidValue::Text(_)) {
        return world
            .get::<Label>(control.editor)
            .map(|label| LiquidValue::Text(label.text.clone()));
    }
    Some(control.value.clone())
}

pub(crate) fn editor(world: &World, root: Entity) -> Option<Entity> {
    world.get::<Control>(root).map(|control| control.editor)
}

fn publish(world: &mut World, root: Entity, value: LiquidValue) {
    let Some(control) = world.get_mut::<Control>(root) else {
        return;
    };
    if control.value == value {
        return;
    }
    control.value = value.clone();
    let entity = control.editor;
    send(
        world,
        LiquidControlChanged {
            control: entity,
            value,
        },
    );
}
fn checked(world: &mut World, entity: Entity, value: &SelectionValue) {
    let Some(root) = world.get::<Owner>(entity).map(|owner| owner.0) else {
        return;
    };
    if let SelectionValue::Checked(value) = value {
        publish(world, root, LiquidValue::Checked(*value));
        let label = world
            .get::<Semantics>(entity)
            .and_then(|s| s.name.clone())
            .unwrap_or_else(|| "Checkbox".into());
        let _ = Checkbox::mount(world, root, UiId(1), SelectionProps::new(label), *value);
    }
}
fn field(world: &mut World, entity: Entity, transaction: FieldTransaction) {
    let Some(root) = world.get::<Owner>(entity).map(|owner| owner.0) else {
        return;
    };
    if let FieldTransaction::Changed(text) = transaction {
        publish(world, root, LiquidValue::Text(text));
    }
}
fn text(world: &mut World, parent: Entity, value: &str) -> Entity {
    let entity = world.spawn((
        Label::new(value).overflow(TextOverflow::Wrap),
        LabelBox::CONTENT,
        UiRecipe(UiStyle {
            foreground: Some(Token::Text.into()),
            typography: Some(Typography::Label),
            ..UiStyle::default()
        }),
        Layout {
            width: Size::Fill,
            height: Size::Hug,
            ..Layout::column()
        },
    ));
    set_parent(world, entity, parent);
    entity
}
fn slider_value(world: &mut World, root: Entity, value: f64, user: bool) {
    let Some(control) = world.get::<Control>(root).cloned() else {
        return;
    };
    let Some(props) = control.slider else {
        return;
    };
    let value = (props.min + ((value - props.min) / props.step).round() * props.step)
        .clamp(props.min, props.max);
    if user {
        publish(world, root, LiquidValue::Number(value));
    } else if let Some(control) = world.get_mut::<Control>(root) {
        control.value = LiquidValue::Number(value);
    }
    if let Some(label) = world.get_mut::<Label>(control.editor) {
        let proportion = (value - props.min) / (props.max - props.min);
        let at = (proportion * 20.0).round() as usize;
        // Bundled UI fonts need not contain box-drawing or symbol glyphs.
        // A deliberately ASCII track stays legible with every supported engine.
        label.text = format!("[{}|{}]  {value:.2}", "=".repeat(at), "-".repeat(20 - at));
    }
}
fn slider_key(world: &mut World, entity: Entity, key: &KeyEvent) -> bool {
    if !access_details(world, entity).activate {
        return false;
    }
    let Some(root) = world.get::<Owner>(entity).map(|owner| owner.0) else {
        return false;
    };
    let Some(control) = world.get::<Control>(root) else {
        return false;
    };
    let Some(props) = control.slider.as_ref() else {
        return false;
    };
    let LiquidValue::Number(value) = control.value else {
        return false;
    };
    let next = match key.logical {
        LogicalKey::Named(NamedKey::ArrowLeft | NamedKey::ArrowDown) => value - props.step,
        LogicalKey::Named(NamedKey::ArrowRight | NamedKey::ArrowUp) => value + props.step,
        LogicalKey::Named(NamedKey::Home) => props.min,
        LogicalKey::Named(NamedKey::End) => props.max,
        _ => return false,
    };
    if !key.released && !key.synthetic {
        slider_value(world, root, next, true);
    }
    true
}

pub(crate) fn mount(world: &mut World, root: Entity, block: &LiquidBlock) -> bool {
    let (editor, label, value, slider) = match block {
        LiquidBlock::Checkbox(props) => {
            let Ok(handle) = Checkbox::mount(
                world,
                root,
                UiId(1),
                SelectionProps::new(if props.label.trim().is_empty() {
                    "Checkbox"
                } else {
                    &props.label
                }),
                props.value,
            ) else {
                return true;
            };
            world.insert(handle.root, ChangeHandler(checked));
            (
                handle.root,
                handle.root,
                LiquidValue::Checked(props.value),
                None,
            )
        }
        LiquidBlock::Input(props) => {
            let label = text(world, root, &props.label);
            let editor = text(world, root, &props.value);
            world.insert(editor, TextField);
            world.insert(editor, Focusable);
            world.insert(editor, CapturesKeys);
            world.insert(editor, FieldTransactions(field));
            world.insert(
                editor,
                Semantics {
                    name: Some(props.label.clone()),
                    ..Semantics::default()
                },
            );
            world.insert(
                editor,
                UiRecipe(UiStyle {
                    background: Some(Token::Content.into()),
                    foreground: Some(Token::Text.into()),
                    typography: Some(Typography::Label),
                    ..UiStyle::default()
                }),
            );
            if let Some(layout) = world.get_mut::<Layout>(editor) {
                layout.height = Size::Hug;
                layout.min_height = 36.0;
                layout.padding = 8.0;
            }
            (editor, label, LiquidValue::Text(props.value.clone()), None)
        }
        LiquidBlock::Slider(props) => {
            let label = text(world, root, &props.label);
            let editor = text(world, root, "");
            world.get_mut::<Layout>(editor).unwrap().min_height = 36.0;
            world.get_mut::<Layout>(editor).unwrap().padding = 8.0;
            world.insert(
                editor,
                UiRecipe(UiStyle {
                    background: Some(Token::Content.into()),
                    foreground: Some(Token::Text.into()),
                    typography: Some(Typography::Code),
                    ..UiStyle::default()
                }),
            );
            world.insert(editor, Focusable);
            world.insert(editor, Button::new("liquid-slider"));
            world.insert(
                editor,
                Semantics {
                    name: Some(props.label.clone()),
                    ..Semantics::default()
                },
            );
            world.insert(editor, KeyNavigation(slider_key));
            world.insert(editor, ActivationHandler(|_, _| {}));
            (
                editor,
                label,
                LiquidValue::Number(props.value),
                Some(props.clone()),
            )
        }
        _ => return false,
    };
    world.insert(editor, Owner(root));
    world.insert(
        root,
        Control {
            editor,
            label,
            value,
            slider,
            dragging: false,
        },
    );
    if let LiquidBlock::Slider(props) = block {
        slider_value(world, root, props.value, false);
    }
    true
}

/// Reconcile control data in place. Uncontrolled values remain local; bounds
/// still clamp a slider. Controlled text replacement clears incompatible local
/// text history only when the host actually changes its value.
pub(crate) fn update(world: &mut World, root: Entity, block: &LiquidBlock) -> bool {
    let Some(control) = world.get::<Control>(root).cloned() else {
        return false;
    };
    match block {
        LiquidBlock::Checkbox(props) => {
            let value = if props.controlled {
                props.value
            } else {
                matches!(control.value, LiquidValue::Checked(true))
            };
            if let Some(control) = world.get_mut::<Control>(root) {
                control.value = LiquidValue::Checked(value);
            }
            let _ = Checkbox::mount(
                world,
                root,
                UiId(1),
                SelectionProps::new(if props.label.trim().is_empty() {
                    "Checkbox"
                } else {
                    &props.label
                }),
                value,
            );
        }
        LiquidBlock::Input(props) => {
            if props.controlled
                && control_value(world, root) != Some(LiquidValue::Text(props.value.clone()))
            {
                reset_field_draft(world, control.editor, &props.value);
                if let Some(control) = world.get_mut::<Control>(root) {
                    control.value = LiquidValue::Text(props.value.clone());
                }
            }
            if let Some(label) = world.get_mut::<Label>(control.label) {
                label.text = props.label.clone();
            }
            world.insert(
                control.editor,
                Semantics {
                    name: Some(props.label.clone()),
                    ..Semantics::default()
                },
            );
        }
        LiquidBlock::Slider(props) => {
            let value = if props.controlled {
                props.value
            } else if let LiquidValue::Number(value) = control.value {
                value
            } else {
                props.value
            };
            if let Some(control) = world.get_mut::<Control>(root) {
                control.slider = Some(props.clone());
            }
            slider_value(world, root, value, false);
            if let Some(label) = world.get_mut::<Label>(control.label) {
                label.text = props.label.clone();
            }
            world.insert(
                control.editor,
                Semantics {
                    name: Some(props.label.clone()),
                    ..Semantics::default()
                },
            );
        }
        _ => return false,
    }
    true
}

pub(crate) fn pointer(world: &mut World) {
    let Some(input) = world.resource::<Input>().cloned() else {
        return;
    };
    for root in world.entities_with::<Control>() {
        let Some(control) = world.get::<Control>(root).cloned() else {
            continue;
        };
        // Input/slider boundaries and focus remain distinct from their fills.
        // Resolve from the current theme, not a hard-coded dark surface.
        if control.slider.is_some() || matches!(control.value, LiquidValue::Text(_)) {
            let theme = world.resource::<UiTheme>().cloned().unwrap_or_default();
            let focused =
                world.resource::<Focus>().and_then(|focus| focus.entity) == Some(control.editor);
            world.insert(
                control.editor,
                Border {
                    color: theme.color(if focused {
                        Token::Focus
                    } else {
                        Token::StrongStroke
                    }),
                    width: if focused { theme.focus_width() } else { 1.0 },
                },
            );
        }
        let Some(props) = control.slider else {
            continue;
        };
        if !access_details(world, control.editor).activate {
            if let Some(control) = world.get_mut::<Control>(root) {
                control.dragging = false;
            }
            continue;
        }
        let Some(rect) = world.get::<Rect>(control.editor).copied() else {
            continue;
        };
        let focused =
            world.resource::<Focus>().and_then(|focus| focus.entity) == Some(control.editor);
        if !focused {
            if let Some(control) = world.get_mut::<Control>(root) {
                control.dragging = false;
            }
            continue;
        }
        let [x, y] = input.pointer.position;
        let begin = input.pointer.pressed
            && input.pointer.button == PointerButton::Left
            && focused
            && x >= rect.x
            && x <= rect.x + rect.w
            && y >= rect.y
            && y <= rect.y + rect.h;
        if begin || control.dragging {
            let value = props.min
                + f64::from(((x - rect.x) / rect.w.max(1.0)).clamp(0.0, 1.0))
                    * (props.max - props.min);
            slider_value(world, root, value, true);
            if let Some(control) = world.get_mut::<Control>(root) {
                control.dragging = input.pointer.down && focused;
            }
        }
    }
}
