use common::shapes::{Rectangle, Shape, ShapeEnum};
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
struct RectExtent {
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
}

impl RectExtent {
    fn from_rect(rect: &Rectangle) -> Self {
        Self {
            min_x: rect.left(),
            max_x: rect.right(),
            min_y: rect.top(),
            max_y: rect.bottom(),
        }
    }

    fn intersects_strict(&self, other: &RectExtent) -> bool {
        self.min_x < other.max_x
            && self.max_x > other.min_x
            && self.min_y < other.max_y
            && self.max_y > other.min_y
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

    fn to_rect_extent(self) -> RectExtent {
        RectExtent {
            min_x: self.x - self.w,
            max_x: self.x + self.w,
            min_y: self.y - self.h,
            max_y: self.y + self.h,
        }
    }
}

type NodeStack = SmallVec<[(u32, HalfExtent); 64]>;

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

    fn zero() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            radius: 0.0,
            radius_sq: 0.0,
        }
    }
}

#[derive(Clone, Copy)]
enum QueryKind {
    Rect { rect: Rectangle },
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
                kind: QueryKind::Rect { rect: *rect },
            },
            ShapeEnum::Circle(circle) => {
                let rect = circle.bounding_box();
                Self {
                    extent: RectExtent::from_rect(&rect),
                    kind: QueryKind::Circle {
                        x: circle.x,
                        y: circle.y,
                        radius: circle.radius,
                        radius_sq: circle.radius * circle.radius,
                    },
                }
            }
        }
    }
}

fn circle_circle_raw(ax: f32, ay: f32, ar: f32, bx: f32, by: f32, br: f32) -> bool {
    let dx = ax - bx;
    let dy = ay - by;
    let radius_sum = ar + br;
    dx * dx + dy * dy < radius_sum * radius_sum
}

fn circle_rect_raw(cx: f32, cy: f32, radius: f32, radius_sq: f32, rect: &Rectangle) -> bool {
    let dx = (cx - rect.x).abs();
    let dy = (cy - rect.y).abs();
    let half_rect_width = rect.width * 0.5;
    let half_rect_height = rect.height * 0.5;

    if dx > half_rect_width + radius || dy > half_rect_height + radius {
        return false;
    }

    if dx <= half_rect_width || dy <= half_rect_height {
        return true;
    }

    let corner_dx = dx - half_rect_width;
    let corner_dy = dy - half_rect_height;
    corner_dx * corner_dx + corner_dy * corner_dy <= radius_sq
}

#[derive(Clone, Copy)]
#[repr(transparent)]
struct NodeEntity(u32);

impl NodeEntity {
    const INDEX_MASK: u32 = 0x7fff_ffff;
    const LAST_MASK: u32 = 0x8000_0000;

    fn new(index: u32, is_last: bool) -> Self {
        let mut value = index & Self::INDEX_MASK;
        if is_last {
            value |= Self::LAST_MASK;
        }
        NodeEntity(value)
    }

    fn index(self) -> u32 {
        self.0 & Self::INDEX_MASK
    }

    fn is_last(self) -> bool {
        (self.0 & Self::LAST_MASK) != 0
    }

    fn set_index(&mut self, index: u32) {
        self.0 = (self.0 & Self::LAST_MASK) | (index & Self::INDEX_MASK);
    }

    fn set_is_last(&mut self, is_last: bool) {
        if is_last {
            self.0 |= Self::LAST_MASK;
        } else {
            self.0 &= Self::INDEX_MASK;
        }
    }
}

struct Node {
    slots: [u32; 4],
}

impl Node {
    fn new_leaf(position_flags: u8) -> Self {
        Self {
            slots: [0, position_flags as u32, 0, 0],
        }
    }

    fn reset_leaf(&mut self, position_flags: u8) {
        self.slots[0] = 0;
        self.slots[1] = position_flags as u32;
        self.slots[2] = 0;
        self.slots[3] = 0;
    }

    fn set_free_next(&mut self, next: u32) {
        self.slots[0] = next;
        self.slots[1] = 0;
        self.slots[2] = 0;
        self.slots[3] = 0;
    }

    fn head(&self) -> u32 {
        self.slots[0]
    }

    fn set_head(&mut self, head: u32) {
        self.slots[0] = head;
    }

    fn position_flags(&self) -> u8 {
        self.slots[1] as u8
    }

    fn count(&self) -> u32 {
        self.slots[2]
    }

    fn set_count(&mut self, count: u32) {
        self.slots[2] = count;
    }

    fn is_leaf(&self) -> bool {
        self.slots[3] == 0
    }

    fn set_children(&mut self, children: [u32; 4]) {
        self.slots = children;
    }

    fn child(&self, index: usize) -> u32 {
        self.slots[index]
    }
}

#[derive(Clone)]
struct Entity {
    value: u32,
    shape_kind: u8,
    entity_type: Option<u32>,
    extent: RectExtent,
    bbox: Rectangle,
    in_nodes_minus_one: u32,
    query_tick: u32,
    update_tick: u8,
    reinsertion_tick: u8,
    status_changed: bool,
    alive: bool,
    next_free: u32,
}

impl Entity {
    fn new(
        value: u32,
        shape_kind: u8,
        bbox: Rectangle,
        extent: RectExtent,
        entity_type: Option<u32>,
    ) -> Self {
        Self {
            value,
            shape_kind,
            entity_type,
            extent,
            bbox,
            in_nodes_minus_one: 0,
            query_tick: 0,
            update_tick: 0,
            reinsertion_tick: 0,
            status_changed: false,
            alive: true,
            next_free: 0,
        }
    }

    fn reset(
        &mut self,
        value: u32,
        shape_kind: u8,
        bbox: Rectangle,
        extent: RectExtent,
        entity_type: Option<u32>,
    ) {
        self.value = value;
        self.shape_kind = shape_kind;
        self.entity_type = entity_type;
        self.bbox = bbox;
        self.extent = extent;
        self.in_nodes_minus_one = 0;
        self.query_tick = 0;
        self.update_tick = 0;
        self.reinsertion_tick = 0;
        self.status_changed = false;
        self.alive = true;
        self.next_free = 0;
    }
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
    used: Vec<usize>,
}

impl PairDedupe {
    fn new() -> Self {
        Self {
            table: Vec::new(),
            used: Vec::new(),
        }
    }

    fn ensure_capacity(&mut self, desired: usize) {
        let mut size = desired.next_power_of_two();
        if size < 1024 {
            size = 1024;
        }
        if self.table.len() < size {
            self.table.resize(size, 0);
        }
    }

    fn clear(&mut self) {
        for &idx in &self.used {
            self.table[idx] = 0;
        }
        self.used.clear();
    }

    fn insert(&mut self, key: u64) -> bool {
        let mask = self.table.len() - 1;
        let mut idx = (key as usize).wrapping_mul(2654435761) & mask;
        loop {
            let slot = self.table[idx];
            if slot == 0 {
                self.table[idx] = key;
                self.used.push(idx);
                return true;
            }
            if slot == key {
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
    free_node: u32,
    node_entities: Vec<NodeEntity>,
    node_entities_next: Vec<u32>,
    node_entities_flags: Vec<u8>,
    free_node_entity: u32,
    entities: Vec<Entity>,
    entity_shapes: Vec<ShapeEnum>,
    circle_x: Vec<f32>,
    circle_y: Vec<f32>,
    circle_radius: Vec<f32>,
    circle_radius_sq: Vec<f32>,
    free_entity: u32,
    insertions: Vec<u32>,
    removals: Vec<u32>,
    node_removals: Vec<NodeRemoval>,
    reinsertions: Vec<u32>,
    merge_ht: Vec<u32>,
    normalization: Normalization,
    update_tick: u8,
    query_tick: u32,
    update_pending: bool,
    split_threshold: u32,
    merge_threshold: u32,
    max_depth: u32,
    min_size: f32,
    owner_map: FxHashMap<u32, u32>,
    dense_owner: Vec<u32>,
    pair_dedupe: PairDedupe,
    changed_entities: Vec<u32>,
    insert_stack: NodeStack,
    remove_stack: NodeStack,
    query_stack: NodeStack,
    update_stack: NodeStack,
    circle_count: u32,
    alive_count: u32,
    rebalance_pending: bool,
}

impl QuadTreeInner {
    const DENSE_OWNER_LIMIT: usize = 1_000_000;

    pub fn new_with_config(bounding_box: Rectangle, config: Config) -> Self {
        let root_extent = RectExtent::from_rect(&bounding_box);
        let root_half = HalfExtent::from_rect_extent(root_extent);
        let split_threshold = config.node_capacity as u32;
        let merge_threshold = split_threshold.saturating_sub(1).max(1);
        let max_depth = config.max_depth as u32;
        let min_size = 1.0;
        let merge_ht_size = (merge_threshold * 2).next_power_of_two().max(8) as usize;

        let mut nodes = Vec::new();
        nodes.push(Node::new_leaf(
            FLAG_LEFT | FLAG_RIGHT | FLAG_TOP | FLAG_BOTTOM,
        ));

        let mut node_entities = Vec::new();
        node_entities.push(NodeEntity::new(0, false));

        let mut node_entities_next = Vec::new();
        node_entities_next.push(0);

        let mut node_entities_flags = Vec::new();
        node_entities_flags.push(0);

        let mut entities = Vec::new();
        let sentinel_bbox = Rectangle::default();
        let sentinel_extent = RectExtent::from_rect(&sentinel_bbox);
        entities.push(Entity::new(
            0,
            SHAPE_RECT,
            sentinel_bbox,
            sentinel_extent,
            None,
        ));
        entities[0].alive = false;
        let mut entity_shapes = Vec::new();
        entity_shapes.push(ShapeEnum::Rectangle(Rectangle::default()));
        let mut circle_x = Vec::new();
        circle_x.push(0.0);
        let mut circle_y = Vec::new();
        circle_y.push(0.0);
        let mut circle_radius = Vec::new();
        circle_radius.push(0.0);
        let mut circle_radius_sq = Vec::new();
        circle_radius_sq.push(0.0);

        Self {
            root_half,
            nodes,
            free_node: 0,
            node_entities,
            node_entities_next,
            node_entities_flags,
            free_node_entity: 0,
            entities,
            entity_shapes,
            circle_x,
            circle_y,
            circle_radius,
            circle_radius_sq,
            free_entity: 0,
            insertions: Vec::new(),
            removals: Vec::new(),
            node_removals: Vec::new(),
            reinsertions: Vec::new(),
            merge_ht: vec![0; merge_ht_size],
            normalization: Normalization::Normal,
            update_tick: 0,
            query_tick: 0,
            update_pending: false,
            split_threshold,
            merge_threshold,
            max_depth,
            min_size,
            owner_map: FxHashMap::default(),
            dense_owner: Vec::new(),
            pair_dedupe: PairDedupe::new(),
            changed_entities: Vec::new(),
            insert_stack: NodeStack::with_capacity(
                (max_depth as usize).saturating_mul(3).saturating_add(1),
            ),
            remove_stack: NodeStack::with_capacity(
                (max_depth as usize).saturating_mul(3).saturating_add(1),
            ),
            query_stack: NodeStack::with_capacity(
                (max_depth as usize).saturating_mul(3).saturating_add(1),
            ),
            update_stack: NodeStack::with_capacity(
                (max_depth as usize)
                    .saturating_mul(3)
                    .saturating_add(1),
            ),
            circle_count: 0,
            alive_count: 0,
            rebalance_pending: false,
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

    fn alloc_node(&mut self, position_flags: u8) -> u32 {
        let idx = if self.free_node != 0 {
            let idx = self.free_node;
            self.free_node = self.nodes[idx as usize].head();
            idx
        } else {
            self.nodes.push(Node::new_leaf(position_flags));
            return (self.nodes.len() - 1) as u32;
        };

        let node = &mut self.nodes[idx as usize];
        node.reset_leaf(position_flags);

        idx
    }

    fn alloc_node_entity(&mut self) -> u32 {
        if self.free_node_entity != 0 {
            let idx = self.free_node_entity;
            self.free_node_entity = self.node_entities_next[idx as usize];
            idx
        } else {
            self.node_entities.push(NodeEntity::new(0, false));
            self.node_entities_next.push(0);
            self.node_entities_flags.push(0);
            (self.node_entities.len() - 1) as u32
        }
    }

    fn alloc_entity(&mut self, value: u32, shape: ShapeEnum, entity_type: Option<u32>) -> u32 {
        let (shape_kind, bbox, extent, circle_data) = Self::shape_metadata(&shape);
        let idx = if self.free_entity != 0 {
            let idx = self.free_entity;
            let next = self.entities[idx as usize].next_free;
            self.free_entity = next;
            self.entities[idx as usize].reset(value, shape_kind, bbox, extent, entity_type);
            self.entity_shapes[idx as usize] = shape;
            idx
        } else {
            self.entities
                .push(Entity::new(value, shape_kind, bbox, extent, entity_type));
            self.entity_shapes.push(shape);
            self.circle_x.push(0.0);
            self.circle_y.push(0.0);
            self.circle_radius.push(0.0);
            self.circle_radius_sq.push(0.0);
            (self.entities.len() - 1) as u32
        };

        self.set_circle_data(idx as usize, circle_data);
        self.alive_count = self.alive_count.saturating_add(1);
        if shape_kind == SHAPE_CIRCLE {
            self.circle_count = self.circle_count.saturating_add(1);
        }

        let entity = &mut self.entities[idx as usize];
        entity.query_tick = self.query_tick;
        entity.update_tick = self.update_tick;
        entity.reinsertion_tick = self.update_tick;

        idx
    }

    fn entity_extent(&self, entity_idx: u32) -> RectExtent {
        self.entities[entity_idx as usize].extent
    }

    fn set_circle_data(&mut self, idx: usize, data: Option<CircleData>) {
        let data = data.unwrap_or_else(CircleData::zero);
        self.circle_x[idx] = data.x;
        self.circle_y[idx] = data.y;
        self.circle_radius[idx] = data.radius;
        self.circle_radius_sq[idx] = data.radius_sq;
    }

    fn shape_metadata(shape: &ShapeEnum) -> (u8, Rectangle, RectExtent, Option<CircleData>) {
        let bbox = shape.bounding_box();
        let extent = RectExtent::from_rect(&bbox);
        match shape {
            ShapeEnum::Rectangle(_) => (SHAPE_RECT, bbox, extent, None),
            ShapeEnum::Circle(circle) => (
                SHAPE_CIRCLE,
                bbox,
                extent,
                Some(CircleData::new(circle.x, circle.y, circle.radius)),
            ),
        }
    }

    pub fn insert(&mut self, value: u32, shape: ShapeEnum, entity_type: Option<u32>) {
        if self.owner_lookup(value).is_some() {
            self.delete(value);
        }

        let entity_idx = self.alloc_entity(value, shape, entity_type);
        self.owner_insert(value, entity_idx);
        self.insertions.push(entity_idx);
        self.normalization = Normalization::Hard;
        self.rebalance_pending = true;
    }

    pub fn delete(&mut self, value: u32) {
        let entity_idx = match self.owner_remove(value) {
            Some(idx) => idx,
            None => return,
        };
        self.removals.push(entity_idx);
        self.normalization = Normalization::Hard;
        self.rebalance_pending = true;
    }

    pub fn relocate_batch(&mut self, relocation_requests: Vec<RelocationRequest>) {
        for request in relocation_requests {
            self.relocate(request.value, request.shape, request.entity_type);
        }
    }

    pub fn relocate(&mut self, value: u32, shape: ShapeEnum, entity_type: Option<u32>) {
        let entity_idx = match self.owner_lookup(value) {
            Some(idx) => idx,
            None => {
                self.insert(value, shape, entity_type);
                return;
            }
        };

        let (shape_kind, bbox, extent, circle_data) = Self::shape_metadata(&shape);
        let prev_kind = self.entities[entity_idx as usize].shape_kind;
        if prev_kind != shape_kind {
            if prev_kind == SHAPE_CIRCLE {
                self.circle_count = self.circle_count.saturating_sub(1);
            } else if shape_kind == SHAPE_CIRCLE {
                self.circle_count = self.circle_count.saturating_add(1);
            }
        }
        self.entity_shapes[entity_idx as usize] = shape;
        self.set_circle_data(entity_idx as usize, circle_data);
        let entity = &mut self.entities[entity_idx as usize];
        entity.shape_kind = shape_kind;
        entity.entity_type = entity_type;
        entity.bbox = bbox;
        entity.extent = extent;
        if !entity.status_changed {
            entity.status_changed = true;
            self.changed_entities.push(entity_idx);
        }
        self.update_pending = true;
    }

    pub fn update(&mut self) {
        self.normalize_hard();
    }

    fn normalize_hard(&mut self) {
        if self.normalization == Normalization::Normal && !self.update_pending {
            return;
        }

        self.normalize();

        if self.update_pending {
            self.update_entities();
            if self.normalization != Normalization::Normal {
                self.normalize();
            }
        }
    }

    fn normalize(&mut self) {
        if self.normalization == Normalization::Normal {
            return;
        }

        self.normalization = Normalization::Normal;

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

                let entity = &mut self.entities[removal.entity_idx as usize];
                if entity.in_nodes_minus_one > 0 {
                    entity.in_nodes_minus_one -= 1;
                }

                self.node_entities_next[node_entity_idx as usize] = self.free_node_entity;
                self.free_node_entity = node_entity_idx;
            }

            self.node_removals.clear();
        }

        if !self.reinsertions.is_empty() {
            let reinsertions = std::mem::take(&mut self.reinsertions);
            for entity_idx in reinsertions {
                if !self.entities[entity_idx as usize].alive {
                    continue;
                }
                self.reinsert_entity(entity_idx);
            }
        }

        if !self.removals.is_empty() {
            let removals = std::mem::take(&mut self.removals);
            for entity_idx in removals {
                self.remove_entity(entity_idx);
            }
        }

        if !self.insertions.is_empty() {
            let insertions = std::mem::take(&mut self.insertions);
            for entity_idx in insertions {
                if !self.entities[entity_idx as usize].alive {
                    continue;
                }
                self.insert_entity_new(entity_idx);
            }
        }

        if self.rebalance_pending {
            self.rebalance();
            self.rebalance_pending = false;
        }
        self.compact_storage();
    }

    fn update_entities(&mut self) {
        self.update_pending = false;
        self.update_tick ^= 1;
        let update_tick = self.update_tick;

        let nodes_ptr = self.nodes.as_ptr();
        let node_entities_ptr = self.node_entities.as_ptr();
        let node_entities_flags_ptr = self.node_entities_flags.as_mut_ptr();
        let entities_ptr = self.entities.as_mut_ptr();
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

                if entity.status_changed {
                    let flags_ptr = unsafe { node_entities_flags_ptr.add(node_entity_idx) };
                    let mut flags = unsafe { *flags_ptr };
                    let mut crossed_new_boundary = false;
                    let extent = entity.extent;

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

        for &entity_idx in &self.changed_entities {
            self.entities[entity_idx as usize].status_changed = false;
        }
        self.changed_entities.clear();
    }

    fn insert_entity(&mut self, entity_idx: u32) {
        self.insert_entity_inner(entity_idx, true);
    }

    fn insert_entity_new(&mut self, entity_idx: u32) {
        self.insert_entity_inner(entity_idx, false);
    }

    fn insert_entity_inner(&mut self, entity_idx: u32, check_duplicates: bool) {
        let extent = self.entity_extent(entity_idx);
        let mut in_nodes = 0u32;

        let mut stack = std::mem::take(&mut self.insert_stack);
        stack.clear();
        stack.push((0u32, self.root_half));

        while let Some((node_idx, half)) = stack.pop() {
            if !self.nodes[node_idx as usize].is_leaf() {
                Self::descend(&self.nodes, node_idx, half, extent, &mut stack);
                continue;
            }

            in_nodes += 1;
            let node_extent = half.to_rect_extent();
            let position_flags = self.nodes[node_idx as usize].position_flags();
            let mut node_entity_idx = 0u32;
            if check_duplicates {
                node_entity_idx = self.nodes[node_idx as usize].head();
                while node_entity_idx != 0 {
                    if self.node_entities[node_entity_idx as usize].index() == entity_idx {
                        break;
                    }
                    node_entity_idx = self.node_entities_next[node_entity_idx as usize];
                }
            }

            if node_entity_idx == 0 {
                let node_entity_idx = self.alloc_node_entity();
                let head = self.nodes[node_idx as usize].head();
                self.node_entities_next[node_entity_idx as usize] = head;
                self.node_entities[node_entity_idx as usize].set_index(entity_idx);
                self.node_entities[node_entity_idx as usize].set_is_last(head == 0);
                self.node_entities_flags[node_entity_idx as usize] =
                    Self::compute_node_entity_flags(node_extent, position_flags, extent);
                let node = &mut self.nodes[node_idx as usize];
                node.set_head(node_entity_idx);
                node.set_count(node.count() + 1);
            } else {
                self.node_entities_flags[node_entity_idx as usize] =
                    Self::compute_node_entity_flags(node_extent, position_flags, extent);
            }
        }

        self.insert_stack = stack;

        if in_nodes == 0 {
            in_nodes = 1;
        }
        self.entities[entity_idx as usize].in_nodes_minus_one = in_nodes - 1;
    }

    fn reinsert_entity(&mut self, entity_idx: u32) {
        self.insert_entity(entity_idx);
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
                    let entity = &mut self.entities[entity_idx as usize];
                    if entity.in_nodes_minus_one > 0 {
                        entity.in_nodes_minus_one -= 1;
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
        if entity.alive {
            self.alive_count = self.alive_count.saturating_sub(1);
            if entity.shape_kind == SHAPE_CIRCLE {
                self.circle_count = self.circle_count.saturating_sub(1);
            }
        }
        entity.alive = false;
        entity.next_free = self.free_entity;
        self.free_entity = entity_idx;
    }

    fn rebalance(&mut self) {
        self.rebalance_node(0, 0, self.root_half);
    }

    fn compact_storage(&mut self) {
        let old_node_entities = std::mem::take(&mut self.node_entities);
        let old_node_entities_next = std::mem::take(&mut self.node_entities_next);
        let old_node_entities_flags = std::mem::take(&mut self.node_entities_flags);

        let mut new_node_entities = Vec::with_capacity(old_node_entities.len().max(1));
        let mut new_node_entities_next = Vec::with_capacity(old_node_entities_next.len().max(1));
        let mut new_node_entities_flags = Vec::with_capacity(old_node_entities_flags.len().max(1));

        new_node_entities.push(NodeEntity::new(0, false));
        new_node_entities_next.push(0);
        new_node_entities_flags.push(0);

        let mut stack: SmallVec<[u32; 64]> = SmallVec::new();
        stack.push(0);

        while let Some(node_idx) = stack.pop() {
            let node = &mut self.nodes[node_idx as usize];
            if !node.is_leaf() {
                for i in 0..4 {
                    let child = node.child(i);
                    if child != 0 {
                        stack.push(child);
                    }
                }
                continue;
            }

            let head = node.head();
            if head == 0 {
                continue;
            }

            node.set_head(new_node_entities.len() as u32);
            let mut node_entity_idx = head;
            loop {
                let old_entity_idx = old_node_entities[node_entity_idx as usize].index();
                let is_last = old_node_entities_next[node_entity_idx as usize] == 0;
                let new_node_entity_idx = new_node_entities.len() as u32;
                new_node_entities.push(NodeEntity::new(old_entity_idx, is_last));
                new_node_entities_flags.push(old_node_entities_flags[node_entity_idx as usize]);
                new_node_entities_next.push(if is_last {
                    0
                } else {
                    new_node_entity_idx + 1
                });

                if is_last {
                    break;
                }
                node_entity_idx = old_node_entities_next[node_entity_idx as usize];
            }
        }

        self.node_entities = new_node_entities;
        self.node_entities_next = new_node_entities_next;
        self.node_entities_flags = new_node_entities_flags;
        self.free_node_entity = 0;
    }

    fn rebalance_node(&mut self, node_idx: u32, depth: u32, half: HalfExtent) {
        if self.nodes[node_idx as usize].is_leaf() {
            let count = self.nodes[node_idx as usize].count();
            if count >= self.split_threshold
                && depth < self.max_depth
                && half.w >= self.min_size
                && half.h >= self.min_size
            {
                self.split_node(node_idx, depth, half);
            }
        }

        if !self.nodes[node_idx as usize].is_leaf() {
            for i in 0..4 {
                let child = self.nodes[node_idx as usize].child(i);
                if child != 0 {
                    let child_half = Self::child_half_extent(half, i);
                    self.rebalance_node(child, depth + 1, child_half);
                }
            }

            let children = [
                self.nodes[node_idx as usize].child(0),
                self.nodes[node_idx as usize].child(1),
                self.nodes[node_idx as usize].child(2),
                self.nodes[node_idx as usize].child(3),
            ];
            if children
                .iter()
                .all(|&child| child != 0 && self.nodes[child as usize].is_leaf())
            {
                let mut total = 0u32;
                for &child in &children {
                    total += self.nodes[child as usize].count();
                }
                if total <= self.merge_threshold {
                    self.merge_node(node_idx, children, half);
                }
            }
        }
    }

    fn split_node(&mut self, node_idx: u32, _depth: u32, half: HalfExtent) {
        let position_flags = self.nodes[node_idx as usize].position_flags();
        let masks = [0b0011, 0b1001, 0b0110, 0b1100];
        let mut child_idxs = [0u32; 4];
        for i in 0..4 {
            let child_idx = self.alloc_node(position_flags & masks[i]);
            child_idxs[i] = child_idx;
        }

        let node = &mut self.nodes[node_idx as usize];
        let head = node.head();
        node.set_children(child_idxs);

        let mut current = head;
        while current != 0 {
            let next = self.node_entities_next[current as usize];
            let entity_idx = self.node_entities[current as usize].index();
            let extent = self.entities[entity_idx as usize].extent;
            let mut targets = [0u32; 4];
            let mut targets_len = 0usize;

            if extent.min_x <= half.x {
                if extent.min_y <= half.y {
                    targets[targets_len] = 0;
                    targets_len += 1;
                }
                if extent.max_y >= half.y {
                    targets[targets_len] = 1;
                    targets_len += 1;
                }
            }
            if extent.max_x >= half.x {
                if extent.min_y <= half.y {
                    targets[targets_len] = 2;
                    targets_len += 1;
                }
                if extent.max_y >= half.y {
                    targets[targets_len] = 3;
                    targets_len += 1;
                }
            }

            if targets_len == 0 {
                targets[0] = 0;
                targets_len = 1;
            }

            let entity = &mut self.entities[entity_idx as usize];
            entity.in_nodes_minus_one += targets_len as u32 - 1;

            for target in targets.iter().take(targets_len) {
                let child_idx = child_idxs[*target as usize];
                let child_head = self.nodes[child_idx as usize].head();
                let node_entity_idx = self.alloc_node_entity();

                self.node_entities_next[node_entity_idx as usize] = child_head;
                self.node_entities[node_entity_idx as usize].set_index(entity_idx);
                self.node_entities[node_entity_idx as usize].set_is_last(child_head == 0);
                self.node_entities_flags[node_entity_idx as usize] =
                    self.node_entities_flags[current as usize];
                let child = &mut self.nodes[child_idx as usize];
                child.set_head(node_entity_idx);
                child.set_count(child.count() + 1);
            }

            self.node_entities_next[current as usize] = self.free_node_entity;
            self.free_node_entity = current;
            current = next;
        }
    }

    fn merge_node(&mut self, node_idx: u32, children: [u32; 4], half: HalfExtent) {
        let mut merged_head = 0u32;
        let mut merged_count = 0u32;
        let mut position_flags = 0u8;
        let node_extent = half.to_rect_extent();

        self.merge_ht.fill(0);

        for &child_idx in &children {
            let child_position_flags = self.nodes[child_idx as usize].position_flags();
            position_flags |= child_position_flags;

            let mut current = self.nodes[child_idx as usize].head();
            while current != 0 {
                let next = self.node_entities_next[current as usize];
                let entity_idx = self.node_entities[current as usize].index();
                let mut hash = (entity_idx as usize * 2654435761) & (self.merge_ht.len() - 1);

                loop {
                    let entry = self.merge_ht[hash];
                    if entry == 0 {
                        self.merge_ht[hash] = entity_idx;
                        self.node_entities_next[current as usize] = merged_head;
                        self.node_entities[current as usize].set_index(entity_idx);
                        self.node_entities[current as usize].set_is_last(merged_head == 0);
                        let entity_extent = self.entities[entity_idx as usize].extent;
                        self.node_entities_flags[current as usize] =
                            Self::compute_node_entity_flags(
                                node_extent,
                                position_flags,
                                entity_extent,
                            );
                        merged_head = current;
                        merged_count += 1;
                        break;
                    }

                    if entry == entity_idx {
                        let entity = &mut self.entities[entity_idx as usize];
                        if entity.in_nodes_minus_one > 0 {
                            entity.in_nodes_minus_one -= 1;
                        }
                        self.node_entities_next[current as usize] = self.free_node_entity;
                        self.free_node_entity = current;
                        break;
                    }

                    hash = (hash + 1) & (self.merge_ht.len() - 1);
                }

                current = next;
            }

            let child = &mut self.nodes[child_idx as usize];
            child.set_free_next(self.free_node);
            self.free_node = child_idx;
        }

        let node = &mut self.nodes[node_idx as usize];
        node.reset_leaf(position_flags);
        node.set_head(merged_head);
        node.set_count(merged_count);
    }

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

        if filter_entity_types.is_none() {
            if all_rectangles && matches!(query_kind, QueryKind::Rect { .. }) {
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
        let circle_x = &self.circle_x;
        let circle_y = &self.circle_y;
        let circle_radius = &self.circle_radius;
        let circle_radius_sq = &self.circle_radius_sq;

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
                let entity = &mut entities[entity_idx as usize];
                if entity.query_tick == tick {
                    if node_entity.is_last() {
                        break;
                    }
                    current += 1;
                    continue;
                }
                entity.query_tick = tick;

                if let Some(filter) = filter_entity_types {
                    match entity.entity_type {
                        Some(entity_type) if filter.contains(entity_type) => {}
                        _ => {
                            if node_entity.is_last() {
                                break;
                            }
                            current += 1;
                            continue;
                        }
                    }
                }

                let hit = match query_kind {
                    QueryKind::Rect { rect } => {
                        if all_rectangles || entity.shape_kind == SHAPE_RECT {
                            entity.extent.intersects_strict(&query_extent)
                        } else {
                            let idx = entity_idx as usize;
                            circle_rect_raw(
                                circle_x[idx],
                                circle_y[idx],
                                circle_radius[idx],
                                circle_radius_sq[idx],
                                &rect,
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
                            circle_rect_raw(x, y, radius, radius_sq, &entity.bbox)
                        } else {
                            let idx = entity_idx as usize;
                            circle_circle_raw(
                                x,
                                y,
                                radius,
                                circle_x[idx],
                                circle_y[idx],
                                circle_radius[idx],
                            )
                        }
                    }
                };

                if hit {
                    f(entity.value);
                }

                if node_entity.is_last() {
                    break;
                }
                current += 1;
            }
        }

        self.query_stack = stack;
    }

    fn collisions_rect_fast_with<F>(
        &mut self,
        query_extent: RectExtent,
        tick: u32,
        f: &mut F,
    ) where
        F: FnMut(u32),
    {
        let nodes_ptr = self.nodes.as_ptr();
        let node_entities_ptr = self.node_entities.as_ptr();
        let entities_ptr = self.entities.as_mut_ptr();

        let mut stack = std::mem::take(&mut self.query_stack);
        stack.clear();
        stack.push((0u32, self.root_half));

        while let Some((node_idx, half)) = stack.pop() {
            let node = unsafe { &*nodes_ptr.add(node_idx as usize) };
            if !node.is_leaf() {
                Self::descend(&self.nodes, node_idx, half, query_extent, &mut stack);
                continue;
            }

            let mut current = node.head();
            if current == 0 {
                continue;
            }

            loop {
                let node_entity = unsafe { &*node_entities_ptr.add(current as usize) };
                let entity_idx = node_entity.index() as usize;
                let entity = unsafe { &mut *entities_ptr.add(entity_idx) };

                if entity.query_tick != tick {
                    entity.query_tick = tick;
                    if entity.extent.intersects_strict(&query_extent) {
                        f(entity.value);
                    }
                }

                if node_entity.is_last() {
                    break;
                }
                current += 1;
            }
        }

        self.query_stack = stack;
    }

    fn collisions_circle_fast_with<F>(&mut self, query: Query, tick: u32, f: &mut F)
    where
        F: FnMut(u32),
    {
        let query_extent = query.extent;
        let query_kind = query.kind;

        let mut stack = std::mem::take(&mut self.query_stack);
        stack.clear();
        stack.push((0u32, self.root_half));

        let nodes = &self.nodes;
        let node_entities = &self.node_entities;
        let entities = &mut self.entities;
        let circle_x = &self.circle_x;
        let circle_y = &self.circle_y;
        let circle_radius = &self.circle_radius;
        let circle_radius_sq = &self.circle_radius_sq;

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
                let entity_idx = node_entity.index() as usize;
                let entity = &mut entities[entity_idx];

                if entity.query_tick != tick {
                    entity.query_tick = tick;
                    let hit = match query_kind {
                        QueryKind::Rect { rect } => circle_rect_raw(
                            circle_x[entity_idx],
                            circle_y[entity_idx],
                            circle_radius[entity_idx],
                            circle_radius_sq[entity_idx],
                            &rect,
                        ),
                        QueryKind::Circle {
                            x,
                            y,
                            radius,
                            radius_sq: _,
                        } => circle_circle_raw(
                            x,
                            y,
                            radius,
                            circle_x[entity_idx],
                            circle_y[entity_idx],
                            circle_radius[entity_idx],
                        ),
                    };

                    if hit {
                        f(entity.value);
                    }
                }

                if node_entity.is_last() {
                    break;
                }
                current += 1;
            }
        }

        self.query_stack = stack;
    }

    fn next_query_tick(&mut self) -> u32 {
        self.query_tick = self.query_tick.wrapping_add(1);
        if self.query_tick == 0 {
            self.query_tick = 1;
            for entity in &mut self.entities {
                entity.query_tick = 0;
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
        let circle_x = &self.circle_x;
        let circle_y = &self.circle_y;
        let circle_radius = &self.circle_radius;
        let circle_radius_sq = &self.circle_radius_sq;
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
            let a_extent = a.extent;
            let a_is_circle = a.shape_kind == SHAPE_CIRCLE;

            let mut other_idx = idx + 1;
            loop {
                let other_node_entity = node_entities[other_idx];
                let b_idx = other_node_entity.index();
                let b_idx_usize = b_idx as usize;
                let b = &entities[b_idx_usize];
                let b_is_circle = b.shape_kind == SHAPE_CIRCLE;

                let hit = if !a_is_circle && !b_is_circle {
                    a_extent.intersects_strict(&b.extent)
                } else if a_is_circle && b_is_circle {
                    circle_circle_raw(
                        circle_x[a_idx_usize],
                        circle_y[a_idx_usize],
                        circle_radius[a_idx_usize],
                        circle_x[b_idx_usize],
                        circle_y[b_idx_usize],
                        circle_radius[b_idx_usize],
                    )
                } else if a_is_circle {
                    circle_rect_raw(
                        circle_x[a_idx_usize],
                        circle_y[a_idx_usize],
                        circle_radius[a_idx_usize],
                        circle_radius_sq[a_idx_usize],
                        &b.bbox,
                    )
                } else {
                    circle_rect_raw(
                        circle_x[b_idx_usize],
                        circle_y[b_idx_usize],
                        circle_radius[b_idx_usize],
                        circle_radius_sq[b_idx_usize],
                        &a.bbox,
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

                    f(a.value, b.value);
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
        let entities_ptr = self.entities.as_ptr();
        let node_entities_len = self.node_entities.len();

        let mut idx = 1usize;
        while idx < node_entities_len {
            let node_entity = unsafe { &*node_entities_ptr.add(idx) };
            if node_entity.is_last() {
                idx += 1;
                continue;
            }

            let a_idx = node_entity.index();
            let a = unsafe { &*entities_ptr.add(a_idx as usize) };
            let a_extent = a.extent;

            let mut other_idx = idx + 1;
            loop {
                let other_node_entity = unsafe { &*node_entities_ptr.add(other_idx) };
                let b_idx = other_node_entity.index();
                let b = unsafe { &*entities_ptr.add(b_idx as usize) };

                if a_extent.intersects_strict(&b.extent) {
                    let needs_dedupe = a.in_nodes_minus_one > 0 || b.in_nodes_minus_one > 0;
                    if needs_dedupe {
                        let (min, max) = if a_idx < b_idx {
                            (a_idx, b_idx)
                        } else {
                            (b_idx, a_idx)
                        };
                        let key = (u64::from(min) << 32) | u64::from(max);
                        if !self.pair_dedupe.insert(key) {
                            if other_node_entity.is_last() {
                                break;
                            }
                            other_idx += 1;
                            continue;
                        }
                    }

                    f(a.value, b.value);
                }

                if other_node_entity.is_last() {
                    break;
                }
                other_idx += 1;
            }

            idx += 1;
        }
    }

    fn for_each_collision_pair_circle_fast<F>(&mut self, f: &mut F)
    where
        F: FnMut(u32, u32),
    {
        let node_entities = &self.node_entities;
        let entities = &self.entities;
        let circle_x = &self.circle_x;
        let circle_y = &self.circle_y;
        let circle_radius = &self.circle_radius;
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

            let mut other_idx = idx + 1;
            loop {
                let other_node_entity = node_entities[other_idx];
                let b_idx = other_node_entity.index();
                let b_idx_usize = b_idx as usize;
                let b = &entities[b_idx_usize];

                let hit = circle_circle_raw(
                    circle_x[a_idx_usize],
                    circle_y[a_idx_usize],
                    circle_radius[a_idx_usize],
                    circle_x[b_idx_usize],
                    circle_y[b_idx_usize],
                    circle_radius[b_idx_usize],
                );
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

                    f(a.value, b.value);
                }

                if other_node_entity.is_last() {
                    break;
                }
                other_idx += 1;
            }

            idx += 1;
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
        for (idx, entity) in self.entities.iter().enumerate() {
            if entity.alive {
                shapes.push(self.entity_shapes[idx].clone());
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

    pub fn delete(&mut self, value: u32) {
        self.inner.get_mut().delete(value);
    }

    pub fn relocate_batch(&mut self, relocation_requests: Vec<RelocationRequest>) {
        self.inner.get_mut().relocate_batch(relocation_requests);
    }

    pub fn relocate(&mut self, value: u32, shape: ShapeEnum, entity_type: Option<u32>) {
        self.inner.get_mut().relocate(value, shape, entity_type);
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
}

impl Default for Config {
    fn default() -> Self {
        Config {
            pool_size: 4000,
            node_capacity: 4,
            max_depth: 6,
        }
    }
}

#[derive(Clone)]
pub struct RelocationRequest {
    pub value: u32,
    pub shape: ShapeEnum,
    pub entity_type: Option<u32>,
}
