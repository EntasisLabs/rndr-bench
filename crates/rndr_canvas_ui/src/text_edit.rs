//! Canvas authority and geometry around core's grapheme editing primitives.
//! The draft is disposable; only CanvasStore owns document history.
use crate::editor::{Button, CanvasEditor, Key, Modifiers, PointerInput, TextEdit};
use crate::paint::{affine, text_source, EngineMeasureRef};
use rndr_canvas::{Content, Node, SceneIndex, Vec2};
use rndr_core::{
    edit_string, Clipboard, Color, EditAction, Input, LogicalKey, NamedKey, Platform, TextEngine,
    TextLayout, TextStyle,
};
use rndr_paint::{Canvas, FillRule, Path, Stroke, TextSource};

pub(crate) fn shape(node: &Node, engine: &dyn TextEngine) -> (TextLayout, TextStyle, f32) {
    let width = rndr_canvas::text::layout_node(node, &EngineMeasureRef(engine))
        .expect("text node")
        .width;
    let source = text_source(node, width).expect("text node");
    (
        engine.layout(&source.text, &source.style, &source.spans, source.max_width),
        source.style,
        source.origin[0],
    )
}

fn apply(edit: &mut TextEdit, key: Option<rndr_core::Key>, shift: bool, chars: &str) {
    let Content::Text { text, .. } = &mut edit.node.content else {
        return;
    };
    let before = text.clone();
    (edit.caret, edit.anchor) = edit_string(text, edit.caret, edit.anchor, key, shift, chars, true);
    if *text != before {
        // Inline spans are UTF-16 offsets. Plain editing deliberately drops
        // them rather than silently applying old offsets to unrelated text.
        if let Some(layout) = &mut edit.node.text_layout {
            layout.spans.clear();
        }
    }
}

pub(crate) fn insert(edit: &mut TextEdit, text: &str) {
    if text.is_empty() {
        return;
    }
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    for (i, line) in normalized.split('\n').enumerate() {
        if i > 0 {
            apply(edit, Some(rndr_core::Key::Enter), false, "");
        }
        if !line.is_empty() {
            apply(edit, None, false, line);
        }
    }
}

pub(crate) fn key(edit: &mut TextEdit, key: Key, mods: Modifiers, engine: &dyn TextEngine) {
    use rndr_core::Key as CoreKey;
    match key {
        Key::Left => apply(edit, Some(CoreKey::Left), mods.shift, ""),
        Key::Right => apply(edit, Some(CoreKey::Right), mods.shift, ""),
        Key::Backspace => apply(edit, Some(CoreKey::Backspace), false, ""),
        Key::Delete => {
            if edit.caret == edit.anchor {
                let old = edit.caret;
                apply(edit, Some(CoreKey::Right), true, "");
                if old == edit.caret {
                    return;
                }
            }
            apply(edit, Some(CoreKey::Backspace), false, "");
        }
        Key::Enter if !mods.primary => apply(edit, Some(CoreKey::Enter), false, ""),
        Key::Char('a') if mods.primary => {
            edit.anchor = 0;
            if let Content::Text { text, .. } = &edit.node.content {
                edit.caret = text.len();
            }
        }
        Key::Up | Key::Down | Key::Home | Key::End => {
            let (layout, _, _) = shape(&edit.node, engine);
            let (x, y) = layout.caret_xy(edit.caret);
            let row = layout
                .lines
                .iter()
                .position(|l| (l.y - y).abs() < 0.01)
                .unwrap_or(0);
            edit.caret = match key {
                Key::Home => layout.lines.get(row).map_or(0, |l| l.start),
                Key::End => layout.lines.get(row).map_or(0, |l| l.end),
                _ => {
                    let next = if key == Key::Up {
                        row.saturating_sub(1)
                    } else {
                        (row + 1).min(layout.lines.len().saturating_sub(1))
                    };
                    layout.index_on_line(next, x)
                }
            };
            if !mods.shift {
                edit.anchor = edit.caret;
            }
        }
        _ => {}
    }
}

impl CanvasEditor {
    /// Native text input adapter. Uses core's composition filtering, shortcut
    /// table and grapheme-safe editing, and the runner-owned clipboard resource.
    /// Returns false when no text draft owns input. Call instead of replaying
    /// the same keys through `handle`; `view::update` does this automatically.
    /// A text session is one canvas transaction, not a second source document.
    /// Undo during editing commits and undoes that session, then leaves editing.
    pub fn handle_native_text(
        &mut self,
        input: &Input,
        platform: Platform,
        clipboard: &mut Clipboard,
    ) -> bool {
        if self.text.is_none() {
            return false;
        }
        let frame = rndr_core::text_frame(input);
        self.text.as_mut().unwrap().preedit = frame.preedit.clone();
        // The composing input method owns navigation/shortcuts until commit or
        // cancellation. A commit may arrive alongside the next preedit.
        if !frame.preedit.is_empty() {
            insert(self.text.as_mut().unwrap(), &frame.commit);
            return true;
        }
        for event in &frame.keys {
            if event.released || event.synthetic {
                continue;
            }
            let action = rndr_core::standard_action(platform, event);
            match action {
                Some(EditAction::Undo | EditAction::Redo) => {
                    if !frame.preedit.is_empty() {
                        continue;
                    }
                    self.finish_text();
                    if action == Some(EditAction::Undo) {
                        self.undo();
                    } else {
                        self.redo();
                    }
                    return true;
                }
                Some(EditAction::Copy | EditAction::Cut) => {
                    let edit = self.text.as_mut().unwrap();
                    let Content::Text { text, .. } = &edit.node.content else {
                        unreachable!()
                    };
                    let (start, end) = (edit.anchor.min(edit.caret), edit.anchor.max(edit.caret));
                    if start != end {
                        clipboard.text = text[start..end].into();
                        clipboard.html.clear();
                        clipboard.html_text = clipboard.text.clone();
                        clipboard.dirty = true;
                        if action == Some(EditAction::Cut) && frame.preedit.is_empty() {
                            apply(edit, Some(rndr_core::Key::Backspace), false, "");
                        }
                    }
                    continue;
                }
                Some(EditAction::Paste) => {
                    if frame.preedit.is_empty() {
                        insert(self.text.as_mut().unwrap(), &clipboard.text);
                    }
                    continue;
                }
                Some(EditAction::SelectAll) => {
                    let edit = self.text.as_mut().unwrap();
                    key(
                        edit,
                        Key::Char('a'),
                        Modifiers {
                            primary: true,
                            ..Modifiers::default()
                        },
                        self.text_engine.as_ref(),
                    );
                    continue;
                }
                Some(EditAction::DocStart | EditAction::DocEnd) => {
                    let edit = self.text.as_mut().unwrap();
                    let Content::Text { text, .. } = &edit.node.content else {
                        unreachable!()
                    };
                    edit.caret = if action == Some(EditAction::DocStart) {
                        0
                    } else {
                        text.len()
                    };
                    if !event.modifiers.shift {
                        edit.anchor = edit.caret;
                    }
                    continue;
                }
                _ => {}
            }
            let key = match action {
                Some(EditAction::LineStart) => Some(Key::Home),
                Some(EditAction::LineEnd) => Some(Key::End),
                _ => match &event.logical {
                    LogicalKey::Named(NamedKey::Escape) => Some(Key::Escape),
                    LogicalKey::Named(NamedKey::Enter) => Some(Key::Enter),
                    LogicalKey::Named(NamedKey::ArrowLeft) => Some(Key::Left),
                    LogicalKey::Named(NamedKey::ArrowRight) => Some(Key::Right),
                    LogicalKey::Named(NamedKey::ArrowUp) => Some(Key::Up),
                    LogicalKey::Named(NamedKey::ArrowDown) => Some(Key::Down),
                    LogicalKey::Named(NamedKey::Home) => Some(Key::Home),
                    LogicalKey::Named(NamedKey::End) => Some(Key::End),
                    LogicalKey::Named(NamedKey::Delete) => Some(Key::Delete),
                    LogicalKey::Named(NamedKey::Backspace) => Some(Key::Backspace),
                    _ => None,
                },
            };
            if frame.preedit.is_empty() {
                let mods = Modifiers {
                    shift: event.modifiers.shift,
                    alt: event.modifiers.alt,
                    primary: event.modifiers.primary(platform),
                };
                if key == Some(Key::Escape) || (key == Some(Key::Enter) && mods.primary) {
                    self.finish_text();
                    return true;
                }
                let edit = self.text.as_mut().unwrap();
                if let Some(k) = key {
                    crate::text_edit::key(edit, k, mods, self.text_engine.as_ref());
                }
                if let Some(text) = event.insertable_text() {
                    insert(edit, &text);
                }
            }
        }
        if frame.keys.is_empty() && frame.preedit.is_empty() {
            if let Some(legacy) = frame.legacy_key {
                let key = match legacy {
                    rndr_core::Key::Left => Some(Key::Left),
                    rndr_core::Key::Right => Some(Key::Right),
                    rndr_core::Key::Up => Some(Key::Up),
                    rndr_core::Key::Down => Some(Key::Down),
                    rndr_core::Key::Enter => Some(Key::Enter),
                    rndr_core::Key::Backspace => Some(Key::Backspace),
                    rndr_core::Key::Delete => Some(Key::Delete),
                    _ => None,
                };
                if let Some(k) = key {
                    crate::text_edit::key(
                        self.text.as_mut().unwrap(),
                        k,
                        Modifiers {
                            shift: frame.shift,
                            ..Modifiers::default()
                        },
                        self.text_engine.as_ref(),
                    );
                }
            }
            if frame.legacy_key == Some(rndr_core::Key::Escape) {
                self.finish_text();
                return true;
            }
            insert(self.text.as_mut().unwrap(), &frame.chars);
        }
        if !frame.commit.is_empty() {
            insert(self.text.as_mut().unwrap(), &frame.commit);
        }
        true
    }

    pub(crate) fn text_pointer(
        &mut self,
        pointer: &PointerInput,
        shift: bool,
        measure: &dyn rndr_canvas::text::TextMeasure,
    ) -> bool {
        let Some(edit) = &self.text else { return false };
        if pointer.button != Button::Primary || !edit.preedit.is_empty() {
            return false;
        }
        let nodes = self.nodes();
        let index = SceneIndex::with_measure(&nodes, measure);
        let Some(node) = index.get(&edit.id) else {
            return false;
        };
        let local = index
            .world_matrix(node)
            .inverse()
            .apply(self.camera.view_to_scene(Vec2::new(
                pointer.position[0] as f64,
                pointer.position[1] as f64,
            )));
        let (w, h) = index.dimensions(node);
        let inside = local.x >= 0.0 && local.y >= 0.0 && local.x <= w && local.y <= h;
        if !(pointer.pressed && inside || edit.dragging) {
            return false;
        }
        let (layout, _, padding) = shape(node, self.text_engine.as_ref());
        let caret = layout.index_at(local.x as f32 - padding, local.y as f32 - padding);
        let edit = self.text.as_mut().unwrap();
        if pointer.pressed {
            edit.dragging = true;
            if !shift {
                edit.anchor = caret;
            }
        }
        if pointer.pressed || pointer.down || pointer.released {
            edit.caret = caret;
        }
        if pointer.released || !pointer.down {
            edit.dragging = false;
        }
        true
    }

    pub(crate) fn paint_text_edit(
        &self,
        canvas: &mut Canvas,
        world: &rndr_core::World,
        measure: &dyn rndr_canvas::text::TextMeasure,
        accent: Color,
    ) {
        let Some(edit) = &self.text else { return };
        let nodes = self.nodes();
        let index = SceneIndex::with_measure(&nodes, measure);
        let Some(node) = index.get(&edit.id) else {
            return;
        };
        let (layout, style, padding) = shape(node, self.text_engine.as_ref());
        canvas.push_transform(affine(index.world_matrix(node)));
        for rect in layout.selection_boxes(edit.anchor, edit.caret) {
            canvas.fill(
                Path::round_rect(
                    rndr_core::Rect {
                        x: rect.x + padding,
                        y: rect.y + padding,
                        w: rect.w,
                        h: rect.h,
                    },
                    0.0,
                ),
                Color { a: 0.24, ..accent },
                FillRule::NonZero,
            );
        }
        let at = if edit.preedit.is_empty() {
            edit.caret
        } else {
            edit.anchor.min(edit.caret)
        };
        let (x, y) = layout.caret_xy(at);
        let height = layout
            .lines
            .iter()
            .find(|l| (l.y - y).abs() < 0.01)
            .map_or(style.size * 1.3, |l| l.h);
        canvas.stroke(
            Path::line(
                [x + padding, y + padding],
                [x + padding, y + padding + height],
            ),
            accent,
            Stroke::solid((1.0 / self.camera.zoom.max(1e-6)) as f32),
        );
        if !edit.preedit.is_empty() {
            canvas.text(
                world,
                TextSource {
                    text: edit.preedit.clone(),
                    style,
                    spans: vec![],
                    max_width: f32::INFINITY,
                    origin: [x + padding, y + padding],
                    color: accent,
                },
            );
        }
        canvas.pop_transform();
    }
}
