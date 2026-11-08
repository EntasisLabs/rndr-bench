//! Explicit, programmatic host content. Markdown never executes or requests UI.
use super::*;

/// Caller-selected identity, scoped to one markdown view. Never reuse an ID
/// within that view's lifetime, even after removal (prevents stale-handle ABA).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MarkdownSlotId(pub u64);

/// Failure without partial mutation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkdownSlotError {
    /// Missing view, invalid block index, pending mount source, or changed expected block.
    StaleBlock,
    /// Repeated retained handle/index, reused ID, or this block already has a host.
    Duplicate,
    /// Removed/dead/replaced handle or subtree.
    StaleHandle,
    /// Height must be finite and positive.
    InvalidHeight,
}

/// Retained root owned by the markdown view. Add real child entities with
/// `set_parent`, or mount a semantic UI component under `root`. The host owns
/// descendant content/updates, not the root's layout or parent. Descendants
/// still attached at removal die on detach, block replacement/removal, or view
/// despawn; explicitly reparented descendants follow their new owner. No
/// removal callbacks run.
/// Copying a handle does not extend lifetime; methods validate it each time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MarkdownSlot {
    /// View that owns this slot.
    pub view: Entity,
    /// Stable caller identity.
    pub id: MarkdownSlotId,
    /// Container, never a raster/text proxy. Do not reparent it.
    pub root: Entity,
}

pub(super) struct Binding {
    pub index: usize,
    pub root: Entity,
    id: MarkdownSlotId,
    block: MarkdownBlock,
    pub height: f32,
    /// Source byte range after the last sync. `usize::MAX` means the anchor was lost.
    start: usize,
    end: usize,
    /// When true, [`measure_hosts`] replaces [`Self::height`] from the subtree.
    pub(super) measured: bool,
}
pub(super) struct Replacement {
    source: String,
    retained: Vec<(MarkdownSlot, usize)>,
}
struct HostRoot;
impl Component for HostRoot {}

// Ignore theme-derived spans: a theme update is not document replacement.
fn same(a: &MarkdownBlock, b: &MarkdownBlock) -> bool {
    a.kind == b.kind
        && a.depth == b.depth
        && a.quote == b.quote
        && a.tight == b.tight
        && a.text == b.text
        && a.language == b.language
        && a.links == b.links
        && a.spans.len() == b.spans.len()
        && a.spans.iter().zip(&b.spans).all(|(a, b)| {
            a.start == b.start
                && a.end == b.end
                && a.face == b.face
                && a.weight == b.weight
                && a.italic == b.italic
                && a.family == b.family
                && a.underline == b.underline
                && a.highlight.is_some() == b.highlight.is_some()
        })
}

/// Replace the complete visual body of one last-synced block with a host root.
/// `expected` must match `MarkdownView::blocks()[index]`; pending source edits
/// are rejected. The slot's identity is the block's source byte range. An
/// insert above that range, or a streaming append after it, moves the range
/// with the edit and the slot follows that block. An edit that deletes the
/// range retires the slot. Theme and width changes keep the root. Identical
/// blocks at distinct ranges stay independent. No special markdown syntax is
/// interpreted.
///
/// Height is measured from the mounted subtree after layout. Until that
/// measurement exists the root hugs its children. [`MarkdownSlot::set_height`]
/// opts out and reserves a constant. Width follows the reading column. The
/// ordinary view scroll clip applies.
/// Source/selection/copy remain markdown, not embedded control values. Pointer
/// selection and markdown copy shortcuts yield to focused embedded controls.
/// Only mount trusted application content; untrusted markdown cannot mount UI.
pub fn mount_markdown_slot(
    world: &mut World,
    view: Entity,
    index: usize,
    expected: &MarkdownBlock,
    id: MarkdownSlotId,
) -> Result<MarkdownSlot, MarkdownSlotError> {
    let doc = world
        .get::<MarkdownView>(view)
        .ok_or(MarkdownSlotError::StaleBlock)?;
    if doc.dirty
        || !doc
            .blocks
            .get(index)
            .is_some_and(|block| same(block, expected))
    {
        return Err(MarkdownSlotError::StaleBlock);
    }
    if doc.used_host_ids.contains(&id) || doc.hosts.iter().any(|binding| binding.index == index) {
        return Err(MarkdownSlotError::Duplicate);
    }
    let (start, end) = {
        let doc = world
            .get::<MarkdownView>(view)
            .ok_or(MarkdownSlotError::StaleBlock)?;
        let start = *doc.starts.get(index).ok_or(MarkdownSlotError::StaleBlock)?;
        let end = *doc.ends.get(index).ok_or(MarkdownSlotError::StaleBlock)?;
        if end < start {
            return Err(MarkdownSlotError::StaleBlock);
        }
        (start, end)
    };
    let root = world.spawn((
        Layout {
            width: Size::Fill,
            height: Size::Hug,
            ..Layout::column()
        },
        HostRoot,
    ));
    set_parent(world, root, view);
    let doc = world.get_mut::<MarkdownView>(view).expect("validated view");
    doc.hosts.push(Binding {
        index,
        root,
        id,
        block: expected.clone(),
        height: 0.0,
        start,
        end,
        measured: true,
    });
    doc.used_host_ids.insert(id);
    doc.hosts_dirty = true;
    Ok(MarkdownSlot { view, id, root })
}

/// Replace markdown while explicitly retaining live host roots at new block indices.
///
/// Indices address the blocks parsed from `source`, not the last-synced blocks.
/// Every handle must belong to `view` and still own its live, attached root.
/// Repeated handles or indices return [`MarkdownSlotError::Duplicate`]; invalid
/// indices or a missing view return [`MarkdownSlotError::StaleBlock`]; invalid
/// handles return [`MarkdownSlotError::StaleHandle`]. Refusal changes nothing.
///
/// Source and selection change immediately, as with [`MarkdownView::set_markdown`].
/// On the next markdown sync, listed roots keep their descendants and focus,
/// regardless of block content or order. All unlisted hosts retire, including
/// interaction targets in their attached descendants. Until sync, handle methods
/// conservatively report pending source. Repeated successful calls before sync
/// are latest-wins and may retain any still-live binding from the previous sync,
/// even one omitted by an earlier pending call. An ordinary `set_markdown` cancels
/// explicit retention; subsequent `push_str` moves these anchors normally.
pub fn set_markdown_with_slots(
    world: &mut World,
    view: Entity,
    source: &str,
    retained: &[(MarkdownSlot, usize)],
) -> Result<(), MarkdownSlotError> {
    let doc = world
        .get::<MarkdownView>(view)
        .ok_or(MarkdownSlotError::StaleBlock)?;
    let count = parse_range(source, 0, &doc.theme).blocks.len();
    let mut handles = std::collections::HashSet::new();
    let mut indices = std::collections::HashSet::new();
    for &(slot, index) in retained {
        if slot.view != view
            || !world.is_alive(slot.root)
            || !world.has::<HostRoot>(slot.root)
            || world.get::<crate::Parent>(slot.root).map(|p| p.entity) != Some(view)
            || !doc.hosts.iter().any(|binding| {
                binding.id == slot.id && binding.root == slot.root && binding.start != usize::MAX
            })
        {
            return Err(MarkdownSlotError::StaleHandle);
        }
        if index >= count {
            return Err(MarkdownSlotError::StaleBlock);
        }
        if !handles.insert(slot.id) || !indices.insert(index) {
            return Err(MarkdownSlotError::Duplicate);
        }
    }
    let doc = world.get_mut::<MarkdownView>(view).expect("validated view");
    doc.set_markdown(source);
    doc.pending_hosts = Some(Replacement {
        source: source.to_owned(),
        retained: retained.to_vec(),
    });
    doc.hosts_dirty = true;
    Ok(())
}

impl MarkdownSlot {
    /// Check whether the retained handle is still mounted. Pending source edits
    /// conservatively make it unavailable until sync has checked the binding.
    pub fn is_mounted(self, world: &World) -> bool {
        world.get::<MarkdownView>(self.view).is_some_and(|doc| {
            !doc.dirty
                && doc
                    .hosts
                    .iter()
                    .any(|b| b.id == self.id && b.root == self.root && valid(world, doc, b))
        })
    }
    /// Reserve a new content height without replacing descendants/state.
    pub fn set_height(self, world: &mut World, height: f32) -> Result<(), MarkdownSlotError> {
        if !height.is_finite() || height <= 0.0 {
            return Err(MarkdownSlotError::InvalidHeight);
        }
        if !self.is_mounted(world) {
            return Err(MarkdownSlotError::StaleHandle);
        }
        let doc = world.get_mut::<MarkdownView>(self.view).unwrap();
        let binding = doc.hosts.iter_mut().find(|b| b.id == self.id).unwrap();
        binding.height = height;
        binding.measured = false;
        doc.hosts_dirty = true;
        Ok(())
    }
    /// Destroy this subtree now; ordinary markdown returns on the next sync.
    /// Host-owned application records/resources are not modified or destroyed.
    pub fn detach(self, world: &mut World) -> Result<(), MarkdownSlotError> {
        if !world.is_alive(self.root) {
            return Err(MarkdownSlotError::StaleHandle);
        }
        let doc = world
            .get_mut::<MarkdownView>(self.view)
            .ok_or(MarkdownSlotError::StaleHandle)?;
        let index = doc
            .hosts
            .iter()
            .position(|b| b.id == self.id && b.root == self.root)
            .ok_or(MarkdownSlotError::StaleHandle)?;
        doc.hosts.remove(index);
        doc.hosts_dirty = true;
        retire_root(world, self.root);
        Ok(())
    }
}
fn valid(world: &World, doc: &MarkdownView, binding: &Binding) -> bool {
    world.is_alive(binding.root)
        && binding.start != usize::MAX
        && doc.starts.get(binding.index) == Some(&binding.start)
        && doc.ends.get(binding.index) == Some(&binding.end)
        && doc
            .blocks
            .get(binding.index)
            .is_some_and(|block| same(block, &binding.block))
}

/// Move anchors through the edit since the last sync, before the new parse.
pub(super) fn shift_anchors(doc: &mut MarkdownView) {
    if doc.pending_hosts.is_some() {
        return;
    }
    let (at, removed, inserted) = span_edit(&doc.synced_source, &doc.source);
    for binding in &mut doc.hosts {
        match remap(binding.start, binding.end, at, removed, inserted) {
            Some((start, end)) => {
                binding.start = start;
                binding.end = end;
            }
            None => binding.start = usize::MAX,
        }
    }
}

/// Point each shifted anchor at the block that now owns that source range.
pub(super) fn rebind(doc: &mut MarkdownView) {
    if let Some(replacement) = doc.pending_hosts.take() {
        let appended = (replacement.source != doc.source)
            .then(|| parse_range(&replacement.source, 0, &doc.theme));
        let edit = span_edit(&replacement.source, &doc.source);
        for binding in &mut doc.hosts {
            let target = replacement
                .retained
                .iter()
                .find(|(slot, _)| slot.id == binding.id && slot.root == binding.root);
            let Some(&(_, index)) = target else {
                binding.start = usize::MAX;
                continue;
            };
            if let Some(parsed) = &appended {
                // Streaming after replacement follows the explicitly chosen
                // source range through the append, using the usual anchor policy.
                let range = remap(
                    parsed.starts[index],
                    parsed.ends[index],
                    edit.0,
                    edit.1,
                    edit.2,
                );
                (binding.start, binding.end) = range.unwrap_or((usize::MAX, usize::MAX));
            } else {
                binding.index = index;
                binding.start = doc.starts[index];
                binding.end = doc.ends[index];
                binding.block = doc.blocks[index].clone();
            }
        }
        if appended.is_none() {
            return;
        }
    }
    for binding in &mut doc.hosts {
        if binding.start == usize::MAX {
            continue;
        }
        let exact = doc.starts.iter().enumerate().find(|(index, start)| {
            **start == binding.start && doc.ends.get(*index) == Some(&binding.end)
        });
        let contained = doc.starts.iter().enumerate().filter(|(index, start)| {
            **start <= binding.start && doc.ends.get(*index).is_some_and(|end| *end >= binding.end)
        });
        let index = exact
            .map(|(index, _)| index)
            .or_else(|| only(contained))
            .or_else(|| {
                only(
                    doc.starts
                        .iter()
                        .enumerate()
                        .filter(|(_, start)| **start == binding.start),
                )
            });
        if let Some(index) = index {
            binding.index = index;
            binding.start = doc.starts[index];
            binding.end = doc.ends[index];
            binding.block = doc.blocks[index].clone();
        } else {
            binding.start = usize::MAX;
        }
    }
}

/// Replace measured slot heights from the previous layout of their children.
pub(super) fn measure_hosts(world: &World, doc: &mut MarkdownView) {
    for binding in &mut doc.hosts {
        if !binding.measured {
            continue;
        }
        let Some(height) = content_height(world, binding.root) else {
            continue;
        };
        if height != binding.height {
            binding.height = height;
            doc.hosts_dirty = true;
        }
    }
}

fn content_height(world: &World, root: Entity) -> Option<f32> {
    let children = crate::children_of(world, root);
    if children.is_empty() {
        return None;
    }
    let gap = world
        .get::<Layout>(root)
        .map(|layout| layout.gap)
        .unwrap_or(0.0);
    let mut height = 0.0;
    let mut count = 0usize;
    for child in children {
        let rect = world.get::<Rect>(child)?;
        if !rect.h.is_finite() || rect.h <= 0.0 {
            return None;
        }
        height += rect.h;
        count += 1;
    }
    if count > 1 {
        height += gap * (count - 1) as f32;
    }
    height.is_finite().then_some(height)
}

/// One span replacement: `(at, removed, inserted)` in bytes.
fn span_edit(old: &str, new: &str) -> (usize, usize, usize) {
    let prefix: usize = old
        .chars()
        .zip(new.chars())
        .take_while(|(a, b)| a == b)
        .map(|(ch, _)| ch.len_utf8())
        .sum();
    let old_tail = &old[prefix..];
    let new_tail = &new[prefix..];
    let suffix: usize = old_tail
        .chars()
        .rev()
        .zip(new_tail.chars().rev())
        .take_while(|(a, b)| a == b)
        .map(|(ch, _)| ch.len_utf8())
        .sum();
    let removed = old_tail.len() - suffix;
    let inserted = new_tail.len() - suffix;
    (prefix, removed, inserted)
}

fn remap(
    start: usize,
    end: usize,
    at: usize,
    removed: usize,
    inserted: usize,
) -> Option<(usize, usize)> {
    if start == usize::MAX || end < start {
        return None;
    }
    let delta = inserted as isize - removed as isize;
    if end <= at {
        return Some((start, end));
    }
    if start >= at.saturating_add(removed) {
        return Some((shift(start, delta)?, shift(end, delta)?));
    }
    if start <= at && end >= at.saturating_add(removed) {
        return Some((start, shift(end, delta)?));
    }
    None
}

fn only<'a>(mut found: impl Iterator<Item = (usize, &'a usize)>) -> Option<usize> {
    let first = found.next().map(|(index, _)| index);
    if found.next().is_some() {
        None
    } else {
        first
    }
}

fn shift(value: usize, delta: isize) -> Option<usize> {
    let next = value as isize + delta;
    (next >= 0).then_some(next as usize)
}

pub(super) fn stale(world: &World, doc: &MarkdownView) -> bool {
    doc.hosts.iter().any(|b| !valid(world, doc, b))
}
pub(super) fn retire_stale(world: &mut World, doc: &mut MarkdownView) {
    let mut keep = Vec::new();
    for binding in std::mem::take(&mut doc.hosts) {
        if valid(world, doc, &binding) {
            keep.push(binding);
        } else {
            retire_root(world, binding.root);
        }
    }
    doc.hosts = keep;
}

fn retire_root(world: &mut World, root: Entity) {
    let mut targets = Vec::new();
    let mut pending = vec![root];
    let mut visited = std::collections::HashSet::new();
    while let Some(entity) = pending.pop() {
        if visited.insert(entity) {
            targets.push(entity);
            pending.extend(children_of(world, entity));
        }
    }
    crate::interaction::forget_interaction_targets(world, &targets);
    despawn(world, root);
}
pub(super) fn contains(world: &World, mut entity: Entity) -> bool {
    // Follow arbitrary valid host depth, stopping on malformed hierarchy cycles.
    let mut visited = std::collections::HashSet::new();
    while visited.insert(entity) {
        if world.has::<HostRoot>(entity) {
            return true;
        }
        let Some(parent) = world.get::<crate::Parent>(entity) else {
            break;
        };
        entity = parent.entity;
    }
    false
}

#[cfg(test)]
mod shift_tests {
    use super::*;

    #[test]
    fn explicit_retirement_forgets_descendant_interaction_targets() {
        let mut app = App::new();
        app.add_plugin(MarkdownPlugin);
        let view = markdown_view(app.world_mut(), "Host");
        app.update();
        let block = app.world().get::<MarkdownView>(view).unwrap().blocks()[0].clone();
        let slot =
            mount_markdown_slot(app.world_mut(), view, 0, &block, MarkdownSlotId(1)).unwrap();
        let column = app.world_mut().spawn(Layout::column());
        set_parent(app.world_mut(), column, slot.root);
        let child = app
            .world_mut()
            .spawn((crate::Button::new("retired"), crate::ReleaseActivation));
        set_parent(app.world_mut(), child, column);
        let outside = app
            .world_mut()
            .spawn((crate::Button::new("unrelated"), crate::ReleaseActivation));
        app.update();
        app.world_mut()
            .insert_resource(crate::ActionIntents::default());
        assert_eq!(
            crate::try_activate(app.world_mut(), child),
            crate::ActivationResult::Accepted
        );
        assert_eq!(
            crate::try_activate(app.world_mut(), outside),
            crate::ActivationResult::Accepted
        );
        app.world_mut().insert_resource(crate::Focus {
            entity: Some(child),
        });
        app.world_mut().insert_resource(crate::Drag {
            entity: Some(child),
            ..Default::default()
        });
        app.world_mut().insert_resource(crate::ScrollDrag {
            entity: Some(column),
            active: true,
            ..Default::default()
        });
        set_markdown_with_slots(app.world_mut(), view, "Replacement", &[]).unwrap();
        // Exercise this owner directly, before any generic input cleanup can mask it.
        sync_one(app.world_mut(), view);
        assert_eq!(app.world().resource::<crate::Focus>().unwrap().entity, None);
        assert_eq!(app.world().resource::<crate::Drag>().unwrap().entity, None);
        let scroll_drag = app.world().resource::<crate::ScrollDrag>().unwrap();
        assert_eq!(scroll_drag.entity, None);
        assert!(!scroll_drag.active);
        let queued = app
            .world()
            .resource::<crate::ActionIntents>()
            .unwrap()
            .pending();
        assert_eq!(queued.len(), 1);
        assert_eq!(queued[0].entity, outside);
        assert!(!app.world().is_alive(column));
        assert!(!app.world().is_alive(child));
    }

    #[test]
    fn insert_above_is_a_prefix_insertion() {
        let edit = span_edit("Keep\n\nTail", "Intro\n\nKeep\n\nTail");
        assert_eq!(edit, (0, 0, "Intro\n\n".len()), "{edit:?}");
        assert_eq!(remap(0, 4, 0, 0, "Intro\n\n".len()), Some((7, 11)));
    }

    #[test]
    fn append_does_not_move_an_earlier_range() {
        assert_eq!(
            span_edit("Keep\n\nTail", "Keep\n\nTail\n\nMore"),
            (10, 0, 6)
        );
        assert_eq!(remap(0, 4, 10, 0, 6), Some((0, 4)));
    }
}
