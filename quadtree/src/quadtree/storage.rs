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

struct Node {
    head: u32,
    count: u32,
    position_flags: u8,
    node_flags: u8,
    dedupe_start: u32,
    children: [u32; 4],
}

impl Node {
    #[inline(always)]
    fn new_leaf(position_flags: u8) -> Self {
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
    fn head(&self) -> u32 {
        self.head
    }

    #[inline(always)]
    fn set_head(&mut self, head: u32) {
        self.head = head;
    }

    #[inline(always)]
    fn dedupe_start(&self) -> u32 {
        self.dedupe_start
    }

    #[inline(always)]
    fn set_dedupe_start(&mut self, dedupe_start: u32) {
        self.dedupe_start = dedupe_start;
    }
    #[inline(always)]
    fn position_flags(&self) -> u8 {
        self.position_flags
    }

    #[inline(always)]
    fn has_dedupe(&self) -> bool {
        self.node_flags & NODE_FLAG_HAS_DEDUPE != 0
    }

    #[inline(always)]
    fn set_has_dedupe(&mut self, has_dedupe: bool) {
        if has_dedupe {
            self.node_flags |= NODE_FLAG_HAS_DEDUPE;
        } else {
            self.node_flags &= !NODE_FLAG_HAS_DEDUPE;
        }
    }

    #[inline(always)]
    fn count(&self) -> u32 {
        self.count
    }

    #[inline(always)]
    fn set_count(&mut self, count: u32) {
        self.count = count;
    }

    #[inline(always)]
    fn is_leaf(&self) -> bool {
        self.children[3] == 0
    }

    #[inline(always)]
    fn set_children(&mut self, children: [u32; 4]) {
        self.children = children;
    }

    #[inline(always)]
    fn child(&self, index: usize) -> u32 {
        self.children[index]
    }
}


#[derive(Default)]
struct NodeCentersSoa {
    x: Vec<f32>,
    y: Vec<f32>,
}

impl NodeCentersSoa {
    fn new() -> Self {
        Self { x: Vec::new(), y: Vec::new() }
    }

    fn clear(&mut self) {
        self.x.clear();
        self.y.clear();
    }

    fn reserve(&mut self, additional: usize) {
        self.x.reserve(additional);
        self.y.reserve(additional);
    }

    fn push(&mut self, x: f32, y: f32) {
        self.x.push(x);
        self.y.push(y);
    }

    #[inline(always)]
    fn x_ptr(&self) -> *const f32 {
        self.x.as_ptr()
    }

    #[inline(always)]
    fn y_ptr(&self) -> *const f32 {
        self.y.as_ptr()
    }
}

#[derive(Clone, Copy)]
struct NodeReorderInfo {
    node_idx: u32,
    half: HalfExtent,
    parent_idx: u32,
    child_slot: u8,
    depth: u32,
}

#[derive(Clone, Copy)]
struct NodeQueryInfo {
    node_idx: u32,
}

const QUERY_STACK_INLINE: usize = 64;
const ENTITY_REORDER_INTERVAL: u32 = 16;
const NODE_FLAG_HAS_DEDUPE: u8 = 0b0000_0001;

struct EntityExtents<'a> {
    extents: &'a [RectExtent],
}

impl<'a> EntityExtents<'a> {
    #[inline(always)]
    fn extent(&self, idx: usize) -> RectExtent {
        self.extents[idx]
    }
}

#[derive(Clone, Copy)]
struct NodeRemoval {
    node_idx: u32,
    prev_idx: u32,
    node_entity_idx: u32,
    entity_idx: u32,
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
enum Normalization {
    Normal,
    Soft,
    Hard,
}

struct EntityTypeFilter {
    small: Option<Vec<u32>>,
    bitset: Option<Vec<bool>>,
    set: Option<FxHashSet<u32>>,
}

impl EntityTypeFilter {
    fn from_vec(values: Vec<u32>) -> Self {
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

    fn is_universal_for(&self, max_value: u32) -> bool {
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

    fn contains(&self, value: u32) -> bool {
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

struct PairDedupe {
    table: Vec<u64>,
    stamps: Vec<u32>,
    generation: u32,
}

impl PairDedupe {
    fn new() -> Self {
        Self {
            table: Vec::new(),
            stamps: Vec::new(),
            generation: 1,
        }
    }

    fn ensure_capacity(&mut self, desired: usize) {
        let mut size = desired.next_power_of_two();
        if size < 1024 {
            size = 1024;
        }
        if self.table.len() < size {
            self.table.resize(size, 0);
            self.stamps.resize(size, 0);
        }
    }

    fn rehash(&mut self, desired: usize) {
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
            if old_stamps
                .get(idx)
                .copied()
                .unwrap_or(0)
                == old_generation
            {
                let _ = self.insert(key);
            }
        }
    }

    fn clear(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        if self.generation == 0 {
            self.generation = 1;
            self.stamps.fill(0);
        }
    }

    fn insert(&mut self, key: u64) -> bool {
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
    inner: RefCell<QuadTreeInner>,
}

struct QuadTreeInner {
    root_half: HalfExtent,
    nodes: Vec<Node>,
    nodes_scratch: Vec<Node>,
    node_centers: NodeCentersSoa,
    node_centers_scratch: NodeCentersSoa,
    node_extents_tight: ExtentAos,
    node_extents_tight_scratch: ExtentAos,
    node_extents_loose: ExtentAos,
    node_extents_loose_scratch: ExtentAos,
    free_node: u32,
    node_entities: Vec<NodeEntity>,
    node_entities_scratch: Vec<NodeEntity>,
    node_entity_extents: NodeEntityExtentsSoa,
    node_entity_extents_scratch: NodeEntityExtentsSoa,
    node_entity_packed: Vec<NodeEntityPacked>,
    node_entity_packed_scratch: Vec<NodeEntityPacked>,
    node_entities_next: Vec<u32>,
    node_entities_next_scratch: Vec<u32>,
    node_entity_values: Vec<u32>,
    node_entity_values_scratch: Vec<u32>,
    node_entities_flags: Vec<u8>,
    node_entities_flags_scratch: Vec<u8>,
    node_entities_last: Vec<u8>,
    node_entities_last_scratch: Vec<u8>,
    free_node_entity: u32,
    entities: Vec<Entity>,
    entities_scratch: Vec<Entity>,
    entity_extents: ExtentAos,
    entity_extents_scratch: ExtentAos,
    query_marks: Vec<u32>,
    query_marks_scratch: Vec<u32>,
    entity_values: Vec<u32>,
    entity_values_scratch: Vec<u32>,
    entity_types: Option<Vec<u32>>,
    entity_types_scratch: Option<Vec<u32>>,
    circle_data: Option<Vec<CircleData>>,
    circle_data_scratch: Option<Vec<CircleData>>,
    free_entity: u32,
    insertions: Vec<u32>,
    removals: Vec<u32>,
    node_removals: Vec<NodeRemoval>,
    reinsertions: Vec<u32>,
    rebuild_stack: Vec<NodeReorderInfo>,
    merge_ht: Vec<u32>,
    normalization: Normalization,
    update_tick: u8,
    status_tick: u8,
    query_tick: u32,
    #[allow(dead_code)]
    query_stats: QueryStats,
    profile_remaining: u32,
    profile_summary: bool,
    profile_detail: bool,
    reorder_counter: u32,
    allow_duplicates: bool,
    update_pending: bool,
    use_avx2: bool,
    large_entity_threshold: f32,
    large_entities: Vec<u32>,
    large_entity_slots: Vec<u32>,
    split_threshold: u32,
    merge_threshold: u32,
    max_depth: u32,
    min_size: f32,
    looseness: f32,
    owner_map: FxHashMap<u32, u32>,
    dense_owner: Vec<u32>,
    pair_dedupe: PairDedupe,
    insert_stack: NodeStack,
    remove_stack: NodeStack,
    query_stack: NodeStack,
    query_info_stack: Vec<NodeQueryInfo>,
    update_stack: NodeStack,
    circle_count: u32,
    typed_count: u32,
    alive_count: u32,
    max_entity_type: u32,
    max_entity_type_dirty: bool,
    entity_reorder_map: Vec<u32>,
}
