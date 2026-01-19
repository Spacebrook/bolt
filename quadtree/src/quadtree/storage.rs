use super::*;
use fxhash::{FxHashMap, FxHashSet};
use std::cell::RefCell;

impl EntityReorder {
    #[inline(always)]
    fn map_entity(&mut self, old_idx: u32, in_nodes_minus_one: u32) -> u32 {
        // Safety: pointers in EntityReorder point to allocated buffers sized for entity_map_len.
        if old_idx == 0 {
            return 0;
        }
        let old_idx_usize = old_idx as usize;
        debug_assert!(old_idx_usize < self.entity_map_len);
        let mapped = unsafe { *self.entity_map.add(old_idx_usize) };
        if mapped != 0 {
            return mapped;
        }
        let dst = self.new_len;
        let mut entity = unsafe { *self.old_entities.add(old_idx_usize) };
        let extent = unsafe { *self.old_extents.add(old_idx_usize) };
        let value = unsafe { *self.old_values.add(old_idx_usize) };
        let stored_type = if self.has_entity_types {
            unsafe { *self.old_types.add(old_idx_usize) }
        } else {
            u32::MAX
        };
        let circle = if self.all_rectangles {
            CircleData::new(0.0, 0.0, 0.0)
        } else {
            unsafe { *self.old_circle_data.add(old_idx_usize) }
        };
        entity.in_nodes_minus_one = in_nodes_minus_one;
        entity.alive = 1;
        entity.next_free = 0;
        entity.shape_kind = if self.all_rectangles {
            SHAPE_RECT
        } else if self.all_circles {
            SHAPE_CIRCLE
        } else {
            entity.shape_kind
        };
        if !self.all_rectangles {
            if self.all_circles || entity.shape_kind == SHAPE_CIRCLE {
                unsafe {
                    self.new_circle_data.add(dst).write(circle);
                }
            } else {
                unsafe {
                    self.new_circle_data
                        .add(dst)
                        .write(CircleData::new(0.0, 0.0, 0.0));
                }
            }
        }
        unsafe {
            self.new_entities.add(dst).write(entity);
            self.new_extents.add(dst).write(extent);
            self.new_values.add(dst).write(value);
            if self.has_entity_types {
                self.new_types.add(dst).write(stored_type);
            }
        }
        if self.all_circles {
            self.circle_count = self.circle_count.saturating_add(1);
        } else if !self.all_rectangles && entity.shape_kind == SHAPE_CIRCLE {
            self.circle_count = self.circle_count.saturating_add(1);
        }
        self.alive_count = self.alive_count.saturating_add(1);
        unsafe {
            *self.entity_map.add(old_idx_usize) = dst as u32;
        }
        self.new_len += 1;
        dst as u32
    }
    #[inline(always)]
    fn update_in_nodes_if_mapped(&mut self, old_idx: u32, in_nodes_minus_one: u32) {
        // Safety: pointers in EntityReorder point to allocated buffers sized for entity_map_len.
        if old_idx == 0 {
            return;
        }
        let old_idx_usize = old_idx as usize;
        debug_assert!(old_idx_usize < self.entity_map_len);
        let mapped = unsafe { *self.entity_map.add(old_idx_usize) };
        if mapped == 0 {
            return;
        }
        let new_idx = mapped as usize;
        unsafe {
            (*self.new_entities.add(new_idx)).in_nodes_minus_one = in_nodes_minus_one;
        }
    }
}
impl EntityMapper for EntityReorder {
    #[inline(always)]
    fn map_entity(&mut self, old_idx: u32, in_nodes_minus_one: u32) -> u32 {
        Self::map_entity(self, old_idx, in_nodes_minus_one)
    }
    #[inline(always)]
    fn update_in_nodes_if_mapped(&mut self, old_idx: u32, in_nodes_minus_one: u32) {
        Self::update_in_nodes_if_mapped(self, old_idx, in_nodes_minus_one)
    }
}

pub(crate) struct Node {
    pub(crate) head: u32,
    pub(crate) count: u32,
    pub(crate) position_flags: u8,
    pub(crate) node_flags: u8,
    pub(crate) dedupe_start: u32,
    pub(crate) children: [u32; 4],
}

impl Node {
    #[inline(always)]
    pub(crate) fn new_leaf(position_flags: u8) -> Self {
        Self {
            head: 0,
            count: 0,
            position_flags,
            node_flags: 0,
            dedupe_start: 0,
            children: [0; 4],
        }
    }

    #[inline(always)]
    pub(crate) fn head(&self) -> u32 {
        self.head
    }

    #[inline(always)]
    pub(crate) fn set_head(&mut self, head: u32) {
        self.head = head;
    }

    #[inline(always)]
    pub(crate) fn dedupe_start(&self) -> u32 {
        self.dedupe_start
    }

    #[inline(always)]
    pub(crate) fn set_dedupe_start(&mut self, dedupe_start: u32) {
        self.dedupe_start = dedupe_start;
    }
    #[inline(always)]
    pub(crate) fn position_flags(&self) -> u8 {
        self.position_flags
    }

    #[inline(always)]
    pub(crate) fn has_dedupe(&self) -> bool {
        self.node_flags & NODE_FLAG_HAS_DEDUPE != 0
    }

    #[inline(always)]
    pub(crate) fn set_has_dedupe(&mut self, has_dedupe: bool) {
        if has_dedupe {
            self.node_flags |= NODE_FLAG_HAS_DEDUPE;
        } else {
            self.node_flags &= !NODE_FLAG_HAS_DEDUPE;
        }
    }

    #[inline(always)]
    pub(crate) fn count(&self) -> u32 {
        self.count
    }

    #[inline(always)]
    pub(crate) fn set_count(&mut self, count: u32) {
        self.count = count;
    }

    #[inline(always)]
    pub(crate) fn is_leaf(&self) -> bool {
        self.children[3] == 0
    }

    #[inline(always)]
    pub(crate) fn set_children(&mut self, children: [u32; 4]) {
        self.children = children;
    }

    #[inline(always)]
    pub(crate) fn child(&self, index: usize) -> u32 {
        self.children[index]
    }
}

#[derive(Default)]
pub(crate) struct NodeCentersSoa {
    x: Vec<f32>,
    y: Vec<f32>,
}

impl NodeCentersSoa {
    pub(crate) fn new() -> Self {
        Self {
            x: Vec::new(),
            y: Vec::new(),
        }
    }

    pub(crate) fn clear(&mut self) {
        self.x.clear();
        self.y.clear();
    }

    pub(crate) fn reserve(&mut self, additional: usize) {
        self.x.reserve(additional);
        self.y.reserve(additional);
    }

    pub(crate) fn push(&mut self, x: f32, y: f32) {
        self.x.push(x);
        self.y.push(y);
    }

    #[inline(always)]
    pub(crate) fn x_ptr(&self) -> *const f32 {
        self.x.as_ptr()
    }

    #[inline(always)]
    pub(crate) fn y_ptr(&self) -> *const f32 {
        self.y.as_ptr()
    }
}

#[derive(Clone, Copy)]
pub(crate) struct NodeReorderInfo {
    pub(crate) node_idx: u32,
    pub(crate) half: HalfExtent,
    pub(crate) parent_idx: u32,
    pub(crate) child_slot: u8,
    pub(crate) depth: u32,
}

#[derive(Clone, Copy)]
pub(crate) struct NodeQueryInfo {
    pub(crate) node_idx: u32,
}

pub(crate) const QUERY_STACK_INLINE: usize = 64;
pub(crate) const ENTITY_REORDER_INTERVAL: u32 = 16;
pub(crate) const NODE_FLAG_HAS_DEDUPE: u8 = 0b0000_0001;

pub(crate) struct EntityExtents<'a> {
    pub(crate) extents: &'a [RectExtent],
}

impl<'a> EntityExtents<'a> {
    #[inline(always)]
    pub(crate) fn extent(&self, idx: usize) -> RectExtent {
        self.extents[idx]
    }
}

#[derive(Clone, Copy)]
pub(crate) struct NodeRemoval {
    pub(crate) node_idx: u32,
    pub(crate) prev_idx: u32,
    pub(crate) node_entity_idx: u32,
    pub(crate) entity_idx: u32,
}

#[cfg(feature = "query_stats")]
#[derive(Clone, Copy, Default)]
pub struct QueryStats {
    pub query_calls: u64,
    pub node_visits: u64,
    pub entity_visits: u64,
}

#[cfg(not(feature = "query_stats"))]
#[derive(Clone, Copy, Default)]
pub struct QueryStats;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Normalization {
    Normal,
    Soft,
    Hard,
}

pub(crate) struct EntityTypeFilter {
    small: Option<Vec<u32>>,
    bitset: Option<Vec<bool>>,
    set: Option<FxHashSet<u32>>,
}

impl EntityTypeFilter {
    pub(crate) fn from_vec(values: Vec<u32>) -> Self {
        const SMALL_LIMIT: usize = 16;
        const BITSET_MAX: usize = 4096;
        const BITSET_DENSITY_NUM: usize = 1;
        const BITSET_DENSITY_DEN: usize = 4;
        if values.len() <= SMALL_LIMIT {
            Self {
                small: Some(values),
                bitset: None,
                set: None,
            }
        } else {
            let max_value = values.iter().copied().max().unwrap_or(0) as usize;
            if max_value <= BITSET_MAX
                && values.len() * BITSET_DENSITY_DEN >= (max_value + 1) * BITSET_DENSITY_NUM
            {
                let mut bitset = vec![false; max_value + 1];
                for value in values {
                    bitset[value as usize] = true;
                }
                Self {
                    small: None,
                    bitset: Some(bitset),
                    set: None,
                }
            } else {
                let set = values.into_iter().collect();
                Self {
                    small: None,
                    bitset: None,
                    set: Some(set),
                }
            }
        }
    }

    pub(crate) fn is_universal_for(&self, max_value: u32) -> bool {
        const UNIVERSAL_MAX: u32 = 4096;
        if max_value > UNIVERSAL_MAX {
            return false;
        }
        let max = max_value as usize;
        if let Some(list) = &self.small {
            if max >= 64 || list.len() != max + 1 {
                return false;
            }
            let mut mask = 0u64;
            for &value in list {
                let idx = value as usize;
                if idx > max {
                    return false;
                }
                mask |= 1u64 << idx;
            }
            let expected = if max == 63 {
                u64::MAX
            } else {
                (1u64 << (max + 1)) - 1
            };
            mask == expected
        } else if let Some(bitset) = &self.bitset {
            if bitset.len() <= max {
                return false;
            }
            bitset[..=max].iter().all(|&value| value)
        } else if let Some(set) = &self.set {
            if set.len() != max + 1 {
                return false;
            }
            (0..=max).all(|value| set.contains(&(value as u32)))
        } else {
            false
        }
    }

    pub(crate) fn contains(&self, value: u32) -> bool {
        if let Some(list) = &self.small {
            list.contains(&value)
        } else if let Some(bitset) = &self.bitset {
            bitset.get(value as usize).copied().unwrap_or(false)
        } else if let Some(set) = &self.set {
            set.contains(&value)
        } else {
            false
        }
    }
}

pub(crate) struct PairDedupe {
    table: Vec<u64>,
    stamps: Vec<u32>,
    generation: u32,
}

impl PairDedupe {
    pub(crate) fn new() -> Self {
        Self {
            table: Vec::new(),
            stamps: Vec::new(),
            generation: 1,
        }
    }

    pub(crate) fn ensure_capacity(&mut self, desired: usize) {
        let mut size = desired.next_power_of_two();
        if size < 1024 {
            size = 1024;
        }
        if self.table.len() < size {
            self.table.resize(size, 0);
            self.stamps.resize(size, 0);
        }
    }

    pub(crate) fn rehash(&mut self, desired: usize) {
        let mut size = desired.next_power_of_two();
        if size < 1024 {
            size = 1024;
        }
        if size <= self.table.len() {
            return;
        }
        let old_table = std::mem::take(&mut self.table);
        let old_stamps = std::mem::take(&mut self.stamps);
        let old_generation = self.generation;
        self.table = vec![0; size];
        self.stamps = vec![0; size];
        self.generation = 1;
        for (idx, key) in old_table.into_iter().enumerate() {
            if old_stamps.get(idx).copied().unwrap_or(0) == old_generation {
                let _ = self.insert(key);
            }
        }
    }

    pub(crate) fn clear(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        if self.generation == 0 {
            self.generation = 1;
            self.stamps.fill(0);
        }
    }

    pub(crate) fn insert(&mut self, key: u64) -> bool {
        if self.table.is_empty() {
            self.ensure_capacity(1024);
        }
        let mut mask = self.table.len() - 1;
        let mut idx = (key as usize).wrapping_mul(2654435761) & mask;
        let mut probes = 0usize;
        loop {
            if self.stamps[idx] != self.generation {
                self.table[idx] = key;
                self.stamps[idx] = self.generation;
                return true;
            }
            if self.table[idx] == key {
                return false;
            }
            idx = (idx + 1) & mask;
            probes += 1;
            if probes > mask {
                let new_size = self.table.len().saturating_mul(2).max(1024);
                self.rehash(new_size);
                mask = self.table.len() - 1;
                idx = (key as usize).wrapping_mul(2654435761) & mask;
                probes = 0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PairDedupe;

    #[test]
    fn pair_dedupe_grows_when_full() {
        let mut dedupe = PairDedupe::new();
        dedupe.ensure_capacity(1024);
        dedupe.clear();

        let mut inserted = 0usize;
        for key in 1u64..=1025u64 {
            if dedupe.insert(key) {
                inserted += 1;
            }
        }

        assert_eq!(inserted, 1025);
        assert!(dedupe.table.len() > 1024);
    }
}

pub struct QuadTree {
    pub(crate) inner: RefCell<QuadTreeInner>,
}

pub(crate) struct QuadTreeInner {
    pub(crate) root_half: HalfExtent,
    pub(crate) nodes: Vec<Node>,
    pub(crate) nodes_scratch: Vec<Node>,
    pub(crate) node_centers: NodeCentersSoa,
    pub(crate) node_centers_scratch: NodeCentersSoa,
    pub(crate) node_extents_tight: ExtentAos,
    pub(crate) node_extents_tight_scratch: ExtentAos,
    pub(crate) node_extents_loose: ExtentAos,
    pub(crate) node_extents_loose_scratch: ExtentAos,
    pub(crate) free_node: u32,
    pub(crate) node_entities: Vec<NodeEntity>,
    pub(crate) node_entities_scratch: Vec<NodeEntity>,
    pub(crate) node_entity_extents: NodeEntityExtentsSoa,
    pub(crate) node_entity_extents_scratch: NodeEntityExtentsSoa,
    pub(crate) node_entity_packed: Vec<NodeEntityPacked>,
    pub(crate) node_entity_packed_scratch: Vec<NodeEntityPacked>,
    pub(crate) node_entities_next: Vec<u32>,
    pub(crate) node_entities_next_scratch: Vec<u32>,
    pub(crate) node_entity_values: Vec<u32>,
    pub(crate) node_entity_values_scratch: Vec<u32>,
    pub(crate) node_entities_flags: Vec<u8>,
    pub(crate) node_entities_flags_scratch: Vec<u8>,
    pub(crate) free_node_entity: u32,
    pub(crate) entities: Vec<Entity>,
    pub(crate) entities_scratch: Vec<Entity>,
    pub(crate) entity_extents: ExtentAos,
    pub(crate) entity_extents_scratch: ExtentAos,
    pub(crate) query_marks: Vec<u32>,
    pub(crate) query_marks_scratch: Vec<u32>,
    pub(crate) entity_values: Vec<u32>,
    pub(crate) entity_values_scratch: Vec<u32>,
    pub(crate) entity_types: Option<Vec<u32>>,
    pub(crate) entity_types_scratch: Option<Vec<u32>>,
    pub(crate) circle_data: Option<Vec<CircleData>>,
    pub(crate) circle_data_scratch: Option<Vec<CircleData>>,
    pub(crate) free_entity: u32,
    pub(crate) insertions: Vec<u32>,
    pub(crate) removals: Vec<u32>,
    pub(crate) node_removals: Vec<NodeRemoval>,
    pub(crate) reinsertions: Vec<u32>,
    pub(crate) rebuild_stack: Vec<NodeReorderInfo>,
    pub(crate) merge_ht: Vec<u32>,
    pub(crate) normalization: Normalization,
    pub(crate) update_tick: u8,
    pub(crate) status_tick: u8,
    pub(crate) query_tick: u32,
    #[allow(dead_code)]
    pub(crate) query_stats: QueryStats,
    pub(crate) profile_remaining: u32,
    pub(crate) profile_summary: bool,
    pub(crate) profile_detail: bool,
    pub(crate) reorder_counter: u32,
    pub(crate) update_pending: bool,
    pub(crate) use_avx2: bool,
    pub(crate) large_entity_threshold: f32,
    pub(crate) large_entities: Vec<u32>,
    pub(crate) large_entity_slots: Vec<u32>,
    pub(crate) split_threshold: u32,
    pub(crate) merge_threshold: u32,
    pub(crate) max_depth: u32,
    pub(crate) min_size: f32,
    pub(crate) looseness: f32,
    pub(crate) owner_map: FxHashMap<u32, u32>,
    pub(crate) dense_owner: Vec<u32>,
    pub(crate) pair_dedupe: PairDedupe,
    pub(crate) insert_stack: NodeStack,
    pub(crate) remove_stack: NodeStack,
    pub(crate) query_stack: NodeStack,
    pub(crate) query_info_stack: Vec<std::mem::MaybeUninit<NodeQueryInfo>>,
    pub(crate) update_stack: NodeStack,
    pub(crate) circle_count: u32,
    pub(crate) typed_count: u32,
    pub(crate) alive_count: u32,
    pub(crate) max_entity_type: u32,
    pub(crate) max_entity_type_dirty: bool,
    pub(crate) entity_reorder_map: Vec<u32>,
}
