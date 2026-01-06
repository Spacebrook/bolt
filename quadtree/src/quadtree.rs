use common::shapes::{Circle, Rectangle, ShapeEnum};
use fxhash::{FxHashMap, FxHashSet};
use smallvec::SmallVec;
use std::cell::RefCell;

const FLAG_LEFT: u8 = 0b0001;
const FLAG_BOTTOM: u8 = 0b0010;
const FLAG_RIGHT: u8 = 0b0100;
const FLAG_TOP: u8 = 0b1000;
const SHAPE_RECT: u8 = 0;
const SHAPE_CIRCLE: u8 = 1;

#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct RectExtent {
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
}

impl RectExtent {
    #[inline(always)]
    fn from_rect(rect: &Rectangle) -> Self {
        let half_w = rect.width * 0.5;
        let half_h = rect.height * 0.5;
        Self {
            min_x: rect.x - half_w,
            min_y: rect.y - half_h,
            max_x: rect.x + half_w,
            max_y: rect.y + half_h,
        }
    }

    #[inline(always)]
    fn from_min_max(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> Self {
        Self {
            min_x,
            min_y,
            max_x,
            max_y,
        }
    }

    #[inline(always)]
    fn intersects_strict(&self, other: &RectExtent) -> bool {
        self.max_x >= other.min_x
            && self.max_y >= other.min_y
            && other.max_x >= self.min_x
            && other.max_y >= self.min_y
    }
}

#[derive(Clone, Copy, Debug)]
struct HalfExtent {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl HalfExtent {
    #[inline(always)]
    fn from_rect_extent(extent: RectExtent) -> Self {
        let half_w = (extent.max_x - extent.min_x) * 0.5;
        let half_h = (extent.max_y - extent.min_y) * 0.5;
        Self {
            x: extent.min_x + half_w,
            y: extent.min_y + half_h,
            w: half_w,
            h: half_h,
        }
    }

    #[inline(always)]
    fn to_rect_extent(self) -> RectExtent {
        RectExtent {
            min_x: self.x - self.w,
            min_y: self.y - self.h,
            max_x: self.x + self.w,
            max_y: self.y + self.h,
        }
    }
}

type NodeStack = SmallVec<[(u32, HalfExtent); 64]>;

#[inline(always)]
fn point_to_extent_distance_sq(x: f32, y: f32, extent: RectExtent) -> f32 {
    let dx = if x < extent.min_x {
        extent.min_x - x
    } else if x > extent.max_x {
        x - extent.max_x
    } else {
        0.0
    };

    let dy = if y < extent.min_y {
        extent.min_y - y
    } else if y > extent.max_y {
        y - extent.max_y
    } else {
        0.0
    };

    dx * dx + dy * dy
}

#[derive(Clone, Copy)]
struct CircleData {
    x: f32,
    y: f32,
    radius: f32,
    radius_sq: f32,
}

impl CircleData {
    fn new(x: f32, y: f32, radius: f32) -> Self {
        Self {
            x,
            y,
            radius,
            radius_sq: radius * radius,
        }
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
struct Entity {
    next_free: u32,
    in_nodes_minus_one: u32,
    update_tick: u8,
    reinsertion_tick: u8,
    status_changed: u8,
    alive: u8,
    shape_kind: u8,
    _padding: [u8; 3],
}

impl Entity {
    fn sentinel() -> Self {
        Self {
            next_free: 0,
            in_nodes_minus_one: 0,
            update_tick: 0,
            reinsertion_tick: 0,
            status_changed: 0,
            alive: 0,
            shape_kind: SHAPE_RECT,
            _padding: [0; 3],
        }
    }

}

#[derive(Clone, Copy)]
enum QueryKind {
    Rect,
    Circle {
        x: f32,
        y: f32,
        radius: f32,
        radius_sq: f32,
    },
}

#[derive(Clone, Copy)]
struct Query {
    extent: RectExtent,
    kind: QueryKind,
}

impl Query {
    fn from_shape(shape: &ShapeEnum) -> Self {
        match shape {
            ShapeEnum::Rectangle(rect) => Self {
                extent: RectExtent::from_rect(rect),
                kind: QueryKind::Rect,
            },
            ShapeEnum::Circle(circle) => {
                let radius = circle.radius;
                Self {
                    extent: RectExtent::from_min_max(
                        circle.x - radius,
                        circle.y - radius,
                        circle.x + radius,
                        circle.y + radius,
                    ),
                    kind: QueryKind::Circle {
                        x: circle.x,
                        y: circle.y,
                        radius,
                        radius_sq: radius * radius,
                    },
                }
            }
        }
    }

    #[inline(always)]
    fn from_rect_extent(extent: RectExtent) -> Self {
        Self {
            extent,
            kind: QueryKind::Rect,
        }
    }

    #[inline(always)]
    fn from_circle_raw(x: f32, y: f32, radius: f32) -> Self {
        let extent = RectExtent::from_min_max(x - radius, y - radius, x + radius, y + radius);
        Self {
            extent,
            kind: QueryKind::Circle {
                x,
                y,
                radius,
                radius_sq: radius * radius,
            },
        }
    }
}

#[inline(always)]
fn circle_circle_raw(ax: f32, ay: f32, ar: f32, bx: f32, by: f32, br: f32) -> bool {
    let dx = ax - bx;
    let dy = ay - by;
    let radius_sum = ar + br;
    dx * dx + dy * dy < radius_sum * radius_sum
}

#[inline(always)]
fn circle_extent_raw(cx: f32, cy: f32, radius_sq: f32, extent: RectExtent) -> bool {
    let dx = if cx < extent.min_x {
        extent.min_x - cx
    } else if cx > extent.max_x {
        cx - extent.max_x
    } else {
        0.0
    };
    let dy = if cy < extent.min_y {
        extent.min_y - cy
    } else if cy > extent.max_y {
        cy - extent.max_y
    } else {
        0.0
    };

    dx * dx + dy * dy <= radius_sq
}

#[derive(Clone, Copy)]
#[repr(transparent)]
struct NodeEntity(u32);

impl NodeEntity {
    const INDEX_MASK: u32 = 0x7fff_ffff;
    const LAST_MASK: u32 = 0x8000_0000;

    #[inline(always)]
    fn new(index: u32, is_last: bool) -> Self {
        let mut value = index & Self::INDEX_MASK;
        if is_last {
            value |= Self::LAST_MASK;
        }
        NodeEntity(value)
    }

    #[inline(always)]
    fn index(self) -> u32 {
        self.0 & Self::INDEX_MASK
    }

    #[inline(always)]
    fn is_last(self) -> bool {
        (self.0 & Self::LAST_MASK) != 0
    }

    #[inline(always)]
    fn set_index(&mut self, index: u32) {
        self.0 = (self.0 & Self::LAST_MASK) | (index & Self::INDEX_MASK);
    }

    #[inline(always)]
    fn set_is_last(&mut self, is_last: bool) {
        if is_last {
            self.0 |= Self::LAST_MASK;
        } else {
            self.0 &= Self::INDEX_MASK;
        }
    }
}

struct EntityReorder {
    old_entities: *const Entity,
    new_entities: *mut Entity,
    old_extents: *const RectExtent,
    new_extents: *mut RectExtent,
    old_values: *const u32,
    new_values: *mut u32,
    old_query_ticks: *const u32,
    new_query_ticks: *mut u32,
    old_types: *const u32,
    new_types: *mut u32,
    old_circle_data: *const CircleData,
    new_circle_data: *mut CircleData,
    entity_map: *mut u32,
    entity_map_len: usize,
    new_len: usize,
    circle_count: u32,
    alive_count: u32,
    all_rectangles: bool,
    all_circles: bool,
    has_entity_types: bool,
}

trait EntityMapper {
    fn map_entity(&mut self, old_idx: u32, in_nodes_minus_one: u32) -> u32;
    fn update_in_nodes_if_mapped(&mut self, old_idx: u32, in_nodes_minus_one: u32);
}

struct IdentityMapper;

impl EntityMapper for IdentityMapper {
    #[inline(always)]
    fn map_entity(&mut self, old_idx: u32, _in_nodes_minus_one: u32) -> u32 {
        old_idx
    }

    #[inline(always)]
    fn update_in_nodes_if_mapped(&mut self, _old_idx: u32, _in_nodes_minus_one: u32) {}
}

impl EntityReorder {
    #[inline(always)]
    fn map_entity(&mut self, old_idx: u32, in_nodes_minus_one: u32) -> u32 {
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
        let query_tick = unsafe { *self.old_query_ticks.add(old_idx_usize) };
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
            self.new_query_ticks.add(dst).write(query_tick);
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
    slots: [u32; 4],
}

impl Node {
    #[inline(always)]
    fn new_leaf(position_flags: u8) -> Self {
        Self {
            slots: [0, position_flags as u32, 0, 0],
        }
    }

    #[inline(always)]
    fn head(&self) -> u32 {
        self.slots[0]
    }

    #[inline(always)]
    fn set_head(&mut self, head: u32) {
        self.slots[0] = head;
    }

    #[inline(always)]
    fn position_flags(&self) -> u8 {
        self.slots[1] as u8
    }

    #[inline(always)]
    fn count(&self) -> u32 {
        self.slots[2]
    }

    #[inline(always)]
    fn set_count(&mut self, count: u32) {
        self.slots[2] = count;
    }

    #[inline(always)]
    fn is_leaf(&self) -> bool {
        self.slots[3] == 0
    }

    #[inline(always)]
    fn set_children(&mut self, children: [u32; 4]) {
        self.slots = children;
    }

    #[inline(always)]
    fn child(&self, index: usize) -> u32 {
        self.slots[index]
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
    half: HalfExtent,
}

#[derive(Clone, Copy)]
struct NodeRemoval {
    node_idx: u32,
    prev_idx: u32,
    node_entity_idx: u32,
    entity_idx: u32,
}

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

    fn clear(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        if self.generation == 0 {
            self.generation = 1;
            self.stamps.fill(0);
        }
    }

    fn insert(&mut self, key: u64) -> bool {
        let mask = self.table.len() - 1;
        let mut idx = (key as usize).wrapping_mul(2654435761) & mask;
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
        }
    }
}

pub struct QuadTree {
    inner: RefCell<QuadTreeInner>,
}

struct QuadTreeInner {
    root_half: HalfExtent,
    nodes: Vec<Node>,
    nodes_scratch: Vec<Node>,
    free_node: u32,
    node_entities: Vec<NodeEntity>,
    node_entities_scratch: Vec<NodeEntity>,
    node_entities_next: Vec<u32>,
    node_entities_next_scratch: Vec<u32>,
    node_entities_flags: Vec<u8>,
    node_entities_flags_scratch: Vec<u8>,
    free_node_entity: u32,
    entities: Vec<Entity>,
    entities_scratch: Vec<Entity>,
    entity_extents: Vec<RectExtent>,
    entity_extents_scratch: Vec<RectExtent>,
    entity_query_ticks: Vec<u32>,
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
    update_pending: bool,
    split_threshold: u32,
    merge_threshold: u32,
    max_depth: u32,
    min_size: f32,
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
    entity_reorder_map: Vec<u32>,
}

impl QuadTreeInner {
    const DENSE_OWNER_LIMIT: usize = 1_000_000;

    pub fn new_with_config(bounding_box: Rectangle, config: Config) -> Self {
        let root_extent = RectExtent::from_rect(&bounding_box);
        let root_half = HalfExtent::from_rect_extent(root_extent);
        let split_threshold = config.node_capacity as u32;
        let merge_threshold = split_threshold.saturating_sub(1).max(1);
        let max_depth = config.max_depth as u32;
        let min_size = if config.min_size > 0.0 {
            config.min_size
        } else {
            1.0
        };
        let merge_ht_size = (merge_threshold * 2).next_power_of_two().max(8) as usize;

        let mut nodes = Vec::new();
        nodes.push(Node::new_leaf(
            FLAG_LEFT | FLAG_RIGHT | FLAG_TOP | FLAG_BOTTOM,
        ));
        let nodes_scratch = Vec::new();

        let mut node_entities = Vec::new();
        node_entities.push(NodeEntity::new(0, false));
        let node_entities_scratch = Vec::new();

        let mut node_entities_next = Vec::new();
        node_entities_next.push(0);
        let node_entities_next_scratch = Vec::new();

        let mut node_entities_flags = Vec::new();
        node_entities_flags.push(0);
        let node_entities_flags_scratch = Vec::new();
        let mut entities = Vec::new();
        entities.push(Entity::sentinel());
        let entities_scratch = Vec::new();
        let mut entity_extents = Vec::new();
        entity_extents.push(RectExtent::from_min_max(0.0, 0.0, 0.0, 0.0));
        let entity_extents_scratch = Vec::new();
        let mut entity_query_ticks = Vec::new();
        entity_query_ticks.push(0);
        let mut entity_values = Vec::new();
        entity_values.push(0);
        let entity_values_scratch = Vec::new();
        let entity_types = None;
        let entity_types_scratch = None;
        let circle_data = None;
        let circle_data_scratch = None;
        let rebuild_stack = Vec::with_capacity(
            (max_depth as usize).saturating_mul(3).saturating_add(1),
        );

        Self {
            root_half,
            nodes,
            nodes_scratch,
            free_node: 0,
            node_entities,
            node_entities_scratch,
            node_entities_next,
            node_entities_next_scratch,
            node_entities_flags,
            node_entities_flags_scratch,
            free_node_entity: 0,
            entities,
            entities_scratch,
            entity_extents,
            entity_extents_scratch,
            entity_query_ticks,
            entity_values,
            entity_values_scratch,
            entity_types,
            entity_types_scratch,
            circle_data,
            circle_data_scratch,
            free_entity: 0,
            insertions: Vec::new(),
            removals: Vec::new(),
            node_removals: Vec::new(),
            reinsertions: Vec::new(),
            rebuild_stack,
            merge_ht: vec![0; merge_ht_size],
            normalization: Normalization::Normal,
            update_tick: 0,
            status_tick: 1,
            query_tick: 0,
            update_pending: false,
            split_threshold,
            merge_threshold,
            max_depth,
            min_size,
            owner_map: FxHashMap::default(),
            dense_owner: Vec::new(),
            pair_dedupe: PairDedupe::new(),
            insert_stack: NodeStack::with_capacity(
                (max_depth as usize).saturating_mul(3).saturating_add(1),
            ),
            remove_stack: NodeStack::with_capacity(
                (max_depth as usize).saturating_mul(3).saturating_add(1),
            ),
            query_stack: NodeStack::with_capacity(
                (max_depth as usize).saturating_mul(3).saturating_add(1),
            ),
            query_info_stack: Vec::with_capacity(
                (max_depth as usize).saturating_mul(3).saturating_add(1),
            ),
            update_stack: NodeStack::with_capacity(
                (max_depth as usize)
                    .saturating_mul(3)
                    .saturating_add(1),
            ),
            circle_count: 0,
            typed_count: 0,
            alive_count: 0,
            entity_reorder_map: Vec::new(),
        }
    }

    pub fn new(bounding_box: Rectangle) -> Self {
        Self::new_with_config(bounding_box, Config::default())
    }
    fn owner_lookup(&self, value: u32) -> Option<u32> {
        let idx = value as usize;
        if idx < self.dense_owner.len() {
            let stored = self.dense_owner[idx];
            if stored != u32::MAX {
                return Some(stored);
            }
        }
        self.owner_map.get(&value).copied()
    }

    fn owner_insert(&mut self, value: u32, entity_idx: u32) {
        let idx = value as usize;
        if idx <= Self::DENSE_OWNER_LIMIT {
            if idx >= self.dense_owner.len() {
                self.dense_owner.resize(idx + 1, u32::MAX);
            }
            self.dense_owner[idx] = entity_idx;
        } else {
            self.owner_map.insert(value, entity_idx);
        }
    }

    fn owner_remove(&mut self, value: u32) -> Option<u32> {
        let idx = value as usize;
        if idx < self.dense_owner.len() {
            let stored = self.dense_owner[idx];
            if stored != u32::MAX {
                self.dense_owner[idx] = u32::MAX;
                return Some(stored);
            }
        }
        self.owner_map.remove(&value)
    }

    fn remap_owner_indices(
        entity_map: &[u32],
        dense_owner: &mut Vec<u32>,
        owner_map: &mut FxHashMap<u32, u32>,
    ) {
        for entry in dense_owner.iter_mut() {
            if *entry == u32::MAX {
                continue;
            }
            let mapped = entity_map[*entry as usize];
            if mapped == 0 {
                *entry = u32::MAX;
            } else {
                *entry = mapped;
            }
        }

        owner_map.retain(|_, idx| {
            let mapped = entity_map[*idx as usize];
            if mapped == 0 {
                false
            } else {
                *idx = mapped;
                true
            }
        });
    }

    #[inline(always)]
    fn alloc_entity_with_metadata(
        &mut self,
        value: u32,
        shape_kind: u8,
        extent: RectExtent,
        circle_data: Option<CircleData>,
        entity_type: Option<u32>,
    ) -> u32 {
        debug_assert!(shape_kind != SHAPE_CIRCLE || circle_data.is_some());
        let stored_type = entity_type.unwrap_or(u32::MAX);
        let circle_value = if shape_kind == SHAPE_CIRCLE {
            circle_data.unwrap_or_else(|| CircleData::new(0.0, 0.0, 0.0))
        } else {
            CircleData::new(0.0, 0.0, 0.0)
        };
        let idx = if self.free_entity != 0 {
            let idx = self.free_entity;
            let next = self.entities[idx as usize].next_free;
            self.free_entity = next;
            let entity = &mut self.entities[idx as usize];
            entity.status_changed = self.status_tick ^ 1;
            entity.alive = 1;
            entity.next_free = 0;
            entity.shape_kind = shape_kind;
            entity.in_nodes_minus_one = 0;
            entity.update_tick = self.update_tick;
            entity.reinsertion_tick = self.update_tick;
            self.entity_extents[idx as usize] = extent;
            self.entity_query_ticks[idx as usize] = self.query_tick;
            if shape_kind == SHAPE_CIRCLE {
                let data = self.ensure_circle_data();
                data[idx as usize] = circle_value;
            }
            self.entity_values[idx as usize] = value;
            if stored_type != u32::MAX || self.entity_types.is_some() {
                let types = self.ensure_entity_types();
                types[idx as usize] = stored_type;
                if stored_type != u32::MAX {
                    self.typed_count = self.typed_count.saturating_add(1);
                }
            }
            idx
        } else {
            let mut entity = Entity::sentinel();
            entity.next_free = 0;
            entity.in_nodes_minus_one = 0;
            entity.update_tick = self.update_tick;
            entity.reinsertion_tick = self.update_tick;
            entity.status_changed = self.status_tick ^ 1;
            entity.alive = 1;
            entity.shape_kind = shape_kind;
            self.entities.push(entity);
            self.entity_values.push(value);
            self.entity_extents.push(extent);
            self.entity_query_ticks.push(self.query_tick);
            let idx = (self.entities.len() - 1) as u32;
            if stored_type != u32::MAX || self.entity_types.is_some() {
                let types = self.ensure_entity_types();
                types[idx as usize] = stored_type;
                if stored_type != u32::MAX {
                    self.typed_count = self.typed_count.saturating_add(1);
                }
            }
            if shape_kind == SHAPE_CIRCLE {
                let data = self.ensure_circle_data();
                data[idx as usize] = circle_value;
            } else if let Some(data) = self.circle_data.as_mut() {
                if data.len() < self.entities.len() {
                    data.resize(
                        self.entities.len(),
                        CircleData::new(0.0, 0.0, 0.0),
                    );
                }
            }
            idx
        };

        self.alive_count = self.alive_count.saturating_add(1);
        if shape_kind == SHAPE_CIRCLE {
            self.circle_count = self.circle_count.saturating_add(1);
        }

        idx
    }

    fn entity_extent(&self, entity_idx: u32) -> RectExtent {
        self.entity_extents[entity_idx as usize]
    }

    fn shape_metadata(shape: &ShapeEnum) -> (u8, RectExtent, Option<CircleData>) {
        match shape {
            ShapeEnum::Rectangle(rect) => {
                let half_w = rect.width * 0.5;
                let half_h = rect.height * 0.5;
                (
                    SHAPE_RECT,
                    RectExtent::from_min_max(
                        rect.x - half_w,
                        rect.y - half_h,
                        rect.x + half_w,
                        rect.y + half_h,
                    ),
                    None,
                )
            }
            ShapeEnum::Circle(circle) => {
                let radius = circle.radius;
                (
                    SHAPE_CIRCLE,
                    RectExtent::from_min_max(
                        circle.x - radius,
                        circle.y - radius,
                        circle.x + radius,
                        circle.y + radius,
                    ),
                    Some(CircleData::new(circle.x, circle.y, radius)),
                )
            }
        }
    }

    fn ensure_entity_types(&mut self) -> &mut Vec<u32> {
        if self.entity_types.is_none() {
            let mut types = Vec::new();
            types.resize(self.entities.len().max(1), u32::MAX);
            self.entity_types = Some(types);
            if self.entity_types_scratch.is_none() {
                self.entity_types_scratch = Some(Vec::new());
            }
        } else if let Some(types) = self.entity_types.as_mut() {
            if types.len() < self.entities.len() {
                types.resize(self.entities.len(), u32::MAX);
            }
        }
        self.entity_types.as_mut().expect("entity types not initialized")
    }

    fn ensure_circle_data(&mut self) -> &mut Vec<CircleData> {
        if self.circle_data.is_none() {
            let mut data = Vec::new();
            data.resize(
                self.entities.len().max(1),
                CircleData::new(0.0, 0.0, 0.0),
            );
            self.circle_data = Some(data);
            if self.circle_data_scratch.is_none() {
                self.circle_data_scratch = Some(Vec::new());
            }
        } else if let Some(data) = self.circle_data.as_mut() {
            if data.len() < self.entities.len() {
                data.resize(
                    self.entities.len(),
                    CircleData::new(0.0, 0.0, 0.0),
                );
            }
        }
        self.circle_data.as_mut().expect("circle data not initialized")
    }

    pub fn insert(&mut self, value: u32, shape: ShapeEnum, entity_type: Option<u32>) {
        let (shape_kind, extent, circle_data) = Self::shape_metadata(&shape);
        self.insert_with_metadata(value, shape_kind, extent, circle_data, entity_type);
    }

    pub fn insert_rect_extent(
        &mut self,
        value: u32,
        min_x: f32,
        min_y: f32,
        max_x: f32,
        max_y: f32,
        entity_type: Option<u32>,
    ) {
        let extent = RectExtent::from_min_max(min_x, min_y, max_x, max_y);
        self.insert_with_metadata(value, SHAPE_RECT, extent, None, entity_type);
    }

    pub fn insert_circle_raw(
        &mut self,
        value: u32,
        x: f32,
        y: f32,
        radius: f32,
        entity_type: Option<u32>,
    ) {
        let extent = RectExtent::from_min_max(x - radius, y - radius, x + radius, y + radius);
        let circle = CircleData::new(x, y, radius);
        self.insert_with_metadata(value, SHAPE_CIRCLE, extent, Some(circle), entity_type);
    }

    fn insert_with_metadata(
        &mut self,
        value: u32,
        shape_kind: u8,
        extent: RectExtent,
        circle_data: Option<CircleData>,
        entity_type: Option<u32>,
    ) {
        if self.owner_lookup(value).is_some() {
            self.delete(value);
        }

        let entity_idx =
            self.alloc_entity_with_metadata(value, shape_kind, extent, circle_data, entity_type);
        self.owner_insert(value, entity_idx);
        self.insertions.push(entity_idx);
        self.normalization = Normalization::Hard;
    }

    pub fn delete(&mut self, value: u32) {
        let entity_idx = match self.owner_remove(value) {
            Some(idx) => idx,
            None => return,
        };
        self.removals.push(entity_idx);
        self.normalization = Normalization::Hard;
    }

    pub fn relocate_batch(&mut self, relocation_requests: Vec<RelocationRequest>) {
        for request in relocation_requests {
            self.relocate(request.value, request.shape, request.entity_type);
        }
    }

    pub fn relocate(&mut self, value: u32, shape: ShapeEnum, entity_type: Option<u32>) {
        let (shape_kind, extent, circle_data) = Self::shape_metadata(&shape);
        self.relocate_with_metadata(value, shape_kind, extent, circle_data, entity_type);
    }

    pub fn relocate_rect_extent(
        &mut self,
        value: u32,
        min_x: f32,
        min_y: f32,
        max_x: f32,
        max_y: f32,
        entity_type: Option<u32>,
    ) {
        let extent = RectExtent::from_min_max(min_x, min_y, max_x, max_y);
        self.relocate_with_metadata(value, SHAPE_RECT, extent, None, entity_type);
    }

    pub fn relocate_circle_raw(
        &mut self,
        value: u32,
        x: f32,
        y: f32,
        radius: f32,
        entity_type: Option<u32>,
    ) {
        let extent = RectExtent::from_min_max(x - radius, y - radius, x + radius, y + radius);
        let circle = CircleData::new(x, y, radius);
        self.relocate_with_metadata(value, SHAPE_CIRCLE, extent, Some(circle), entity_type);
    }

    fn relocate_with_metadata(
        &mut self,
        value: u32,
        shape_kind: u8,
        extent: RectExtent,
        circle_data: Option<CircleData>,
        entity_type: Option<u32>,
    ) {
        let entity_idx = match self.owner_lookup(value) {
            Some(idx) => idx,
            None => {
                let entity_idx = self.alloc_entity_with_metadata(
                    value,
                    shape_kind,
                    extent,
                    circle_data,
                    entity_type,
                );
                self.owner_insert(value, entity_idx);
                self.insertions.push(entity_idx);
                self.normalization = Normalization::Hard;
                return;
            }
        };

        self.update_entity_with_metadata(entity_idx, shape_kind, extent, circle_data, entity_type);
    }

    fn update_entity_with_metadata(
        &mut self,
        entity_idx: u32,
        shape_kind: u8,
        extent: RectExtent,
        circle_data: Option<CircleData>,
        entity_type: Option<u32>,
    ) {
        debug_assert!(shape_kind != SHAPE_CIRCLE || circle_data.is_some());
        let prev_kind = self.entities[entity_idx as usize].shape_kind;
        if prev_kind != shape_kind {
            if prev_kind == SHAPE_CIRCLE {
                self.circle_count = self.circle_count.saturating_sub(1);
            } else if shape_kind == SHAPE_CIRCLE {
                self.circle_count = self.circle_count.saturating_add(1);
            }
        }
        self.entities[entity_idx as usize].shape_kind = shape_kind;
        if shape_kind == SHAPE_CIRCLE {
            if let Some(data) = circle_data {
                let circle_data = self.ensure_circle_data();
                circle_data[entity_idx as usize] = data;
            }
        }
        let new_type = entity_type.unwrap_or(u32::MAX);
        let mut old_type = u32::MAX;
        if new_type != u32::MAX || self.entity_types.is_some() {
            let types = self.ensure_entity_types();
            old_type = types[entity_idx as usize];
            types[entity_idx as usize] = new_type;
        }
        if old_type == u32::MAX && new_type != u32::MAX {
            self.typed_count = self.typed_count.saturating_add(1);
        } else if old_type != u32::MAX && new_type == u32::MAX {
            self.typed_count = self.typed_count.saturating_sub(1);
        }
        if self.typed_count == 0 {
            self.entity_types = None;
            self.entity_types_scratch = None;
        }
        if self.circle_count == 0 {
            self.circle_data = None;
            self.circle_data_scratch = None;
        }
        self.entity_extents[entity_idx as usize] = extent;
        let entity = &mut self.entities[entity_idx as usize];
        entity.status_changed = self.status_tick;
        self.update_pending = true;
    }

    pub fn update(&mut self) {
        self.normalize_full();
    }

    fn normalize_hard(&mut self) {
        if matches!(
            self.normalization,
            Normalization::Normal | Normalization::Soft
        ) && !self.update_pending
        {
            return;
        }

        let has_queued_ops = !self.insertions.is_empty()
            || !self.removals.is_empty()
            || !self.node_removals.is_empty()
            || !self.reinsertions.is_empty();

        if !self.update_pending {
            if self.normalization == Normalization::Hard {
                self.normalize();
            }
            return;
        }

        let mut did_pre_normalize = false;
        if has_queued_ops {
            self.normalize();
            did_pre_normalize = true;
        }

        self.update_entities();

        if self.normalization == Normalization::Hard {
            self.normalize();
        } else if self.normalization == Normalization::Soft && !did_pre_normalize {
            self.normalize();
        }
    }

    fn normalize_full(&mut self) {
        if self.normalization == Normalization::Normal && !self.update_pending {
            return;
        }

        let has_queued_ops = !self.insertions.is_empty()
            || !self.removals.is_empty()
            || !self.node_removals.is_empty()
            || !self.reinsertions.is_empty();

        if !self.update_pending {
            self.normalize();
            return;
        }

        if has_queued_ops {
            self.normalize();
        }

        self.update_entities();

        if self.normalization == Normalization::Hard {
            self.normalize();
        }
    }

    fn normalize(&mut self) {
        if self.normalization == Normalization::Normal {
            return;
        }

        self.normalization = Normalization::Normal;
        let mut did_merge = false;

        if !self.node_removals.is_empty() {
            for removal in self.node_removals.iter().rev() {
                let node_idx = removal.node_idx;
                let node_entity_idx = removal.node_entity_idx;
                let next = self.node_entities_next[node_entity_idx as usize];
                let was_last = next == 0;
                let node = &mut self.nodes[node_idx as usize];

                if removal.prev_idx != 0 {
                    self.node_entities_next[removal.prev_idx as usize] = next;
                    if was_last {
                        self.node_entities[removal.prev_idx as usize].set_is_last(true);
                    }
                } else {
                    node.set_head(next);
                }

                let count = node.count();
                if count > 0 {
                    node.set_count(count - 1);
                }

                let entity_idx = removal.entity_idx as usize;
                let in_nodes = &mut self.entities[entity_idx].in_nodes_minus_one;
                if *in_nodes > 0 {
                    *in_nodes -= 1;
                }

                self.node_entities_next[node_entity_idx as usize] = self.free_node_entity;
                self.free_node_entity = node_entity_idx;
            }

            self.node_removals.clear();
        }

        if !self.reinsertions.is_empty() {
            let mut reinsertions = std::mem::take(&mut self.reinsertions);
            let nodes = &mut self.nodes;
            let node_entities = &mut self.node_entities;
            let node_entities_next = &mut self.node_entities_next;
            let node_entities_flags = &mut self.node_entities_flags;
            let entities = &mut self.entities;
            let mut free_node_entity = self.free_node_entity;

            let mut stack = std::mem::take(&mut self.insert_stack);
            stack.clear();

            for entity_idx in reinsertions.iter().copied() {
                let (alive, extent) = {
                    let entity = &entities[entity_idx as usize];
                    (entity.alive, self.entity_extents[entity_idx as usize])
                };
                if alive == 0 {
                    continue;
                }

                let mut in_nodes = 0u32;
                stack.clear();
                stack.push((0u32, self.root_half));

                while let Some((node_idx, half)) = stack.pop() {
                    let node_idx_usize = node_idx as usize;
                    if !nodes[node_idx_usize].is_leaf() {
                        let half_w = half.w * 0.5;
                        let half_h = half.h * 0.5;

                        if extent.min_x <= half.x {
                            if extent.min_y <= half.y {
                                let child = nodes[node_idx_usize].child(0);
                                if child != 0 {
                                    stack.push((
                                        child,
                                        HalfExtent {
                                            x: half.x - half_w,
                                            y: half.y - half_h,
                                            w: half_w,
                                            h: half_h,
                                        },
                                    ));
                                }
                            }
                            if extent.max_y >= half.y {
                                let child = nodes[node_idx_usize].child(1);
                                if child != 0 {
                                    stack.push((
                                        child,
                                        HalfExtent {
                                            x: half.x - half_w,
                                            y: half.y + half_h,
                                            w: half_w,
                                            h: half_h,
                                        },
                                    ));
                                }
                            }
                        }
                        if extent.max_x >= half.x {
                            if extent.min_y <= half.y {
                                let child = nodes[node_idx_usize].child(2);
                                if child != 0 {
                                    stack.push((
                                        child,
                                        HalfExtent {
                                            x: half.x + half_w,
                                            y: half.y - half_h,
                                            w: half_w,
                                            h: half_h,
                                        },
                                    ));
                                }
                            }
                            if extent.max_y >= half.y {
                                let child = nodes[node_idx_usize].child(3);
                                if child != 0 {
                                    stack.push((
                                        child,
                                        HalfExtent {
                                            x: half.x + half_w,
                                            y: half.y + half_h,
                                            w: half_w,
                                            h: half_h,
                                        },
                                    ));
                                }
                            }
                        }
                        continue;
                    }

                    in_nodes += 1;
                    let node_extent = half.to_rect_extent();
                    let position_flags = nodes[node_idx_usize].position_flags();
                    let mut node_entity_idx = nodes[node_idx_usize].head();
                    while node_entity_idx != 0 {
                        if node_entities[node_entity_idx as usize].index() == entity_idx {
                            break;
                        }
                        node_entity_idx = node_entities_next[node_entity_idx as usize];
                    }

                    if node_entity_idx == 0 {
                        let node_entity_idx = if free_node_entity != 0 {
                            let idx = free_node_entity;
                            free_node_entity = node_entities_next[idx as usize];
                            idx
                        } else {
                            node_entities.push(NodeEntity::new(0, false));
                            node_entities_next.push(0);
                            node_entities_flags.push(0);
                            (node_entities.len() - 1) as u32
                        };
                        let head = nodes[node_idx_usize].head();
                        node_entities_next[node_entity_idx as usize] = head;
                        node_entities[node_entity_idx as usize].set_index(entity_idx);
                        node_entities[node_entity_idx as usize].set_is_last(head == 0);
                        node_entities_flags[node_entity_idx as usize] =
                            Self::compute_node_entity_flags(node_extent, position_flags, extent);
                        let node = &mut nodes[node_idx_usize];
                        node.set_head(node_entity_idx);
                        node.set_count(node.count() + 1);
                    } else {
                        node_entities_flags[node_entity_idx as usize] =
                            Self::compute_node_entity_flags(node_extent, position_flags, extent);
                    }
                }

                if in_nodes == 0 {
                    in_nodes = 1;
                }
                entities[entity_idx as usize].in_nodes_minus_one = in_nodes - 1;
            }

            self.insert_stack = stack;
            self.free_node_entity = free_node_entity;
            reinsertions.clear();
            self.reinsertions = reinsertions;
        }

        if !self.removals.is_empty() {
            let mut removals = std::mem::take(&mut self.removals);
            for entity_idx in removals.iter().copied() {
                self.remove_entity(entity_idx);
            }
            removals.clear();
            self.removals = removals;
        }

        if !self.insertions.is_empty() {
            let mut insertions = std::mem::take(&mut self.insertions);
            for entity_idx in insertions.iter().copied() {
                if self.entities[entity_idx as usize].alive == 0 {
                    continue;
                }
                self.insert_entity_new(entity_idx);
            }
            insertions.clear();
            self.insertions = insertions;
        }

        if self.rebuild_storage(&mut did_merge) {
            self.normalization = Normalization::Soft;
        }
    }

    fn rebuild_storage(&mut self, did_merge: &mut bool) -> bool {
        if self.nodes.is_empty() {
            return false;
        }

        let profile = std::env::var("BOLT_QT_PROFILE").ok().as_deref() == Some("1");
        if self.typed_count == 0 {
            self.entity_types = None;
            self.entity_types_scratch = None;
        }
        if self.circle_count == 0 {
            self.circle_data = None;
            self.circle_data_scratch = None;
        }
        let total_entities = self.entities.len().saturating_sub(1);
        let dead_entities = total_entities.saturating_sub(self.alive_count as usize);
        let reorder_dead_threshold = (total_entities / 8).max(1);
        let do_entity_reorder = dead_entities >= reorder_dead_threshold && dead_entities > 0;

        if do_entity_reorder {
            let old_alive_count = self.alive_count;
            let all_rectangles = self.circle_count == 0;
            let all_circles = self.circle_count != 0 && self.circle_count == old_alive_count;
            let has_entity_types = self.typed_count != 0;

            let mut dense_owner = std::mem::take(&mut self.dense_owner);
            let mut owner_map = std::mem::take(&mut self.owner_map);

            let mut old_entities = std::mem::take(&mut self.entities);
            let old_extents = std::mem::take(&mut self.entity_extents);
            let old_query_ticks = std::mem::take(&mut self.entity_query_ticks);
            let old_values = std::mem::take(&mut self.entity_values);
            let mut old_types = std::mem::take(&mut self.entity_types);
            let mut old_circle_data = std::mem::take(&mut self.circle_data);
            if !has_entity_types {
                old_types = None;
            }
            if all_rectangles {
                old_circle_data = None;
            }

            let needed = old_entities.len();
            let mut entity_reorder_map = std::mem::take(&mut self.entity_reorder_map);
            if entity_reorder_map.len() < needed {
                entity_reorder_map.resize(needed, 0);
            }
            entity_reorder_map.fill(0);

            let mut new_entities = std::mem::take(&mut self.entities_scratch);
            new_entities.clear();
            new_entities.reserve(old_entities.len().max(1));
            let mut new_extents = std::mem::take(&mut self.entity_extents_scratch);
            new_extents.clear();
            new_extents.reserve(old_extents.len().max(1));
            let mut new_values = std::mem::take(&mut self.entity_values_scratch);
            new_values.clear();
            new_values.reserve(old_values.len().max(1));
            let mut new_query_ticks = Vec::new();
            new_query_ticks.reserve(old_query_ticks.len().max(1));
            let mut new_types = std::mem::take(&mut self.entity_types_scratch);
            if !has_entity_types {
                new_types = None;
            }
            let mut new_circle_data = std::mem::take(&mut self.circle_data_scratch);
            if all_rectangles {
                new_circle_data = None;
            }

            let old_types_vec = if has_entity_types {
                old_types
                    .take()
                    .expect("entity types missing while typed_count > 0")
            } else {
                Vec::new()
            };
            let old_circle_data_vec = if all_rectangles {
                Vec::new()
            } else {
                old_circle_data
                    .take()
                    .expect("circle data missing while circle_count > 0")
            };

            let mut new_types_vec = if has_entity_types {
                let mut vec = new_types.take().unwrap_or_default();
                vec.clear();
                vec.reserve(old_types_vec.len().max(1));
                vec
            } else {
                Vec::new()
            };
            let mut new_circle_data_vec = if all_rectangles {
                Vec::new()
            } else {
                let mut vec = new_circle_data.take().unwrap_or_default();
                vec.clear();
                vec.reserve(old_circle_data_vec.len().max(1));
                vec
            };

            // Safety: reserve ensures capacity, we manually write initialized values and set_len after.
            unsafe {
                new_entities.set_len(1);
                *new_entities.as_mut_ptr() = Entity::sentinel();
                new_extents.set_len(1);
                *new_extents.as_mut_ptr() = RectExtent::from_min_max(0.0, 0.0, 0.0, 0.0);
                new_values.set_len(1);
                *new_values.as_mut_ptr() = 0;
                new_query_ticks.set_len(1);
                *new_query_ticks.as_mut_ptr() = 0;
                if has_entity_types {
                    new_types_vec.set_len(1);
                    *new_types_vec.as_mut_ptr() = u32::MAX;
                }
                if !all_rectangles {
                    new_circle_data_vec.set_len(1);
                    *new_circle_data_vec.as_mut_ptr() = CircleData::new(0.0, 0.0, 0.0);
                }
            }

            let start_reorder = if profile {
                Some(std::time::Instant::now())
            } else {
                None
            };

            let (new_len, alive_count, circle_count);
            {
                let mut reorder = EntityReorder {
                    old_entities: old_entities.as_ptr(),
                    new_entities: new_entities.as_mut_ptr(),
                    old_extents: old_extents.as_ptr(),
                    new_extents: new_extents.as_mut_ptr(),
                    old_values: old_values.as_ptr(),
                    new_values: new_values.as_mut_ptr(),
                    old_query_ticks: old_query_ticks.as_ptr(),
                    new_query_ticks: new_query_ticks.as_mut_ptr(),
                    old_types: if has_entity_types {
                        old_types_vec.as_ptr()
                    } else {
                        std::ptr::null()
                    },
                    new_types: if has_entity_types {
                        new_types_vec.as_mut_ptr()
                    } else {
                        std::ptr::null_mut()
                    },
                    old_circle_data: if all_rectangles {
                        std::ptr::null()
                    } else {
                        old_circle_data_vec.as_ptr()
                    },
                    new_circle_data: if all_rectangles {
                        std::ptr::null_mut()
                    } else {
                        new_circle_data_vec.as_mut_ptr()
                    },
                    entity_map: entity_reorder_map.as_mut_ptr(),
                    entity_map_len: entity_reorder_map.len(),
                    new_len: 1,
                    circle_count: 0,
                    alive_count: 0,
                    all_rectangles,
                    all_circles,
                    has_entity_types,
                };

                let mut old_nodes = std::mem::take(&mut self.nodes);
                let mut old_node_entities = std::mem::take(&mut self.node_entities);
                let mut old_node_entities_next = std::mem::take(&mut self.node_entities_next);
                let mut old_node_entities_flags = std::mem::take(&mut self.node_entities_flags);

                let mut new_nodes = std::mem::take(&mut self.nodes_scratch);
                new_nodes.clear();
                new_nodes.reserve(old_nodes.len().max(1));

                let mut new_node_entities = std::mem::take(&mut self.node_entities_scratch);
                new_node_entities.clear();
                new_node_entities.reserve(old_node_entities.len().max(1));

                let mut new_node_entities_next =
                    std::mem::take(&mut self.node_entities_next_scratch);
                new_node_entities_next.clear();
                new_node_entities_next.reserve(old_node_entities_next.len().max(1));

                let mut new_node_entities_flags =
                    std::mem::take(&mut self.node_entities_flags_scratch);
                new_node_entities_flags.clear();
                new_node_entities_flags.reserve(old_node_entities_flags.len().max(1));

                new_node_entities.push(NodeEntity::new(0, false));
                new_node_entities_next.push(0);
                new_node_entities_flags.push(0);

                let start_rebuild = if profile {
                    Some(std::time::Instant::now())
                } else {
                    None
                };
                self.rebuild_nodes_iterative(
                    &mut old_nodes,
                    &mut old_node_entities,
                    &mut old_node_entities_next,
                    &mut old_node_entities_flags,
                    &mut old_entities,
                    &old_extents,
                    &mut reorder,
                    &mut new_nodes,
                    &mut new_node_entities,
                    &mut new_node_entities_next,
                    &mut new_node_entities_flags,
                    did_merge,
                );
                if let Some(start) = start_rebuild {
                    eprintln!(
                        "rebuild_nodes_iterative: {:.3}ms",
                        start.elapsed().as_secs_f64() * 1000.0
                    );
                }

                self.nodes_scratch = old_nodes;
                self.node_entities_scratch = old_node_entities;
                self.node_entities_next_scratch = old_node_entities_next;
                self.node_entities_flags_scratch = old_node_entities_flags;

                self.nodes = new_nodes;
                self.node_entities = new_node_entities;
                self.node_entities_next = new_node_entities_next;
                self.node_entities_flags = new_node_entities_flags;
                self.free_node = 0;
                self.free_node_entity = 0;

                if reorder.alive_count < old_alive_count {
                    for (old_idx, entity) in old_entities.iter().enumerate().skip(1) {
                        if entity.alive == 0 {
                            continue;
                        }
                        if unsafe { *reorder.entity_map.add(old_idx) } != 0 {
                            continue;
                        }
                        let in_nodes = entity.in_nodes_minus_one;
                        reorder.map_entity(old_idx as u32, in_nodes);
                    }
                }

                new_len = reorder.new_len;
                alive_count = reorder.alive_count;
                circle_count = reorder.circle_count;
            }

            self.entity_reorder_map = entity_reorder_map;

            unsafe {
                new_entities.set_len(new_len);
                new_extents.set_len(new_len);
                new_values.set_len(new_len);
                new_query_ticks.set_len(new_len);
            }
            if has_entity_types {
                unsafe {
                    new_types_vec.set_len(new_len);
                }
            }
            if !all_rectangles {
                unsafe {
                    new_circle_data_vec.set_len(new_len);
                }
            }

            if let Some(start) = start_reorder {
                eprintln!(
                    "rebuild_entities: {:.3}ms",
                    start.elapsed().as_secs_f64() * 1000.0
                );
            }

            self.entities_scratch = old_entities;
            self.entities = new_entities;
            self.entity_extents_scratch = old_extents;
            self.entity_extents = new_extents;
            self.entity_values_scratch = old_values;
            self.entity_values = new_values;
            self.entity_query_ticks = new_query_ticks;
            if has_entity_types {
                self.entity_types = Some(new_types_vec);
                self.entity_types_scratch = Some(old_types_vec);
            } else {
                self.entity_types = None;
                self.entity_types_scratch = None;
            }
            if all_rectangles {
                self.circle_data = None;
                self.circle_data_scratch = None;
            } else {
                self.circle_data = Some(new_circle_data_vec);
                self.circle_data_scratch = Some(old_circle_data_vec);
            }
            self.free_entity = 0;
            self.alive_count = alive_count;
            self.circle_count = circle_count;

            Self::remap_owner_indices(&self.entity_reorder_map, &mut dense_owner, &mut owner_map);
            self.dense_owner = dense_owner;
            self.owner_map = owner_map;
        } else {
            let mut old_nodes = std::mem::take(&mut self.nodes);
            let mut old_node_entities = std::mem::take(&mut self.node_entities);
            let mut old_node_entities_next = std::mem::take(&mut self.node_entities_next);
            let mut old_node_entities_flags = std::mem::take(&mut self.node_entities_flags);

            let mut new_nodes = std::mem::take(&mut self.nodes_scratch);
            new_nodes.clear();
            new_nodes.reserve(old_nodes.len().max(1));

            let mut new_node_entities = std::mem::take(&mut self.node_entities_scratch);
            new_node_entities.clear();
            new_node_entities.reserve(old_node_entities.len().max(1));

            let mut new_node_entities_next = std::mem::take(&mut self.node_entities_next_scratch);
            new_node_entities_next.clear();
            new_node_entities_next.reserve(old_node_entities_next.len().max(1));

            let mut new_node_entities_flags = std::mem::take(&mut self.node_entities_flags_scratch);
            new_node_entities_flags.clear();
            new_node_entities_flags.reserve(old_node_entities_flags.len().max(1));

            new_node_entities.push(NodeEntity::new(0, false));
            new_node_entities_next.push(0);
            new_node_entities_flags.push(0);

            let start_rebuild = if profile {
                Some(std::time::Instant::now())
            } else {
                None
            };
            let mut entities = std::mem::take(&mut self.entities);
            let entity_extents = std::mem::take(&mut self.entity_extents);
            let mut mapper = IdentityMapper;
            self.rebuild_nodes_iterative(
                &mut old_nodes,
                &mut old_node_entities,
                &mut old_node_entities_next,
                &mut old_node_entities_flags,
                &mut entities,
                &entity_extents,
                &mut mapper,
                &mut new_nodes,
                &mut new_node_entities,
                &mut new_node_entities_next,
                &mut new_node_entities_flags,
                did_merge,
            );
            if let Some(start) = start_rebuild {
                eprintln!(
                    "rebuild_nodes_iterative: {:.3}ms",
                    start.elapsed().as_secs_f64() * 1000.0
                );
            }

            self.nodes_scratch = old_nodes;
            self.node_entities_scratch = old_node_entities;
            self.node_entities_next_scratch = old_node_entities_next;
            self.node_entities_flags_scratch = old_node_entities_flags;

            self.nodes = new_nodes;
            self.node_entities = new_node_entities;
            self.node_entities_next = new_node_entities_next;
            self.node_entities_flags = new_node_entities_flags;
            self.free_node = 0;
            self.free_node_entity = 0;
            self.entities = entities;
            self.entity_extents = entity_extents;
        }

        *did_merge
    }

    #[allow(clippy::too_many_arguments)]
    fn rebuild_nodes_iterative<M: EntityMapper>(
        &mut self,
        old_nodes: &mut Vec<Node>,
        old_node_entities: &mut Vec<NodeEntity>,
        old_node_entities_next: &mut Vec<u32>,
        old_node_entities_flags: &mut Vec<u8>,
        entities: &mut [Entity],
        entity_extents: &[RectExtent],
        mapper: &mut M,
        new_nodes: &mut Vec<Node>,
        new_node_entities: &mut Vec<NodeEntity>,
        new_node_entities_next: &mut Vec<u32>,
        new_node_entities_flags: &mut Vec<u8>,
        did_merge: &mut bool,
    ) {
        let mut free_node = 0u32;
        let mut free_node_entity = 0u32;
        let mut stack = std::mem::take(&mut self.rebuild_stack);
        stack.clear();
        let stack_cap = (self.max_depth as usize)
            .saturating_mul(3)
            .saturating_add(1);
        if stack.capacity() < stack_cap {
            stack.reserve(stack_cap - stack.capacity());
        }
        let stack_ptr = stack.as_mut_ptr();
        let mut stack_len = 0usize;
        unsafe {
            stack_ptr.add(stack_len).write(NodeReorderInfo {
                node_idx: 0,
                half: self.root_half,
                parent_idx: 0,
                child_slot: 0,
                depth: 1,
            });
        }
        stack_len += 1;

        let position_flags_mask = [0b0011u8, 0b1001u8, 0b0110u8, 0b1100u8];

        while stack_len > 0 {
            stack_len -= 1;
            let info = unsafe { *stack_ptr.add(stack_len) };
            let new_node_idx = new_nodes.len() as u32;
            new_nodes.push(Node { slots: [0; 4] });
            new_nodes[info.parent_idx as usize].slots[info.child_slot as usize] = new_node_idx;

            let node_idx = info.node_idx as usize;
            let mut is_leaf = old_nodes[node_idx].is_leaf();

            if !is_leaf {
                let children = [
                    old_nodes[node_idx].child(0),
                    old_nodes[node_idx].child(1),
                    old_nodes[node_idx].child(2),
                    old_nodes[node_idx].child(3),
                ];
                debug_assert!(children.iter().all(|&child| child != 0));

                let mut total = 0u32;
                let mut can_merge = true;
                for &child_idx in &children {
                    let child = &old_nodes[child_idx as usize];
                    if !child.is_leaf() {
                        can_merge = false;
                        break;
                    }
                    total += child.count();
                }

                if can_merge && total <= self.merge_threshold {
                    *did_merge = true;
                    let node_extent = info.half.to_rect_extent();
                    self.merge_ht.fill(0);
                    let merge_mask = self.merge_ht.len() - 1;

                    let mut position_flags = 0u8;
                    let mut head = 0u32;
                    let mut count = 0u32;

                    for &child_idx in &children {
                        let child_pos = old_nodes[child_idx as usize].position_flags();
                        position_flags |= child_pos;

                        let mut current = old_nodes[child_idx as usize].head();
                        while current != 0 {
                            let entity_idx = old_node_entities[current as usize].index();
                            let next_current = old_node_entities_next[current as usize];
                            let mut hash =
                                (entity_idx as usize).wrapping_mul(2654435761) & merge_mask;

                            loop {
                                let entry = self.merge_ht[hash];
                                if entry == 0 {
                                    self.merge_ht[hash] = entity_idx;
                                    let extent = entity_extents[entity_idx as usize];
                                    old_node_entities_flags[current as usize] =
                                        Self::compute_node_entity_flags(
                                        node_extent,
                                        position_flags,
                                        extent,
                                    );
                                    old_node_entities_next[current as usize] = head;
                                    old_node_entities[current as usize].set_is_last(head == 0);
                                    head = current;
                                    count += 1;
                                    break;
                                }

                                if entry == entity_idx {
                                    let in_nodes =
                                        &mut entities[entity_idx as usize].in_nodes_minus_one;
                                    if *in_nodes > 0 {
                                        *in_nodes -= 1;
                                    }
                                    mapper.update_in_nodes_if_mapped(entity_idx, *in_nodes);
                                    old_node_entities_next[current as usize] = free_node_entity;
                                    free_node_entity = current;
                                    break;
                                }

                                hash = (hash + 1) & merge_mask;
                            }

                            current = next_current;
                        }

                        old_nodes[child_idx as usize].set_head(free_node);
                        free_node = child_idx;
                    }

                    let node = &mut old_nodes[node_idx];
                    *node = Node::new_leaf(position_flags);
                    node.set_head(head);
                    node.set_count(count);
                    is_leaf = true;
                }
            } else {
                let count = old_nodes[node_idx].count();
                let can_split = count >= self.split_threshold
                    && info.depth < self.max_depth
                    && info.half.w >= self.min_size
                    && info.half.h >= self.min_size;
                if can_split {
                    let head = old_nodes[node_idx].head();
                    let position_flags = old_nodes[node_idx].position_flags();

                    let mut child_indices = [0u32; 4];
                    for i in 0..4 {
                        let child_idx = if free_node != 0 {
                            let idx = free_node;
                            free_node = old_nodes[idx as usize].head();
                            idx
                        } else {
                            let idx = old_nodes.len() as u32;
                            old_nodes.push(Node::new_leaf(0));
                            idx
                        };
                        child_indices[i] = child_idx;
                        let child = &mut old_nodes[child_idx as usize];
                        *child = Node::new_leaf(position_flags & position_flags_mask[i]);
                    }

                    old_nodes[node_idx].set_children(child_indices);

                    let mut node_entity_idx = head;
                    while node_entity_idx != 0 {
                        let entity_idx = old_node_entities[node_entity_idx as usize].index();
                        let extent = entity_extents[entity_idx as usize];
                        let mut targets = [0usize; 4];
                        let mut targets_len = 0usize;

                        if extent.min_x <= info.half.x {
                            if extent.min_y <= info.half.y {
                                targets[targets_len] = 0;
                                targets_len += 1;
                            }
                            if extent.max_y >= info.half.y {
                                targets[targets_len] = 1;
                                targets_len += 1;
                            }
                        }
                        if extent.max_x >= info.half.x {
                            if extent.min_y <= info.half.y {
                                targets[targets_len] = 2;
                                targets_len += 1;
                            }
                            if extent.max_y >= info.half.y {
                                targets[targets_len] = 3;
                                targets_len += 1;
                            }
                        }

                        debug_assert!(targets_len > 0);

                        if targets_len > 1 {
                            let in_nodes =
                                &mut entities[entity_idx as usize].in_nodes_minus_one;
                            *in_nodes += targets_len as u32 - 1;
                            mapper.update_in_nodes_if_mapped(entity_idx, *in_nodes);
                        }

                        for target in targets.iter().take(targets_len) {
                            let child_idx = child_indices[*target];
                            let child_head = old_nodes[child_idx as usize].head();
                            let new_node_entity_idx = if free_node_entity != 0 {
                                let idx = free_node_entity;
                                free_node_entity = old_node_entities_next[idx as usize];
                                idx
                            } else {
                                let idx = old_node_entities.len() as u32;
                                old_node_entities.push(NodeEntity::new(0, false));
                                old_node_entities_next.push(0);
                                old_node_entities_flags.push(0);
                                idx
                            };
                            old_node_entities_next[new_node_entity_idx as usize] = child_head;
                            old_node_entities[new_node_entity_idx as usize].set_index(entity_idx);
                            old_node_entities[new_node_entity_idx as usize]
                                .set_is_last(child_head == 0);
                            old_node_entities_flags[new_node_entity_idx as usize] =
                                old_node_entities_flags[node_entity_idx as usize];
                            old_nodes[child_idx as usize].set_head(new_node_entity_idx);
                            let child_count = old_nodes[child_idx as usize].count();
                            old_nodes[child_idx as usize].set_count(child_count + 1);
                        }

                        let next_node_entity_idx =
                            old_node_entities_next[node_entity_idx as usize];
                        old_node_entities_next[node_entity_idx as usize] = free_node_entity;
                        free_node_entity = node_entity_idx;
                        node_entity_idx = next_node_entity_idx;
                    }

                    is_leaf = false;
                }
            }

            if !is_leaf {
                let half_w = info.half.w * 0.5;
                let half_h = info.half.h * 0.5;
                let next_depth = info.depth + 1;

                let children = [
                    old_nodes[node_idx].child(0),
                    old_nodes[node_idx].child(1),
                    old_nodes[node_idx].child(2),
                    old_nodes[node_idx].child(3),
                ];

                debug_assert!(stack_len < stack.capacity());
                unsafe {
                    stack_ptr.add(stack_len).write(NodeReorderInfo {
                        node_idx: children[0],
                        half: HalfExtent {
                            x: info.half.x - half_w,
                            y: info.half.y - half_h,
                            w: half_w,
                            h: half_h,
                        },
                        parent_idx: new_node_idx,
                        child_slot: 0,
                        depth: next_depth,
                    });
                }
                stack_len += 1;

                debug_assert!(stack_len < stack.capacity());
                unsafe {
                    stack_ptr.add(stack_len).write(NodeReorderInfo {
                        node_idx: children[1],
                        half: HalfExtent {
                            x: info.half.x - half_w,
                            y: info.half.y + half_h,
                            w: half_w,
                            h: half_h,
                        },
                        parent_idx: new_node_idx,
                        child_slot: 1,
                        depth: next_depth,
                    });
                }
                stack_len += 1;

                debug_assert!(stack_len < stack.capacity());
                unsafe {
                    stack_ptr.add(stack_len).write(NodeReorderInfo {
                        node_idx: children[2],
                        half: HalfExtent {
                            x: info.half.x + half_w,
                            y: info.half.y - half_h,
                            w: half_w,
                            h: half_h,
                        },
                        parent_idx: new_node_idx,
                        child_slot: 2,
                        depth: next_depth,
                    });
                }
                stack_len += 1;

                debug_assert!(stack_len < stack.capacity());
                unsafe {
                    stack_ptr.add(stack_len).write(NodeReorderInfo {
                        node_idx: children[3],
                        half: HalfExtent {
                            x: info.half.x + half_w,
                            y: info.half.y + half_h,
                            w: half_w,
                            h: half_h,
                        },
                        parent_idx: new_node_idx,
                        child_slot: 3,
                        depth: next_depth,
                    });
                }
                stack_len += 1;
            } else {
                let old_node = &old_nodes[node_idx];
                let position_flags = old_node.position_flags();
                let mut new_node = Node::new_leaf(position_flags);
                let head = old_node.head();
                let count = old_node.count() as usize;
                if head != 0 && count != 0 {
                    let start = new_node_entities.len();
                    new_node_entities.reserve(count);
                    new_node_entities_next.reserve(count);
                    new_node_entities_flags.reserve(count);
                    let new_head = start as u32;

                    unsafe {
                        let node_entities_ptr = new_node_entities.as_mut_ptr().add(start);
                        let node_entities_next_ptr =
                            new_node_entities_next.as_mut_ptr().add(start);
                        let node_entities_flags_ptr =
                            new_node_entities_flags.as_mut_ptr().add(start);
                        let old_node_entities_ptr = old_node_entities.as_ptr();
                        let old_node_entities_next_ptr = old_node_entities_next.as_ptr();
                        let old_node_entities_flags_ptr = old_node_entities_flags.as_ptr();
                        let entities_ptr = entities.as_ptr();

                        new_node_entities.set_len(start + count);
                        new_node_entities_next.set_len(start + count);
                        new_node_entities_flags.set_len(start + count);

                        let mut current = head;
                        let mut offset = 0usize;
                        while offset < count {
                            let entity_idx =
                                (*old_node_entities_ptr.add(current as usize)).index();
                            let in_nodes =
                                (*entities_ptr.add(entity_idx as usize)).in_nodes_minus_one;
                            let mapped_idx = mapper.map_entity(entity_idx, in_nodes);
                            let is_last = offset + 1 == count;
                            *node_entities_ptr.add(offset) =
                                NodeEntity::new(mapped_idx, is_last);
                            *node_entities_flags_ptr.add(offset) =
                                *old_node_entities_flags_ptr.add(current as usize);
                            *node_entities_next_ptr.add(offset) = if is_last {
                                0
                            } else {
                                (start + offset + 1) as u32
                            };
                            current = *old_node_entities_next_ptr.add(current as usize);
                            offset += 1;
                        }
                    }

                    new_node.set_head(new_head);
                    new_node.set_count(count as u32);
                }

                new_nodes[new_node_idx as usize] = new_node;
            }
        }

        unsafe {
            stack.set_len(0);
        }
        self.rebuild_stack = stack;
    }

    fn update_entities(&mut self) {
        self.update_pending = false;
        self.update_tick ^= 1;
        let update_tick = self.update_tick;

        let nodes_ptr = self.nodes.as_ptr();
        let node_entities_ptr = self.node_entities.as_ptr();
        let node_entities_flags_ptr = self.node_entities_flags.as_mut_ptr();
        let entities_ptr = self.entities.as_mut_ptr();
        let entity_extents_ptr = self.entity_extents.as_ptr();
        let mut node_entity_idx = 0usize;
        let mut stack = std::mem::take(&mut self.update_stack);
        stack.clear();
        stack.push((0u32, self.root_half));

        while let Some((node_idx, half)) = stack.pop() {
            let node = unsafe { &*nodes_ptr.add(node_idx as usize) };
            if !node.is_leaf() {
                for i in 0..4 {
                    let child = node.child(i);
                    if child != 0 {
                        stack.push((child, Self::child_half_extent(half, i)));
                    }
                }
                continue;
            }

            let head = node.head();
            if head == 0 {
                continue;
            }

            let node_extent = half.to_rect_extent();
            let position_flags = node.position_flags();
            let head_idx = head as usize;

            loop {
                node_entity_idx += 1;
                let node_entity = unsafe { &*node_entities_ptr.add(node_entity_idx) };
                let entity_idx = node_entity.index() as usize;
                let entity = unsafe { &mut *entities_ptr.add(entity_idx) };
                if entity.update_tick != update_tick {
                    entity.update_tick = update_tick;
                    entity.reinsertion_tick = update_tick ^ 1;
                }

                if entity.status_changed == self.status_tick {
                    let flags_ptr = unsafe { node_entities_flags_ptr.add(node_entity_idx) };
                    let mut flags = unsafe { *flags_ptr };
                    let mut crossed_new_boundary = false;
                    let extent = unsafe { *entity_extents_ptr.add(entity_idx) };

                    if extent.max_y >= node_extent.max_y && (position_flags & FLAG_TOP) == 0 {
                        if (flags & FLAG_TOP) == 0 {
                            flags |= FLAG_TOP;
                            crossed_new_boundary = true;
                        }
                    } else if (flags & FLAG_TOP) != 0 {
                        flags &= !FLAG_TOP;
                    }

                    if extent.max_x >= node_extent.max_x && (position_flags & FLAG_RIGHT) == 0 {
                        if (flags & FLAG_RIGHT) == 0 {
                            flags |= FLAG_RIGHT;
                            crossed_new_boundary = true;
                        }
                    } else if (flags & FLAG_RIGHT) != 0 {
                        flags &= !FLAG_RIGHT;
                    }

                    if extent.min_y <= node_extent.min_y && (position_flags & FLAG_BOTTOM) == 0 {
                        if (flags & FLAG_BOTTOM) == 0 {
                            flags |= FLAG_BOTTOM;
                            crossed_new_boundary = true;
                        }
                    } else if (flags & FLAG_BOTTOM) != 0 {
                        flags &= !FLAG_BOTTOM;
                    }

                    if extent.min_x <= node_extent.min_x && (position_flags & FLAG_LEFT) == 0 {
                        if (flags & FLAG_LEFT) == 0 {
                            flags |= FLAG_LEFT;
                            crossed_new_boundary = true;
                        }
                    } else if (flags & FLAG_LEFT) != 0 {
                        flags &= !FLAG_LEFT;
                    }

                    unsafe {
                        *flags_ptr = flags;
                    }

                    if crossed_new_boundary && entity.reinsertion_tick != update_tick {
                        entity.reinsertion_tick = update_tick;
                        self.reinsertions.push(entity_idx as u32);
                        self.normalization = Normalization::Hard;
                    }

                    if (extent.max_x < node_extent.min_x && (position_flags & FLAG_LEFT) == 0)
                        || (extent.max_y < node_extent.min_y
                            && (position_flags & FLAG_BOTTOM) == 0)
                        || (node_extent.max_x < extent.min_x
                            && (position_flags & FLAG_RIGHT) == 0)
                        || (node_extent.max_y < extent.min_y && (position_flags & FLAG_TOP) == 0)
                    {
                        self.node_removals.push(NodeRemoval {
                            node_idx,
                            prev_idx: if node_entity_idx == head_idx {
                                0
                            } else {
                                (node_entity_idx as u32) - 1
                            },
                            node_entity_idx: node_entity_idx as u32,
                            entity_idx: entity_idx as u32,
                        });
                        self.normalization = Normalization::Hard;
                    }
                }

                if node_entity.is_last() {
                    break;
                }
            }
        }

        self.update_stack = stack;

        self.status_tick ^= 1;
    }

    fn insert_entity_new(&mut self, entity_idx: u32) {
        self.insert_entity_inner(entity_idx);
    }

    fn insert_entity_inner(&mut self, entity_idx: u32) {
        let extent = self.entity_extent(entity_idx);
        let mut in_nodes = 0u32;

        let nodes = &mut self.nodes;
        let node_entities = &mut self.node_entities;
        let node_entities_next = &mut self.node_entities_next;
        let node_entities_flags = &mut self.node_entities_flags;
        let entities = &mut self.entities;
        let mut free_node_entity = self.free_node_entity;

        let mut stack = std::mem::take(&mut self.insert_stack);
        stack.clear();
        stack.push((0u32, self.root_half));

        while let Some((node_idx, half)) = stack.pop() {
            let node_idx_usize = node_idx as usize;
            if !nodes[node_idx_usize].is_leaf() {
                let half_w = half.w * 0.5;
                let half_h = half.h * 0.5;

                if extent.min_x <= half.x {
                    if extent.min_y <= half.y {
                        let child = nodes[node_idx_usize].child(0);
                        if child != 0 {
                            stack.push((
                                child,
                                HalfExtent {
                                    x: half.x - half_w,
                                    y: half.y - half_h,
                                    w: half_w,
                                    h: half_h,
                                },
                            ));
                        }
                    }
                    if extent.max_y >= half.y {
                        let child = nodes[node_idx_usize].child(1);
                        if child != 0 {
                            stack.push((
                                child,
                                HalfExtent {
                                    x: half.x - half_w,
                                    y: half.y + half_h,
                                    w: half_w,
                                    h: half_h,
                                },
                            ));
                        }
                    }
                }
                if extent.max_x >= half.x {
                    if extent.min_y <= half.y {
                        let child = nodes[node_idx_usize].child(2);
                        if child != 0 {
                            stack.push((
                                child,
                                HalfExtent {
                                    x: half.x + half_w,
                                    y: half.y - half_h,
                                    w: half_w,
                                    h: half_h,
                                },
                            ));
                        }
                    }
                    if extent.max_y >= half.y {
                        let child = nodes[node_idx_usize].child(3);
                        if child != 0 {
                            stack.push((
                                child,
                                HalfExtent {
                                    x: half.x + half_w,
                                    y: half.y + half_h,
                                    w: half_w,
                                    h: half_h,
                                },
                            ));
                        }
                    }
                }
                continue;
            }

            in_nodes += 1;
            let node_extent = half.to_rect_extent();
            let position_flags = nodes[node_idx_usize].position_flags();
            let node_entity_idx = if free_node_entity != 0 {
                let idx = free_node_entity;
                free_node_entity = node_entities_next[idx as usize];
                idx
            } else {
                node_entities.push(NodeEntity::new(0, false));
                node_entities_next.push(0);
                node_entities_flags.push(0);
                (node_entities.len() - 1) as u32
            };
            let head = nodes[node_idx_usize].head();
            node_entities_next[node_entity_idx as usize] = head;
            node_entities[node_entity_idx as usize].set_index(entity_idx);
            node_entities[node_entity_idx as usize].set_is_last(head == 0);
            node_entities_flags[node_entity_idx as usize] =
                Self::compute_node_entity_flags(node_extent, position_flags, extent);
            let node = &mut nodes[node_idx_usize];
            node.set_head(node_entity_idx);
            node.set_count(node.count() + 1);
        }

        self.insert_stack = stack;

        if in_nodes == 0 {
            in_nodes = 1;
        }
        entities[entity_idx as usize].in_nodes_minus_one = in_nodes - 1;
        self.free_node_entity = free_node_entity;
    }

    fn remove_entity(&mut self, entity_idx: u32) {
        let extent = self.entity_extent(entity_idx);
        let mut stack = std::mem::take(&mut self.remove_stack);
        stack.clear();
        stack.push((0u32, self.root_half));

        while let Some((node_idx, half)) = stack.pop() {
            if !self.nodes[node_idx as usize].is_leaf() {
                Self::descend(&self.nodes, node_idx, half, extent, &mut stack);
                continue;
            }

            let node = &mut self.nodes[node_idx as usize];
            let mut prev = 0u32;
            let mut current = node.head();
            while current != 0 {
                if self.node_entities[current as usize].index() == entity_idx {
                    let next = self.node_entities_next[current as usize];
                    let was_last = next == 0;
                    if prev != 0 {
                        self.node_entities_next[prev as usize] = next;
                        if was_last {
                            self.node_entities[prev as usize].set_is_last(true);
                        }
                    } else {
                        node.set_head(next);
                    }
                    let count = node.count();
                    if count > 0 {
                        node.set_count(count - 1);
                    }
                    let in_nodes = &mut self.entities[entity_idx as usize].in_nodes_minus_one;
                    if *in_nodes > 0 {
                        *in_nodes -= 1;
                    }
                    self.node_entities_next[current as usize] = self.free_node_entity;
                    self.free_node_entity = current;
                    break;
                }
                prev = current;
                current = self.node_entities_next[current as usize];
            }
        }

        self.remove_stack = stack;

        let entity = &mut self.entities[entity_idx as usize];
        if entity.alive != 0 {
            self.alive_count = self.alive_count.saturating_sub(1);
            if entity.shape_kind == SHAPE_CIRCLE {
                self.circle_count = self.circle_count.saturating_sub(1);
            }
        }
        entity.alive = 0;
        entity.status_changed = self.status_tick ^ 1;
        if let Some(types) = self.entity_types.as_mut() {
            let stored_type = types[entity_idx as usize];
            if stored_type != u32::MAX {
                self.typed_count = self.typed_count.saturating_sub(1);
            }
            types[entity_idx as usize] = u32::MAX;
        }
        if self.typed_count == 0 {
            self.entity_types = None;
            self.entity_types_scratch = None;
        }
        if self.circle_count == 0 {
            self.circle_data = None;
            self.circle_data_scratch = None;
        }
        entity.next_free = self.free_entity;
        self.free_entity = entity_idx;
    }

    #[inline(always)]
    fn compute_node_entity_flags(
        node_extent: RectExtent,
        position_flags: u8,
        entity_extent: RectExtent,
    ) -> u8 {
        let mut flags = 0u8;

        if entity_extent.max_y >= node_extent.max_y && (position_flags & FLAG_TOP) == 0 {
            flags |= FLAG_TOP;
        }
        if entity_extent.max_x >= node_extent.max_x && (position_flags & FLAG_RIGHT) == 0 {
            flags |= FLAG_RIGHT;
        }
        if entity_extent.min_y <= node_extent.min_y && (position_flags & FLAG_BOTTOM) == 0 {
            flags |= FLAG_BOTTOM;
        }
        if entity_extent.min_x <= node_extent.min_x && (position_flags & FLAG_LEFT) == 0 {
            flags |= FLAG_LEFT;
        }

        flags
    }

    #[inline(always)]
    fn child_half_extent(half: HalfExtent, index: usize) -> HalfExtent {
        let half_w = half.w * 0.5;
        let half_h = half.h * 0.5;
        match index {
            0 => HalfExtent {
                x: half.x - half_w,
                y: half.y - half_h,
                w: half_w,
                h: half_h,
            },
            1 => HalfExtent {
                x: half.x - half_w,
                y: half.y + half_h,
                w: half_w,
                h: half_h,
            },
            2 => HalfExtent {
                x: half.x + half_w,
                y: half.y - half_h,
                w: half_w,
                h: half_h,
            },
            _ => HalfExtent {
                x: half.x + half_w,
                y: half.y + half_h,
                w: half_w,
                h: half_h,
            },
        }
    }

    fn descend(
        nodes: &[Node],
        node_idx: u32,
        half: HalfExtent,
        extent: RectExtent,
        stack: &mut NodeStack,
    ) {
        let node = &nodes[node_idx as usize];
        let half_w = half.w * 0.5;
        let half_h = half.h * 0.5;

        if extent.min_x <= half.x {
            if extent.min_y <= half.y {
                let child = node.child(0);
                if child != 0 {
                    stack.push((
                        child,
                        HalfExtent {
                            x: half.x - half_w,
                            y: half.y - half_h,
                            w: half_w,
                            h: half_h,
                        },
                    ));
                }
            }
            if extent.max_y >= half.y {
                let child = node.child(1);
                if child != 0 {
                    stack.push((
                        child,
                        HalfExtent {
                            x: half.x - half_w,
                            y: half.y + half_h,
                            w: half_w,
                            h: half_h,
                        },
                    ));
                }
            }
        }
        if extent.max_x >= half.x {
            if extent.min_y <= half.y {
                let child = node.child(2);
                if child != 0 {
                    stack.push((
                        child,
                        HalfExtent {
                            x: half.x + half_w,
                            y: half.y - half_h,
                            w: half_w,
                            h: half_h,
                        },
                    ));
                }
            }
            if extent.max_y >= half.y {
                let child = node.child(3);
                if child != 0 {
                    stack.push((
                        child,
                        HalfExtent {
                            x: half.x + half_w,
                            y: half.y + half_h,
                            w: half_w,
                            h: half_h,
                        },
                    ));
                }
            }
        }
    }

    pub fn collisions_batch(&mut self, shapes: Vec<ShapeEnum>) -> Vec<Vec<u32>> {
        shapes
            .into_iter()
            .map(|shape| {
                let mut collisions = Vec::new();
                self.collisions_with(shape, |value| collisions.push(value));
                collisions
            })
            .collect()
    }

    pub fn collisions_batch_filter(
        &mut self,
        shapes: Vec<ShapeEnum>,
        filter_entity_types: Option<Vec<u32>>,
    ) -> Vec<Vec<u32>> {
        let filter = filter_entity_types.map(EntityTypeFilter::from_vec);
        shapes
            .into_iter()
            .map(|shape| {
                let mut collisions = Vec::new();
                self.collisions_from_with(&shape, filter.as_ref(), &mut |value| {
                    collisions.push(value);
                });
                collisions
            })
            .collect()
    }

    pub fn collisions(&mut self, shape: ShapeEnum, collisions: &mut Vec<u32>) {
        self.collisions_from(&shape, None, collisions);
    }

    pub fn collisions_filter(
        &mut self,
        shape: ShapeEnum,
        filter_entity_types: Option<Vec<u32>>,
        collisions: &mut Vec<u32>,
    ) {
        let filter = filter_entity_types.map(EntityTypeFilter::from_vec);
        self.collisions_from(&shape, filter.as_ref(), collisions);
    }

    pub fn collisions_with<F>(&mut self, shape: ShapeEnum, mut f: F)
    where
        F: FnMut(u32),
    {
        self.collisions_from_with(&shape, None, &mut f);
    }

    pub fn collisions_with_filter<F>(
        &mut self,
        shape: ShapeEnum,
        filter_entity_types: Option<Vec<u32>>,
        mut f: F,
    ) where
        F: FnMut(u32),
    {
        let filter = filter_entity_types.map(EntityTypeFilter::from_vec);
        self.collisions_from_with(&shape, filter.as_ref(), &mut f);
    }

    pub fn collisions_rect_extent(
        &mut self,
        min_x: f32,
        min_y: f32,
        max_x: f32,
        max_y: f32,
        collisions: &mut Vec<u32>,
    ) {
        self.collisions_rect_extent_with(min_x, min_y, max_x, max_y, |value| {
            collisions.push(value);
        });
    }

    pub fn collisions_rect_extent_with<F>(
        &mut self,
        min_x: f32,
        min_y: f32,
        max_x: f32,
        max_y: f32,
        mut f: F,
    ) where
        F: FnMut(u32),
    {
        self.normalize_hard();
        let query = Query::from_rect_extent(RectExtent::from_min_max(min_x, min_y, max_x, max_y));
        self.collisions_inner_with(query, None, &mut f);
    }

    pub fn collisions_circle_raw(
        &mut self,
        x: f32,
        y: f32,
        radius: f32,
        collisions: &mut Vec<u32>,
    ) {
        self.collisions_circle_raw_with(x, y, radius, |value| {
            collisions.push(value);
        });
    }

    pub fn collisions_circle_raw_with<F>(&mut self, x: f32, y: f32, radius: f32, mut f: F)
    where
        F: FnMut(u32),
    {
        self.normalize_hard();
        let query = Query::from_circle_raw(x, y, radius);
        self.collisions_inner_with(query, None, &mut f);
    }

    fn collisions_from(
        &mut self,
        query_shape: &ShapeEnum,
        filter_entity_types: Option<&EntityTypeFilter>,
        collisions: &mut Vec<u32>,
    ) {
        self.collisions_from_with(query_shape, filter_entity_types, &mut |value| {
            collisions.push(value);
        });
    }

    fn collisions_from_with<F>(
        &mut self,
        query_shape: &ShapeEnum,
        filter_entity_types: Option<&EntityTypeFilter>,
        f: &mut F,
    ) where
        F: FnMut(u32),
    {
        self.normalize_hard();
        let query = Query::from_shape(query_shape);
        self.collisions_inner_with(query, filter_entity_types, f);
    }

    fn collisions_inner_with<F>(
        &mut self,
        query: Query,
        filter_entity_types: Option<&EntityTypeFilter>,
        f: &mut F,
    ) where
        F: FnMut(u32),
    {
        let query_extent = query.extent;
        let query_kind = query.kind;
        let tick = self.next_query_tick();

        let all_rectangles = self.circle_count == 0;
        let all_circles = self.circle_count != 0 && self.circle_count == self.alive_count;
        if filter_entity_types.is_some() && self.entity_types.is_none() {
            return;
        }

        if filter_entity_types.is_none() {
            if all_rectangles && matches!(query_kind, QueryKind::Rect) {
                self.collisions_rect_fast_with(query_extent, tick, f);
                return;
            }

            if all_circles {
                self.collisions_circle_fast_with(query, tick, f);
                return;
            }
        }

        let mut stack = std::mem::take(&mut self.query_stack);
        stack.clear();
        stack.push((0u32, self.root_half));

        let nodes = &self.nodes;
        let node_entities = &self.node_entities;
        let entities = &mut self.entities;
        let entity_extents = &self.entity_extents;
        let entity_query_ticks = &mut self.entity_query_ticks;
        let circle_data = if all_rectangles {
            None
        } else {
            Some(
                self.circle_data
                    .as_ref()
                    .expect("circle data missing for circle entities"),
            )
        };
        let default_circle = CircleData::new(0.0, 0.0, 0.0);
        let entity_values = &self.entity_values;
        let entity_types = self.entity_types.as_ref();

        while let Some((node_idx, half)) = stack.pop() {
            let node = &nodes[node_idx as usize];
            if !node.is_leaf() {
                if let QueryKind::Circle { x, y, radius_sq, .. } = query_kind {
                    let node_extent = half.to_rect_extent();
                    let distance = point_to_extent_distance_sq(x, y, node_extent);
                    if distance > radius_sq {
                        continue;
                    }
                }
                Self::descend(nodes, node_idx, half, query_extent, &mut stack);
                continue;
            }

            let mut current = node.head();
            if current == 0 {
                continue;
            }
            loop {
                let node_entity = node_entities[current as usize];
                let entity_idx = node_entity.index();
                let entity_idx_usize = entity_idx as usize;
                let entity = &mut entities[entity_idx_usize];
                let circle = circle_data
                    .map(|data| data[entity_idx_usize])
                    .unwrap_or(default_circle);
                if entity_query_ticks[entity_idx_usize] == tick {
                    if node_entity.is_last() {
                        break;
                    }
                    current += 1;
                    continue;
                }
                entity_query_ticks[entity_idx_usize] = tick;

                if let Some(filter) = filter_entity_types {
                    let entity_type = entity_types
                        .expect("entity types missing for type filter")[entity_idx_usize];
                    if entity_type == u32::MAX || !filter.contains(entity_type) {
                        if node_entity.is_last() {
                            break;
                        }
                        current += 1;
                        continue;
                    }
                }

                let hit = match query_kind {
                    QueryKind::Rect => {
                        if all_rectangles || entity.shape_kind == SHAPE_RECT {
                            entity_extents[entity_idx_usize].intersects_strict(&query_extent)
                        } else {
                            circle_extent_raw(
                                circle.x,
                                circle.y,
                                circle.radius_sq,
                                query_extent,
                            )
                        }
                    }
                    QueryKind::Circle {
                        x,
                        y,
                        radius,
                        radius_sq,
                    } => {
                        if entity.shape_kind == SHAPE_RECT {
                            circle_extent_raw(x, y, radius_sq, entity_extents[entity_idx_usize])
                        } else {
                            circle_circle_raw(
                                x,
                                y,
                                radius,
                                circle.x,
                                circle.y,
                                circle.radius,
                            )
                        }
                    }
                };

                if hit {
                    f(entity_values[entity_idx_usize]);
                }

                if node_entity.is_last() {
                    break;
                }
                current += 1;
            }
        }

        self.query_stack = stack;
    }

    #[inline(always)]
    fn collisions_rect_fast_with<F>(
        &mut self,
        query_extent: RectExtent,
        tick: u32,
        f: &mut F,
    ) where
        F: FnMut(u32),
    {
        let q_min_x = query_extent.min_x;
        let q_max_x = query_extent.max_x;
        let q_min_y = query_extent.min_y;
        let q_max_y = query_extent.max_y;

        let nodes_ptr = self.nodes.as_ptr();
        let node_entities_ptr = self.node_entities.as_ptr();
        let entity_extents_ptr = self.entity_extents.as_ptr();
        let entity_query_ticks_ptr = self.entity_query_ticks.as_mut_ptr();
        let entity_values_ptr = self.entity_values.as_ptr();

        let mut stack = std::mem::take(&mut self.query_info_stack);
        stack.clear();
        stack.push(NodeQueryInfo {
            node_idx: 0,
            half: self.root_half,
        });

        while let Some(info) = stack.pop() {
            let node = unsafe { &*nodes_ptr.add(info.node_idx as usize) };
            if !node.is_leaf() {
                let half = info.half;
                let half_w = half.w * 0.5;
                let half_h = half.h * 0.5;

                if q_min_x <= half.x {
                    if q_min_y <= half.y {
                        stack.push(NodeQueryInfo {
                            node_idx: node.child(0),
                            half: HalfExtent {
                                x: half.x - half_w,
                                y: half.y - half_h,
                                w: half_w,
                                h: half_h,
                            },
                        });
                    }
                    if q_max_y >= half.y {
                        stack.push(NodeQueryInfo {
                            node_idx: node.child(1),
                            half: HalfExtent {
                                x: half.x - half_w,
                                y: half.y + half_h,
                                w: half_w,
                                h: half_h,
                            },
                        });
                    }
                }
                if q_max_x >= half.x {
                    if q_min_y <= half.y {
                        stack.push(NodeQueryInfo {
                            node_idx: node.child(2),
                            half: HalfExtent {
                                x: half.x + half_w,
                                y: half.y - half_h,
                                w: half_w,
                                h: half_h,
                            },
                        });
                    }
                    if q_max_y >= half.y {
                        stack.push(NodeQueryInfo {
                            node_idx: node.child(3),
                            half: HalfExtent {
                                x: half.x + half_w,
                                y: half.y + half_h,
                                w: half_w,
                                h: half_h,
                            },
                        });
                    }
                }
                continue;
            }

            let head = node.head() as usize;
            let count = node.count() as usize;
            if head == 0 || count == 0 {
                continue;
            }
            let mut current = head;
            let end = head + count;
            while current < end {
                let node_entity = unsafe { &*node_entities_ptr.add(current) };
                let entity_idx = node_entity.index() as usize;
                let entity_tick = unsafe { entity_query_ticks_ptr.add(entity_idx) };
                if unsafe { *entity_tick } != tick {
                    unsafe {
                        *entity_tick = tick;
                    }
                    let extent = unsafe { *entity_extents_ptr.add(entity_idx) };
                    if !(extent.max_x < q_min_x
                        || extent.min_x > q_max_x
                        || extent.max_y < q_min_y
                        || extent.min_y > q_max_y)
                    {
                        let value = unsafe { *entity_values_ptr.add(entity_idx) };
                        f(value);
                    }
                }
                current += 1;
            }
        }

    }

    #[inline(always)]
    fn collisions_circle_fast_with<F>(&mut self, query: Query, tick: u32, f: &mut F)
    where
        F: FnMut(u32),
    {
        let query_extent = query.extent;
        let query_kind = query.kind;
        let nodes_ptr = self.nodes.as_ptr();
        let node_entities_ptr = self.node_entities.as_ptr();
        let entity_query_ticks_ptr = self.entity_query_ticks.as_mut_ptr();
        let circle_data_ptr = self
            .circle_data
            .as_ref()
            .expect("circle data missing for circle entities")
            .as_ptr();
        let entity_values_ptr = self.entity_values.as_ptr();
        let mut stack = std::mem::take(&mut self.query_info_stack);
        stack.clear();
        stack.push(NodeQueryInfo {
            node_idx: 0,
            half: self.root_half,
        });

        while let Some(info) = stack.pop() {
            let node = unsafe { &*nodes_ptr.add(info.node_idx as usize) };
            if !node.is_leaf() {
                if let QueryKind::Circle { x, y, radius_sq, .. } = query_kind {
                    let node_extent = info.half.to_rect_extent();
                    let distance = point_to_extent_distance_sq(x, y, node_extent);
                    if distance > radius_sq {
                        continue;
                    }
                }

                let half = info.half;
                let half_w = half.w * 0.5;
                let half_h = half.h * 0.5;

                if query_extent.min_x <= half.x {
                    if query_extent.min_y <= half.y {
                        stack.push(NodeQueryInfo {
                            node_idx: node.child(0),
                            half: HalfExtent {
                                x: half.x - half_w,
                                y: half.y - half_h,
                                w: half_w,
                                h: half_h,
                            },
                        });
                    }
                    if query_extent.max_y >= half.y {
                        stack.push(NodeQueryInfo {
                            node_idx: node.child(1),
                            half: HalfExtent {
                                x: half.x - half_w,
                                y: half.y + half_h,
                                w: half_w,
                                h: half_h,
                            },
                        });
                    }
                }
                if query_extent.max_x >= half.x {
                    if query_extent.min_y <= half.y {
                        stack.push(NodeQueryInfo {
                            node_idx: node.child(2),
                            half: HalfExtent {
                                x: half.x + half_w,
                                y: half.y - half_h,
                                w: half_w,
                                h: half_h,
                            },
                        });
                    }
                    if query_extent.max_y >= half.y {
                        stack.push(NodeQueryInfo {
                            node_idx: node.child(3),
                            half: HalfExtent {
                                x: half.x + half_w,
                                y: half.y + half_h,
                                w: half_w,
                                h: half_h,
                            },
                        });
                    }
                }
                continue;
            }

            let head = node.head() as usize;
            let count = node.count() as usize;
            if head == 0 || count == 0 {
                continue;
            }
            let mut current = head;
            let end = head + count;
            while current < end {
                let node_entity = unsafe { &*node_entities_ptr.add(current) };
                let entity_idx = node_entity.index() as usize;
                let entity_tick = unsafe { entity_query_ticks_ptr.add(entity_idx) };
                if unsafe { *entity_tick } != tick {
                    unsafe {
                        *entity_tick = tick;
                    }
                    let circle = unsafe { *circle_data_ptr.add(entity_idx) };
                    let hit = match query_kind {
                        QueryKind::Rect => {
                            circle_extent_raw(circle.x, circle.y, circle.radius_sq, query_extent)
                        }
                        QueryKind::Circle {
                            x,
                            y,
                            radius,
                            radius_sq: _,
                        } => circle_circle_raw(
                            x,
                            y,
                            radius,
                            circle.x,
                            circle.y,
                            circle.radius,
                        ),
                    };

                    if hit {
                        let value = unsafe { *entity_values_ptr.add(entity_idx) };
                        f(value);
                    }
                }
                current += 1;
            }
        }

    }

    fn next_query_tick(&mut self) -> u32 {
        self.query_tick = self.query_tick.wrapping_add(1);
        if self.query_tick == 0 {
            self.query_tick = 1;
            for tick in self.entity_query_ticks.iter_mut() {
                *tick = 0;
            }
        }
        self.query_tick
    }

    pub fn for_each_collision_pair<F>(&mut self, mut f: F)
    where
        F: FnMut(u32, u32),
    {
        self.normalize_hard();
        self.for_each_collision_pair_inner(&mut f);
    }

    fn for_each_collision_pair_inner<F>(&mut self, f: &mut F)
    where
        F: FnMut(u32, u32),
    {
        self.pair_dedupe
            .ensure_capacity(self.entities.len().saturating_mul(2).max(1));
        self.pair_dedupe.clear();

        let all_rectangles = self.circle_count == 0;
        let all_circles = self.circle_count != 0 && self.circle_count == self.alive_count;
        if self.node_entities.len() <= 1 {
            return;
        }
        if all_rectangles {
            self.for_each_collision_pair_rect_fast(f);
            return;
        }
        if all_circles {
            self.for_each_collision_pair_circle_fast(f);
            return;
        }

        let node_entities = &self.node_entities;
        let entities = &self.entities;
        let entity_extents = &self.entity_extents;
        let circle_data = self
            .circle_data
            .as_ref()
            .expect("circle data missing for circle entities");
        let entity_values = &self.entity_values;
        let node_entities_len = node_entities.len();
        let pair_dedupe = &mut self.pair_dedupe;
        let mut idx = 1usize;

        while idx < node_entities_len {
            let node_entity = node_entities[idx];
            if node_entity.is_last() {
                idx += 1;
                continue;
            }

            let a_idx = node_entity.index();
            let a_idx_usize = a_idx as usize;
            let a = &entities[a_idx_usize];
            let a_extent = entity_extents[a_idx_usize];
            let a_is_circle = a.shape_kind == SHAPE_CIRCLE;
            let a_circle = circle_data[a_idx_usize];

            let mut other_idx = idx + 1;
            loop {
                let other_node_entity = node_entities[other_idx];
                let b_idx = other_node_entity.index();
                let b_idx_usize = b_idx as usize;
                let b = &entities[b_idx_usize];
                let b_is_circle = b.shape_kind == SHAPE_CIRCLE;
                let b_circle = circle_data[b_idx_usize];
                let b_extent = entity_extents[b_idx_usize];

                let hit = if !a_is_circle && !b_is_circle {
                    a_extent.intersects_strict(&b_extent)
                } else if a_is_circle && b_is_circle {
                    circle_circle_raw(
                        a_circle.x,
                        a_circle.y,
                        a_circle.radius,
                        b_circle.x,
                        b_circle.y,
                        b_circle.radius,
                    )
                } else if a_is_circle {
                    circle_extent_raw(
                        a_circle.x,
                        a_circle.y,
                        a_circle.radius_sq,
                        b_extent,
                    )
                } else {
                    circle_extent_raw(
                        b_circle.x,
                        b_circle.y,
                        b_circle.radius_sq,
                        a_extent,
                    )
                };
                if hit {
                    let needs_dedupe = a.in_nodes_minus_one > 0 || b.in_nodes_minus_one > 0;
                    if needs_dedupe {
                        let (min, max) = if a_idx < b_idx {
                            (a_idx, b_idx)
                        } else {
                            (b_idx, a_idx)
                        };
                        let key = (u64::from(min) << 32) | u64::from(max);
                        if !pair_dedupe.insert(key) {
                            if other_node_entity.is_last() {
                                break;
                            }
                            other_idx += 1;
                            continue;
                        }
                    }

                    f(entity_values[a_idx_usize], entity_values[b_idx_usize]);
                }

                if other_node_entity.is_last() {
                    break;
                }
                other_idx += 1;
            }

            idx += 1;
        }

    }

    fn for_each_collision_pair_rect_fast<F>(&mut self, f: &mut F)
    where
        F: FnMut(u32, u32),
    {
        let node_entities_ptr = self.node_entities.as_ptr();
        let node_entities_len = self.node_entities.len();
        if node_entities_len <= 1 {
            return;
        }

        let pair_dedupe = &mut self.pair_dedupe;
        let entities_ptr = self.entities.as_ptr();
        let entity_extents_ptr = self.entity_extents.as_ptr();
        let entity_values_ptr = self.entity_values.as_ptr();

        let node_entities_end = unsafe { node_entities_ptr.add(node_entities_len - 1) };
        let mut node_entity_ptr = node_entities_ptr;

        while node_entity_ptr != node_entities_end {
            node_entity_ptr = unsafe { node_entity_ptr.add(1) };
            let node_entity = unsafe { *node_entity_ptr };
            if node_entity.is_last() {
                continue;
            }

            let a_idx = node_entity.index();
            let a_entity = unsafe { &*entities_ptr.add(a_idx as usize) };
            let a_extent = unsafe { *entity_extents_ptr.add(a_idx as usize) };
            let a_in_nodes = a_entity.in_nodes_minus_one;
            let a_min_x = a_extent.min_x;
            let a_max_x = a_extent.max_x;
            let a_min_y = a_extent.min_y;
            let a_max_y = a_extent.max_y;

            let mut other_ptr = node_entity_ptr;
            loop {
                other_ptr = unsafe { other_ptr.add(1) };
                let other_node_entity = unsafe { *other_ptr };
                let b_idx = other_node_entity.index();
                let b_entity = unsafe { &*entities_ptr.add(b_idx as usize) };
                let b_extent = unsafe { *entity_extents_ptr.add(b_idx as usize) };

                if a_max_x >= b_extent.min_x
                    && a_max_y >= b_extent.min_y
                    && b_extent.max_x >= a_min_x
                    && b_extent.max_y >= a_min_y
                {
                    let b_in_nodes = b_entity.in_nodes_minus_one;
                    let needs_dedupe = a_in_nodes > 0 || b_in_nodes > 0;
                    if needs_dedupe {
                        let (min, max) = if a_idx < b_idx {
                            (a_idx, b_idx)
                        } else {
                            (b_idx, a_idx)
                        };
                        let key = (u64::from(min) << 32) | u64::from(max);
                        if !pair_dedupe.insert(key) {
                            if other_node_entity.is_last() {
                                break;
                            }
                            continue;
                        }
                    }

                    let a_value = unsafe { *entity_values_ptr.add(a_idx as usize) };
                    let b_value = unsafe { *entity_values_ptr.add(b_idx as usize) };
                    f(a_value, b_value);
                }

                if other_node_entity.is_last() {
                    break;
                }
            }
        }

    }


    fn for_each_collision_pair_circle_fast<F>(&mut self, f: &mut F)
    where
        F: FnMut(u32, u32),
    {
        let node_entities_ptr = self.node_entities.as_ptr();
        let node_entities_len = self.node_entities.len();
        if node_entities_len <= 1 {
            return;
        }

        let pair_dedupe = &mut self.pair_dedupe;
        let entities_ptr = self.entities.as_ptr();
        let circle_data_ptr = self
            .circle_data
            .as_ref()
            .expect("circle data missing for circle entities")
            .as_ptr();
        let entity_values_ptr = self.entity_values.as_ptr();

        let node_entities_end = unsafe { node_entities_ptr.add(node_entities_len - 1) };
        let mut node_entity_ptr = node_entities_ptr;

        while node_entity_ptr != node_entities_end {
            node_entity_ptr = unsafe { node_entity_ptr.add(1) };
            let node_entity = unsafe { *node_entity_ptr };
            if node_entity.is_last() {
                continue;
            }

            let a_idx = node_entity.index();
            let a_idx_usize = a_idx as usize;
            let a_entity = unsafe { &*entities_ptr.add(a_idx_usize) };
            let a_in_nodes = a_entity.in_nodes_minus_one;
            let a_circle = unsafe { *circle_data_ptr.add(a_idx_usize) };

            let mut other_ptr = node_entity_ptr;
            loop {
                other_ptr = unsafe { other_ptr.add(1) };
                let other_node_entity = unsafe { *other_ptr };
                let b_idx = other_node_entity.index();
                let b_idx_usize = b_idx as usize;
                let b_entity = unsafe { &*entities_ptr.add(b_idx_usize) };
                let b_circle = unsafe { *circle_data_ptr.add(b_idx_usize) };

                let hit = circle_circle_raw(
                    a_circle.x,
                    a_circle.y,
                    a_circle.radius,
                    b_circle.x,
                    b_circle.y,
                    b_circle.radius,
                );
                if hit {
                    let b_in_nodes = b_entity.in_nodes_minus_one;
                    let needs_dedupe = a_in_nodes > 0 || b_in_nodes > 0;
                    if needs_dedupe {
                        let (min, max) = if a_idx < b_idx {
                            (a_idx, b_idx)
                        } else {
                            (b_idx, a_idx)
                        };
                        let key = (u64::from(min) << 32) | u64::from(max);
                        if !pair_dedupe.insert(key) {
                            if other_node_entity.is_last() {
                                break;
                            }
                            continue;
                        }
                    }

                    let a_value = unsafe { *entity_values_ptr.add(a_idx_usize) };
                    let b_value = unsafe { *entity_values_ptr.add(b_idx_usize) };
                    f(a_value, b_value);
                }

                if other_node_entity.is_last() {
                    break;
                }
            }
        }

    }

    pub fn all_node_bounding_boxes(&mut self, bounding_boxes: &mut Vec<Rectangle>) {
        self.normalize_hard();
        let mut stack = std::mem::take(&mut self.update_stack);
        stack.clear();
        stack.push((0u32, self.root_half));

        while let Some((node_idx, half)) = stack.pop() {
            let node = &self.nodes[node_idx as usize];
            bounding_boxes.push(Rectangle {
                x: half.x,
                y: half.y,
                width: half.w * 2.0,
                height: half.h * 2.0,
            });

            if !node.is_leaf() {
                for i in 0..4 {
                    let child = node.child(i);
                    if child != 0 {
                        stack.push((child, Self::child_half_extent(half, i)));
                    }
                }
            }
        }

        self.update_stack = stack;
    }

    pub fn all_shapes(&self, shapes: &mut Vec<ShapeEnum>) {
        for (idx, entity) in self.entities.iter().enumerate().skip(1) {
            if entity.alive != 0 {
                if entity.shape_kind == SHAPE_CIRCLE {
                    let circle = self
                        .circle_data
                        .as_ref()
                        .expect("circle data missing for circle entities")[idx];
                    shapes.push(ShapeEnum::Circle(Circle::new(
                        circle.x,
                        circle.y,
                        circle.radius,
                    )));
                } else {
                    let extent = self.entity_extents[idx];
                    shapes.push(ShapeEnum::Rectangle(Rectangle {
                        x: (extent.min_x + extent.max_x) * 0.5,
                        y: (extent.min_y + extent.max_y) * 0.5,
                        width: extent.max_x - extent.min_x,
                        height: extent.max_y - extent.min_y,
                    }));
                }
            }
        }
    }
}

impl QuadTree {
    pub fn new_with_config(bounding_box: Rectangle, config: Config) -> Self {
        Self {
            inner: RefCell::new(QuadTreeInner::new_with_config(bounding_box, config)),
        }
    }

    pub fn new(bounding_box: Rectangle) -> Self {
        Self {
            inner: RefCell::new(QuadTreeInner::new(bounding_box)),
        }
    }

    pub fn insert(&mut self, value: u32, shape: ShapeEnum, entity_type: Option<u32>) {
        self.inner.get_mut().insert(value, shape, entity_type);
    }

    pub fn insert_rect_extent(
        &mut self,
        value: u32,
        min_x: f32,
        min_y: f32,
        max_x: f32,
        max_y: f32,
        entity_type: Option<u32>,
    ) {
        self.inner
            .get_mut()
            .insert_rect_extent(value, min_x, min_y, max_x, max_y, entity_type);
    }

    pub fn insert_circle_raw(
        &mut self,
        value: u32,
        x: f32,
        y: f32,
        radius: f32,
        entity_type: Option<u32>,
    ) {
        self.inner
            .get_mut()
            .insert_circle_raw(value, x, y, radius, entity_type);
    }

    pub fn delete(&mut self, value: u32) {
        self.inner.get_mut().delete(value);
    }

    pub fn relocate_batch(&mut self, relocation_requests: Vec<RelocationRequest>) {
        self.inner.get_mut().relocate_batch(relocation_requests);
    }

    pub fn relocate(&mut self, value: u32, shape: ShapeEnum, entity_type: Option<u32>) {
        self.inner.get_mut().relocate(value, shape, entity_type);
    }

    pub fn relocate_rect_extent(
        &mut self,
        value: u32,
        min_x: f32,
        min_y: f32,
        max_x: f32,
        max_y: f32,
        entity_type: Option<u32>,
    ) {
        self.inner
            .get_mut()
            .relocate_rect_extent(value, min_x, min_y, max_x, max_y, entity_type);
    }

    pub fn relocate_circle_raw(
        &mut self,
        value: u32,
        x: f32,
        y: f32,
        radius: f32,
        entity_type: Option<u32>,
    ) {
        self.inner
            .get_mut()
            .relocate_circle_raw(value, x, y, radius, entity_type);
    }

    pub fn update(&self) {
        self.inner.borrow_mut().update();
    }

    pub fn collisions_batch(&self, shapes: Vec<ShapeEnum>) -> Vec<Vec<u32>> {
        self.inner.borrow_mut().collisions_batch(shapes)
    }

    pub fn collisions_batch_filter(
        &self,
        shapes: Vec<ShapeEnum>,
        filter_entity_types: Option<Vec<u32>>,
    ) -> Vec<Vec<u32>> {
        self.inner
            .borrow_mut()
            .collisions_batch_filter(shapes, filter_entity_types)
    }

    pub fn collisions(&self, shape: ShapeEnum, collisions: &mut Vec<u32>) {
        self.inner.borrow_mut().collisions(shape, collisions);
    }

    pub fn collisions_rect_extent(
        &self,
        min_x: f32,
        min_y: f32,
        max_x: f32,
        max_y: f32,
        collisions: &mut Vec<u32>,
    ) {
        self.inner
            .borrow_mut()
            .collisions_rect_extent(min_x, min_y, max_x, max_y, collisions);
    }

    pub fn collisions_circle_raw(
        &self,
        x: f32,
        y: f32,
        radius: f32,
        collisions: &mut Vec<u32>,
    ) {
        self.inner
            .borrow_mut()
            .collisions_circle_raw(x, y, radius, collisions);
    }

    pub fn collisions_filter(
        &self,
        shape: ShapeEnum,
        filter_entity_types: Option<Vec<u32>>,
        collisions: &mut Vec<u32>,
    ) {
        self.inner
            .borrow_mut()
            .collisions_filter(shape, filter_entity_types, collisions);
    }

    pub fn collisions_with<F>(&self, shape: ShapeEnum, f: F)
    where
        F: FnMut(u32),
    {
        self.inner.borrow_mut().collisions_with(shape, f);
    }

    pub fn collisions_rect_extent_with<F>(
        &self,
        min_x: f32,
        min_y: f32,
        max_x: f32,
        max_y: f32,
        f: F,
    ) where
        F: FnMut(u32),
    {
        self.inner
            .borrow_mut()
            .collisions_rect_extent_with(min_x, min_y, max_x, max_y, f);
    }

    pub fn collisions_circle_raw_with<F>(&self, x: f32, y: f32, radius: f32, f: F)
    where
        F: FnMut(u32),
    {
        self.inner
            .borrow_mut()
            .collisions_circle_raw_with(x, y, radius, f);
    }

    pub fn collisions_with_filter<F>(
        &self,
        shape: ShapeEnum,
        filter_entity_types: Option<Vec<u32>>,
        f: F,
    ) where
        F: FnMut(u32),
    {
        self.inner
            .borrow_mut()
            .collisions_with_filter(shape, filter_entity_types, f);
    }

    pub fn for_each_collision_pair<F>(&self, f: F)
    where
        F: FnMut(u32, u32),
    {
        self.inner.borrow_mut().for_each_collision_pair(f);
    }

    pub fn all_node_bounding_boxes(&self, bounding_boxes: &mut Vec<Rectangle>) {
        self.inner.borrow_mut().all_node_bounding_boxes(bounding_boxes);
    }

    pub fn all_shapes(&self, shapes: &mut Vec<ShapeEnum>) {
        self.inner.borrow().all_shapes(shapes);
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub pool_size: usize,
    pub node_capacity: usize,
    pub max_depth: usize,
    pub min_size: f32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            pool_size: 4000,
            node_capacity: 4,
            max_depth: 6,
            min_size: 1.0,
        }
    }
}

#[derive(Clone)]
pub struct RelocationRequest {
    pub value: u32,
    pub shape: ShapeEnum,
    pub entity_type: Option<u32>,
}
