//! Markdown views whose Liquid block references are mounted as native blocks.
use crate::render::{self, Act, Actions, BlockState};
use crate::{LiquidEvent, LiquidHost};
use rndr_core::*;
use rndr_liquid::{LiquidBlock, LiquidDocument, Segment};
use std::sync::Arc;

/// Liquid state of one [`MarkdownView`].
///
/// [`liquid_view`] spawns both. The view shows [`LiquidDocument::markdown`];
/// each paragraph that is exactly one block reference is replaced by that
/// block, rendered with `rndr_ui`, after the markdown sync of the same frame.
pub struct LiquidView {
    blocks: Arc<Vec<LiquidBlock>>,
    mounted: Vec<Mounted>,
    ids: Vec<Option<String>>,
    next_id: u64,
}

impl Component for LiquidView {}

struct Mounted {
    slot: Option<MarkdownSlot>,
    content: Entity,
    block: usize,
}

/// Outcome of an incremental source submission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateStatus {
    /// Accepted; retained subtrees are attached after the next markdown sync.
    Applied,
    /// An unfinished fence was detected. The entire last valid UI is kept.
    Deferred,
}

/// An update was refused without changing the view or its interactive state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LiquidUpdateError {
    /// The entity does not own both a Liquid and Markdown view.
    MissingView,
    /// Explicit IDs must be unique throughout a document, including nested blocks.
    DuplicateId(String),
}

/// Root of one rendered block: which block it shows and the view it is in.
#[derive(Clone)]
pub(crate) struct BlockRoot {
    pub blocks: Arc<Vec<LiquidBlock>>,
    pub index: usize,
}

impl Component for BlockRoot {}

impl LiquidView {
    /// Parsed blocks the view's markdown references.
    pub fn blocks(&self) -> &[LiquidBlock] {
        &self.blocks
    }

    /// Indices into [`Self::blocks`] that are mounted right now, in reading order.
    pub fn mounted(&self) -> Vec<usize> {
        self.mounted
            .iter()
            .filter(|m| m.slot.is_some())
            .map(|m| m.block)
            .collect()
    }
}

/// Spawn a scrolling markdown view of Liquid `source`. Parent it like any
/// [`markdown_view`]. Duplicate IDs are displayed independently; the first
/// occurrence owns the public ID and is eligible for retention on an incremental
/// update. Later occurrences are retired, never aliased to that first control.
pub fn liquid_view(world: &mut World, source: &str) -> Entity {
    let parsed = LiquidDocument::parse_with_metadata(source);
    let doc = parsed.document;
    let view = markdown_view(world, doc.markdown);
    let mut liquid = LiquidView::new(Arc::new(doc.blocks));
    liquid.ids = parsed
        .metadata
        .into_iter()
        .map(|meta| meta.explicit_id)
        .collect();
    world.insert(view, liquid);
    view
}

/// Replace the whole source of a Liquid view. Every mounted block is rebuilt
/// on the next frame; block state such as the open tab is reset. Duplicate IDs
/// follow the same first-occurrence identity rule as [`liquid_view`].
pub fn set_liquid_source(world: &mut World, view: Entity, source: &str) {
    let parsed = LiquidDocument::parse_with_metadata(source);
    let doc = parsed.document;
    detach_all(world, view);
    if let Some(liquid) = world.get_mut::<LiquidView>(view) {
        liquid.blocks = Arc::new(doc.blocks);
        liquid.ids = parsed
            .metadata
            .into_iter()
            .map(|meta| meta.explicit_id)
            .collect();
    }
    if let Some(md) = world.get_mut::<MarkdownView>(view) {
        md.set_markdown(doc.markdown);
    }
}

/// Incrementally replace source without replacing logically unchanged controls.
///
/// `id:` in a fence is document-scoped identity. Matching IDs with the same
/// component kind retain their subtree, even after reordering. Anonymous blocks
/// match by kind and ordinal *among anonymous blocks of that kind*: Markdown
/// insertions are safe, but inserting/reordering same-kind anonymous controls
/// cannot establish logical identity. Use explicit IDs for dynamic collections.
/// Duplicate IDs refuse the entire update. An incomplete fence defers the entire
/// update, including Markdown; resubmit the complete source when more arrives.
///
/// Input controls are uncontrolled by default: `value` initializes local state
/// only. `controlled: true` reapplies host `value` on every accepted submission.
/// User edits remain visible between submissions. Kind changes and removal retire
/// the old subtree and its component-owned resources. Use [`set_liquid_source`]
/// for intentional reset. Submission and the next app frame form one UI update;
/// do not mutate the underlying MarkdownView independently.
pub fn update_liquid_source(
    world: &mut World,
    view: Entity,
    source: &str,
) -> Result<UpdateStatus, LiquidUpdateError> {
    if !world.has::<LiquidView>(view) || !world.has::<MarkdownView>(view) {
        return Err(LiquidUpdateError::MissingView);
    }
    let parsed = LiquidDocument::parse_with_metadata(source);
    let ids: Vec<_> = parsed
        .metadata
        .into_iter()
        .map(|meta| meta.explicit_id)
        .collect();
    if parsed.incomplete {
        return Ok(UpdateStatus::Deferred);
    }
    let mut seen = std::collections::HashSet::new();
    for id in ids.iter().flatten() {
        if !seen.insert(id) {
            return Err(LiquidUpdateError::DuplicateId(id.clone()));
        }
    }
    let mut liquid = world.remove::<LiquidView>(view).expect("validated");
    let new_blocks = Arc::new(parsed.document.blocks);
    let old_keys = identities(&liquid.blocks, &liquid.ids);
    let new_keys = identities(&new_blocks, &ids);
    let candidates = park_content(world, view, &mut liquid, &new_blocks);
    let mut retained = Vec::new();
    for mut mounted in candidates {
        let next = new_keys
            .iter()
            .position(|key| old_keys.get(mounted.block) == Some(key));
        if let Some(next) = next.filter(|&next| {
            liquid.blocks[mounted.block].kind() == new_blocks[next].kind()
                && world.is_alive(mounted.content)
        }) {
            let changed = liquid.blocks[mounted.block] != new_blocks[next];
            world.insert(
                mounted.content,
                BlockRoot {
                    blocks: Arc::clone(&new_blocks),
                    index: next,
                },
            );
            if !crate::controls::update(world, mounted.content, &new_blocks[next]) && changed {
                render::rebuild(world, mounted.content);
            }
            mounted.block = next;
            retained.push(mounted);
        } else {
            retire(world, mounted);
        }
    }
    liquid.blocks = new_blocks;
    liquid.ids = ids;
    liquid.mounted = retained;
    world.insert(view, liquid);
    world
        .get_mut::<MarkdownView>(view)
        .expect("validated")
        .set_markdown(parsed.document.markdown);
    Ok(UpdateStatus::Applied)
}

fn identities(blocks: &[LiquidBlock], ids: &[Option<String>]) -> Vec<(String, usize)> {
    let mut ordinals = std::collections::HashMap::new();
    let mut occurrences = std::collections::HashMap::new();
    blocks
        .iter()
        .enumerate()
        .map(|(index, block)| {
            if let Some(id) = ids.get(index).and_then(|id| id.as_ref()) {
                // Construction/reset accept arbitrary documents. Keep every
                // occurrence distinct so only the first can match a unique ID.
                let occurrence = occurrences.entry(id).or_insert(0usize);
                let key = (format!("id:{id}"), *occurrence);
                *occurrence += 1;
                return key;
            }
            let ordinal = ordinals.entry(block.kind()).or_insert(0usize);
            let key = (format!("anonymous:{}", block.kind()), *ordinal);
            *ordinal += 1;
            key
        })
        .collect()
}

/// Re-render every mounted block of every Liquid view, asking the host again
/// for images, feeds and tiles. Block state is kept.
pub fn refresh(world: &mut World) {
    let owners: Vec<_> = world
        .entities_with::<LiquidView>()
        .into_iter()
        .filter(|view| {
            let blocks = &world.get::<LiquidView>(*view).expect("listed").blocks;
            let mut parent = world.get::<Parent>(*view).map(|parent| parent.entity);
            while let Some(ancestor) = parent {
                if world
                    .get::<LiquidView>(ancestor)
                    .is_some_and(|owner| Arc::ptr_eq(blocks, &owner.blocks))
                {
                    return false;
                }
                parent = world.get::<Parent>(ancestor).map(|parent| parent.entity);
            }
            true
        })
        .collect();
    for view in owners {
        let Some(mut liquid) = world.remove::<LiquidView>(view) else {
            continue;
        };
        let blocks = Arc::clone(&liquid.blocks);
        let candidates = park_content(world, view, &mut liquid, &blocks);
        for mounted in &candidates {
            render::rebuild(world, mounted.content);
        }
        liquid.mounted = candidates;
        world.insert(view, liquid);
    }
}

// Source updates and host refresh share the same retirement boundary. Park all
// component content before rebuilding an ancestor; slot roots stay core-owned.
fn park_content(
    world: &mut World,
    view: Entity,
    liquid: &mut LiquidView,
    next: &Arc<Vec<LiquidBlock>>,
) -> Vec<Mounted> {
    let mut candidates = std::mem::take(&mut liquid.mounted);
    fn nested_views(world: &World, parent: Entity, out: &mut Vec<Entity>) {
        for child in children_of(world, parent) {
            if world.has::<LiquidView>(child) {
                out.push(child);
            }
            nested_views(world, child, out);
        }
    }
    let mut descendants = Vec::new();
    nested_views(world, view, &mut descendants);
    for nested in descendants {
        if let Some(child) = world.get_mut::<LiquidView>(nested) {
            if Arc::ptr_eq(&child.blocks, &liquid.blocks) {
                candidates.append(&mut child.mounted);
                child.blocks = Arc::clone(next);
            }
        }
    }
    for mounted in &mut candidates {
        set_parent(world, mounted.content, view);
        if let Some(slot) = mounted.slot.take() {
            let _ = slot.detach(world);
        }
    }
    candidates
}

impl LiquidView {
    /// Resolve an explicit fence ID to its live block content root.
    pub fn block(&self, id: &str) -> Option<Entity> {
        let index = self
            .ids
            .iter()
            .position(|value| value.as_deref() == Some(id))?;
        self.mounted
            .iter()
            .find(|mounted| mounted.block == index)
            .map(|mounted| mounted.content)
    }

    /// Resolve an explicit ID to its canonical interactive control root.
    pub fn control(&self, world: &World, id: &str) -> Option<Entity> {
        let index = self
            .ids
            .iter()
            .position(|value| value.as_deref() == Some(id))?;
        let root = world
            .entities_with::<BlockRoot>()
            .into_iter()
            .find(|root| {
                world.get::<BlockRoot>(*root).is_some_and(|block| {
                    block.index == index && Arc::ptr_eq(&self.blocks, &block.blocks)
                })
            })?;
        crate::controls::editor(world, root)
    }

    fn new(blocks: Arc<Vec<LiquidBlock>>) -> Self {
        Self {
            blocks,
            mounted: Vec::new(),
            ids: Vec::new(),
            next_id: 1,
        }
    }
}

fn detach_all(world: &mut World, view: Entity) {
    let Some(liquid) = world.get_mut::<LiquidView>(view) else {
        return;
    };
    let mounted = std::mem::take(&mut liquid.mounted);
    for m in mounted {
        retire(world, m);
    }
}

fn retire(world: &mut World, mounted: Mounted) {
    fn collect(world: &World, root: Entity, all: &mut Vec<Entity>) {
        all.push(root);
        for child in children_of(world, root) {
            collect(world, child, all);
        }
    }
    let mut all = Vec::new();
    collect(world, mounted.content, &mut all);
    forget_interaction_targets(world, &all);
    despawn(world, mounted.content);
    if let Some(slot) = mounted.slot {
        let _ = slot.detach(world);
    }
}

/// A markdown body inside a block: a view without its own scrollport that
/// hugs its content and mounts its own nested blocks.
pub(crate) fn nested(
    world: &mut World,
    parent: Entity,
    blocks: &Arc<Vec<LiquidBlock>>,
    markdown: &str,
) -> Entity {
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
    world.insert(view, LiquidView::new(Arc::clone(blocks)));
    set_parent(world, view, parent);
    view
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
        liquid.mounted.retain(|m| world.is_alive(m.content));
        for (index, block) in blocks.iter().enumerate() {
            let Some(target) = token_of(block) else {
                continue;
            };
            if target >= liquid.blocks.len() {
                continue;
            }
            if liquid
                .mounted
                .iter()
                .any(|m| m.block == target && m.slot.is_some_and(|slot| slot.is_mounted(world)))
            {
                continue;
            }
            let id = MarkdownSlotId(liquid.next_id);
            let Ok(slot) = mount_markdown_slot(world, view, index, block, id) else {
                continue;
            };
            liquid.next_id += 1;
            if let Some(mounted) = liquid.mounted.iter_mut().find(|m| m.block == target) {
                set_parent(world, mounted.content, slot.root);
                mounted.slot = Some(slot);
            } else if let Some(mut retained) = claim_ancestor(world, view, target, &liquid.blocks) {
                set_parent(world, retained.content, slot.root);
                retained.slot = Some(slot);
                liquid.mounted.push(retained);
            } else {
                let content = world.spawn((Layout {
                    width: Size::Fill,
                    height: Size::Hug,
                    ..Layout::column()
                },));
                set_parent(world, content, slot.root);
                world.insert(
                    content,
                    BlockRoot {
                        blocks: Arc::clone(&liquid.blocks),
                        index: target,
                    },
                );
                render::rebuild(world, content);
                liquid.mounted.push(Mounted {
                    slot: Some(slot),
                    content,
                    block: target,
                });
            }
        }
        liquid.mounted.sort_by_key(|m| {
            world
                .get::<Rect>(m.content)
                .map(|r| r.y as i64)
                .unwrap_or(0)
        });
        world.insert(view, liquid);
    }
}

// Claim only a parked subtree from the same document; nested views never get
// permission to take mounted content from another slot or another document.
fn claim_ancestor(
    world: &mut World,
    view: Entity,
    target: usize,
    blocks: &Arc<Vec<LiquidBlock>>,
) -> Option<Mounted> {
    let mut parent = world.get::<Parent>(view)?.entity;
    loop {
        if let Some(owner) = world.get_mut::<LiquidView>(parent) {
            if Arc::ptr_eq(&owner.blocks, blocks) {
                if let Some(at) = owner
                    .mounted
                    .iter()
                    .position(|m| m.block == target && m.slot.is_none())
                {
                    return Some(owner.mounted.remove(at));
                }
            }
        }
        parent = world.get::<Parent>(parent)?.entity;
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
