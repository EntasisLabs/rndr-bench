//! Interactive controls shared by host-driven Liquid renderers.
use crate::kv::parse_kv_block;
use serde::{Deserialize, Serialize};

/// Checkbox source data. Uncontrolled values initialize local state only.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CheckboxProps {
    /// Visible label.
    pub label: String,
    /// Initial value, or authoritative value when controlled.
    pub value: bool,
    /// Whether host updates overwrite local state.
    pub controlled: bool,
}

/// Text input source data. Uncontrolled values initialize local state only.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct InputProps {
    /// Visible label.
    pub label: String,
    /// Initial value, or authoritative value when controlled.
    pub value: String,
    /// Whether host updates overwrite local state.
    pub controlled: bool,
}

/// Numeric slider source data.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SliderProps {
    /// Visible label.
    pub label: String,
    /// Initial value, or authoritative value when controlled.
    pub value: f64,
    /// Lower inclusive limit.
    pub min: f64,
    /// Upper inclusive limit.
    pub max: f64,
    /// Positive increment.
    pub step: f64,
    /// Whether host updates overwrite local state.
    pub controlled: bool,
}
impl Default for SliderProps {
    fn default() -> Self {
        Self {
            label: String::new(),
            value: 0.0,
            min: 0.0,
            max: 100.0,
            step: 1.0,
            controlled: false,
        }
    }
}

fn boolean(fields: &crate::kv::Fields, key: &str) -> Option<bool> {
    match fields.get(key).map(String::as_str) {
        None | Some("false") => Some(false),
        Some("true") => Some(true),
        _ => None,
    }
}

pub(crate) fn checkbox(body: &str) -> Option<CheckboxProps> {
    let fields = parse_kv_block(body);
    Some(CheckboxProps {
        label: fields.get("label").cloned().unwrap_or_default(),
        value: boolean(&fields, "value")?,
        controlled: boolean(&fields, "controlled")?,
    })
}
pub(crate) fn input(body: &str) -> Option<InputProps> {
    let fields = parse_kv_block(body);
    Some(InputProps {
        label: fields.get("label").cloned().unwrap_or_default(),
        value: fields.get("value").cloned().unwrap_or_default(),
        controlled: boolean(&fields, "controlled")?,
    })
}
pub(crate) fn slider(body: &str) -> Option<SliderProps> {
    let fields = parse_kv_block(body);
    let number = |key: &str, default: f64| -> Option<f64> {
        let value = match fields.get(key) {
            Some(v) => v.parse::<f64>().ok()?,
            None => default,
        };
        value.is_finite().then_some(value)
    };
    let min = number("min", 0.0)?;
    let max = number("max", 100.0)?;
    let step = number("step", 1.0)?;
    if min >= max || step <= 0.0 {
        return None;
    }
    Some(SliderProps {
        label: fields.get("label").cloned().unwrap_or_default(),
        value: number("value", min)?.clamp(min, max),
        min,
        max,
        step,
        controlled: boolean(&fields, "controlled")?,
    })
}
