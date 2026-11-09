//! Markdown views whose Liquid block references are mounted as native blocks.
use crate::render::{self, Act, Actions, BlockState};
use crate::{LiquidEvent, LiquidHost};
use rndr_core::*;
use rndr_liquid::{LiquidBlock, LiquidDocument, Segment};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Liquid state of one [`MarkdownView`].
///
/// [`liquid_view`] spawns both. The view shows [`LiquidDocument::markdown`];
/// each paragraph that is exactly one block reference is replaced by that
/// block, rendered with `rndr_ui`, after the markdown sync of the same frame.
pub struct LiquidView {
    blocks: Arc<Vec<LiquidBlock>>,
    mounted: Vec<Mounted>,
    next_id: u64,
    keys: Vec<Identity>,
}

impl Component for LiquidView {}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Identity {
    Explicit(String),
    Inferred(&'static str, usize),
    Ambiguous(usize),
}

fn identities(
    blocks: &[LiquidBlock],
    ids: &[Option<String>],
) -> Result<Vec<Identity>, LiquidUpdateError> {
    let mut seen = HashSet::new();
    let mut ordinals = HashMap::new();
    blocks
        .iter()
        .enumerate()
        .map(|(index, block)| {
            if let Some(id) = ids.get(index).and_then(Option::as_ref) {
                if !seen.insert(id.clone()) {
                    return Err(LiquidUpdateError::DuplicateId(id.clone()));
                }
                Ok(Identity::Explicit(id.clone()))
            } else {
                let ordinal = ordinals.entry(block.kind()).or_insert(0usize);
                let key = Identity::Inferred(block.kind(), *ordinal);
                *ordinal += 1;
                Ok(key)
            }
        })
        .collect()
}

fn initial_keys(blocks: &[LiquidBlock], ids: &[Option<String>]) -> Vec<Identity> {
    // Constructors/reset have no error return. Duplicates never alias: all
    // blocks receive isolated positional identities until a valid update.
    identities(blocks, ids).unwrap_or_else(|_| {
        blocks
            .iter()
            .enumerate()
            .map(|(i, _)| Identity::Ambiguous(i))
            .collect()
    })
}

/// Result of a host source update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiquidUpdate {
    /// Complete candidate accepted; the host is woken to project it next frame.
    Applied,
    /// Unfinished fence detected. The last accepted UI is untouched.
    Incomplete,
}

/// An update refused before changing the document or its controls.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LiquidUpdateError {
    /// Explicit `key:` values must be unique, including nested blocks.
    DuplicateId(String),
    /// Entity is not a live Liquid markdown view.
    MissingView,
    /// Underlying markdown binding could not be retained.
    Markdown(MarkdownSlotError),
}

struct Mounted {
    slot: MarkdownSlot,
    block: usize,
}

/// Root of one rendered block: which block it shows and the view it is in.
#[derive(Clone)]
pub(crate) struct BlockRoot {
    pub blocks: Arc<Vec<LiquidBlock>>,
    pub index: usize,
    pub keys: Vec<Identity>,
}

impl Component for BlockRoot {}

impl LiquidView {
    /// Parsed blocks the view's markdown references.
    pub fn blocks(&self) -> &[LiquidBlock] {
        &self.blocks
    }

    /// Indices into [`Self::blocks`] that are mounted right now, in reading order.
    pub fn mounted(&self) -> Vec<usize> {
        self.mounted.iter().map(|m| m.block).collect()
    }

    /// Retained root for an explicit `key:`, available after mounting.
    /// Owns state and descendants; dies on removal, type change or reset.
    pub fn component(&self, key: &str) -> Option<Entity> {
        self.mounted
            .iter()
            .find(|m| self.keys.get(m.block) == Some(&Identity::Explicit(key.to_string())))
            .map(|m| m.slot.root)
    }
}

/// Spawn a scrolling markdown view of Liquid `source`. Parent it like any
/// [`markdown_view`].
pub fn liquid_view(world: &mut World, source: &str) -> Entity {
    let doc = LiquidDocument::parse(source);
    let view = markdown_view(world, doc.markdown);
    let keys = initial_keys(&doc.blocks, &doc.ids);
    world.insert(view, LiquidView::new(Arc::new(doc.blocks), keys));
    view
}

/// Replace the whole source of a Liquid view. Every mounted block is rebuilt
/// on the next frame; block state such as the open tab is reset. The host is
/// woken so the replacement settles even when the window was otherwise idle.
pub fn set_liquid_source(world: &mut World, view: Entity, source: &str) {
    let doc = LiquidDocument::parse(source);
    detach_all(world, view);
    if let Some(liquid) = world.get_mut::<LiquidView>(view) {
        liquid.keys = initial_keys(&doc.blocks, &doc.ids);
        liquid.blocks = Arc::new(doc.blocks);
    }
    if let Some(md) = world.get_mut::<MarkdownView>(view) {
        md.set_markdown(doc.markdown);
    }
    rndr_core::frame_wake(world).wake();
}

/// Reconcile a complete host snapshot without replacing unchanged controls.
///
/// Explicit `key:` metadata is unique across the document. Without it identity
/// is `(canonical kind, ordinal among unkeyed blocks of that kind)`: Markdown
/// insertion is safe, but inserting/reordering same-kind unkeyed blocks can
/// transfer state. Use explicit keys for editable or movable content.
/// A kind change retires the old subtree. Uncontrolled values are initial
/// defaults only; `controlled: true` republishes host values on every accepted
/// snapshot, including identical snapshots. Incomplete candidates leave the
/// entire last valid document intact; callers resend the completed snapshot.
/// [`set_liquid_source`] remains the intentional destructive reset operation.
/// Accepted updates wake the host so the projection runs on the next frame,
/// even when the window was idle. Incomplete and refused updates do not wake it.
pub fn update_liquid_source(
    world: &mut World,
    view: Entity,
    source: &str,
) -> Result<LiquidUpdate, LiquidUpdateError> {
    if !world.has::<LiquidView>(view) || !world.has::<MarkdownView>(view) {
        return Err(LiquidUpdateError::MissingView);
    }
    if !LiquidDocument::is_complete(source) {
        return Ok(LiquidUpdate::Incomplete);
    }
    let doc = LiquidDocument::parse(source);
    let keys = identities(&doc.blocks, &doc.ids)?;
    apply_document(world, view, Arc::new(doc.blocks), keys, &doc.markdown)?;
    rndr_core::frame_wake(world).wake();
    Ok(LiquidUpdate::Applied)
}

fn apply_document(
    world: &mut World,
    view: Entity,
    blocks: Arc<Vec<LiquidBlock>>,
    keys: Vec<Identity>,
    markdown: &str,
) -> Result<(), LiquidUpdateError> {
    let parsed = parse_markdown(markdown);
    let liquid = world.get::<LiquidView>(view).unwrap();
    let mut retained = Vec::new();
    let mut mounted = Vec::new();
    let mut updates = Vec::new();
    for (index, block) in parsed.iter().enumerate() {
        let Some(target) = token_of(block).filter(|i| *i < blocks.len()) else {
            continue;
        };
        if let Some(old) = liquid.mounted.iter().find(|m| {
            liquid.keys.get(m.block) == keys.get(target)
                && liquid.blocks[m.block].kind() == blocks[target].kind()
        }) {
            retained.push((old.slot, index));
            mounted.push(Mounted {
                slot: old.slot,
                block: target,
            });
            updates.push((
                old.slot.root,
                target,
                liquid.blocks[old.block] != blocks[target],
            ));
        }
    }
    set_markdown_with_slots(world, view, markdown, &retained)
        .map_err(LiquidUpdateError::Markdown)?;
    let liquid = world.get_mut::<LiquidView>(view).unwrap();
    liquid.blocks = Arc::clone(&blocks);
    liquid.keys = keys.clone();
    liquid.mounted = mounted;
    for (root, index, changed) in updates {
        world.insert(
            root,
            BlockRoot {
                blocks: Arc::clone(&blocks),
                index,
                keys: keys.clone(),
            },
        );
        if !crate::control::reconcile(world, root, &blocks[index]) {
            if changed {
                render::rebuild(world, root);
            } else {
                // The enclosing block's body references can be unchanged even
                // when the referenced nested control props changed.
                for child in nested_views(world, root) {
                    let body = world
                        .get::<MarkdownView>(child)
                        .unwrap()
                        .source()
                        .to_string();
                    apply_document(world, child, Arc::clone(&blocks), keys.clone(), &body)?;
                }
            }
        }
    }
    Ok(())
}

/// Re-render every mounted block of every Liquid view, asking the host again
/// for images, feeds and tiles. Block state is kept.
pub fn refresh(world: &mut World) {
    for root in world.entities_with::<BlockRoot>() {
        render::rebuild(world, root);
    }
}

impl LiquidView {
    fn new(blocks: Arc<Vec<LiquidBlock>>, keys: Vec<Identity>) -> Self {
        Self {
            blocks,
            mounted: Vec::new(),
            next_id: 1,
            keys,
        }
    }
}

fn detach_all(world: &mut World, view: Entity) {
    let Some(liquid) = world.get_mut::<LiquidView>(view) else {
        return;
    };
    let mounted = std::mem::take(&mut liquid.mounted);
    for m in mounted {
        let _ = m.slot.detach(world);
    }
    // A prior accepted update may have omitted a root scheduled to retire at
    // the next markdown sync. Reset owns those not-yet-retired roots as well.
    for root in children_of(world, view) {
        if world.has::<BlockRoot>(root) {
            retire(world, root);
        }
    }
}

/// A markdown body inside a block: a view without its own scrollport that
/// hugs its content and mounts its own nested blocks.
pub(crate) fn nested(
    world: &mut World,
    parent: Entity,
    blocks: &Arc<Vec<LiquidBlock>>,
    keys: &[Identity],
    markdown: &str,
    reuse: Option<Entity>,
) -> Entity {
    if let Some(view) = reuse.filter(|e| world.is_alive(*e)) {
        apply_document(world, view, Arc::clone(blocks), keys.to_vec(), markdown)
            .expect("retained nested view owns its live bindings");
        set_parent(world, view, parent);
        return view;
    }
    let view = markdown_view(world, markdown.to_string());
    world.remove::<Scroll>(view);
    world.remove::<Color>(view);
    if let Some(layout) = world.get_mut::<Layout>(view) {
        layout.height = Size::Hug;
        layout.padding = 0.0;
    }
    if let Some(md) = world.get_mut::<MarkdownView>(view) {
        md.set_follow(false);
    }
    world.insert(view, LiquidView::new(Arc::clone(blocks), keys.to_vec()));
    set_parent(world, view, parent);
    view
}

pub(crate) fn nested_views(world: &World, root: Entity) -> Vec<Entity> {
    let mut views = Vec::new();
    for child in children_of(world, root) {
        if world.has::<LiquidView>(child) {
            views.push(child);
        } else {
            views.extend(nested_views(world, child));
        }
    }
    views
}

pub(crate) fn retire(world: &mut World, root: Entity) {
    fn collect(world: &World, entity: Entity, targets: &mut Vec<Entity>) {
        targets.push(entity);
        for child in children_of(world, entity) {
            collect(world, child, targets);
        }
    }
    let mut targets = Vec::new();
    collect(world, root, &mut targets);
    forget_interaction_targets(world, &targets);
    despawn(world, root);
}

fn token_of(block: &MarkdownBlock) -> Option<usize> {
    if block.kind != MarkdownKind::Paragraph || block.quote > 0 || block.depth > 0 {
        return None;
    }
    match rndr_liquid::segments(block.text.trim()).as_slice() {
        [Segment::Block(i)] => Some(*i),
        _ => None,
    }
}

/// Mount blocks for references that markdown just synced. Runs in
/// PostUpdate, after [`MarkdownPlugin`]'s sync.
pub(crate) fn reconcile(world: &mut World) {
    for view in world.entities_with::<LiquidView>() {
        let Some(md) = world.get::<MarkdownView>(view) else {
            continue;
        };
        let blocks: Vec<MarkdownBlock> = md.blocks().to_vec();
        let mut liquid = world.remove::<LiquidView>(view).expect("listed");
        liquid.mounted.retain(|m| m.slot.is_mounted(world));
        for (index, block) in blocks.iter().enumerate() {
            let Some(target) = token_of(block) else {
                continue;
            };
            if target >= liquid.blocks.len() {
                continue;
            }
            if liquid.mounted.iter().any(|m| m.block == target) {
                continue;
            }
            let id = MarkdownSlotId(liquid.next_id);
            let Ok(slot) = mount_markdown_slot(world, view, index, block, id) else {
                continue;
            };
            liquid.next_id += 1;
            world.insert(
                slot.root,
                BlockRoot {
                    blocks: Arc::clone(&liquid.blocks),
                    index: target,
                    keys: liquid.keys.clone(),
                },
            );
            render::rebuild(world, slot.root);
            liquid.mounted.push(Mounted {
                slot,
                block: target,
            });
        }
        liquid.mounted.sort_by_key(|m| {
            blocks
                .iter()
                .position(|block| token_of(block) == Some(m.block))
                .unwrap_or(usize::MAX)
        });
        world.insert(view, liquid);
    }
}

/// Handle one request from [`take_action_intents`]. Returns `true` when it
/// came from a Liquid block: tab, accordion, carousel, slide and checklist
/// changes are applied here, and every other activation is passed to the
/// installed [`LiquidHost`] as a [`LiquidEvent`]. Pass the rest on to your
/// own controls.
pub fn dispatch(world: &mut World, request: &ActionIntent) -> bool {
    if rndr_ui::controls::host_intent(world, request).is_none() {
        return false;
    }
    let found = world
        .entities_with::<Actions>()
        .into_iter()
        .find_map(|root| {
            world
                .get::<Actions>(root)
                .and_then(|a| a.0.get(&request.entity).cloned())
                .map(|act| (root, act))
        });
    let Some((root, act)) = found else {
        return false;
    };
    match act {
        Act::Host(event) => emit(world, event),
        act => {
            if let Some(state) = world.get_mut::<BlockState>(root) {
                state.apply(&act);
            }
            render::rebuild(world, root);
        }
    }
    true
}

pub(crate) fn emit(world: &mut World, event: LiquidEvent) {
    with_host(world, |host, world| host.event(world, event));
}

pub(crate) struct HostSlot(pub Option<Box<dyn LiquidHost>>);

pub(crate) fn with_host<R: Default>(
    world: &mut World,
    f: impl FnOnce(&mut dyn LiquidHost, &mut World) -> R,
) -> R {
    let Some(mut host) = world.resource_mut::<HostSlot>().and_then(|h| h.0.take()) else {
        return R::default();
    };
    let out = f(host.as_mut(), world);
    if let Some(slot) = world.resource_mut::<HostSlot>() {
        slot.0 = Some(host);
    }
    out
}
