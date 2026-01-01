use crate::collision_detection;
use common::shapes::{Rectangle, Shape, ShapeEnum};
use fxhash::{FxHashMap, FxHashSet};
use std::cell::{Cell, RefCell};

fn rect_contains(outer: &Rectangle, inner: &Rectangle) -> bool {
    outer.left() <= inner.left()
        && outer.right() >= inner.right()
        && outer.top() <= inner.top()
        && outer.bottom() >= inner.bottom()
}

#[derive(Clone, Copy)]
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
}

fn extent_intersects(a: &RectExtent, b: &RectExtent) -> bool {
    a.min_x < b.max_x && a.max_x > b.min_x && a.min_y < b.max_y && a.max_y > b.min_y
}

fn extent_intersects_inclusive(a: &RectExtent, b: &RectExtent) -> bool {
    a.min_x <= b.max_x && a.max_x >= b.min_x && a.min_y <= b.max_y && a.max_y >= b.min_y
}

struct Entity {
    value: u32,
    shape: ShapeEnum,
    entity_type: Option<u32>,
    bbox: Rectangle,
    extent: RectExtent,
    nodes: Vec<usize>,
    in_overflow: bool,
    overflow_index: Option<usize>,
    query_tick: Cell<u32>,
    alive: bool,
    cover_bbox: Rectangle,
    cover_valid: bool,
}

impl Entity {
    fn new(value: u32, shape: ShapeEnum, entity_type: Option<u32>) -> Self {
        let bbox = shape.bounding_box();
        let extent = RectExtent::from_rect(&bbox);
        Self {
            value,
            shape,
            entity_type,
            bbox,
            extent,
            nodes: Vec::new(),
            in_overflow: false,
            overflow_index: None,
            query_tick: Cell::new(0),
            alive: true,
            cover_bbox: Rectangle::default(),
            cover_valid: false,
        }
    }

    fn reset(&mut self, value: u32, shape: ShapeEnum, entity_type: Option<u32>) {
        self.value = value;
        self.shape = shape;
        self.entity_type = entity_type;
        self.bbox = self.shape.bounding_box();
        self.extent = RectExtent::from_rect(&self.bbox);
        self.nodes.clear();
        self.in_overflow = false;
        self.overflow_index = None;
        self.query_tick.set(0);
        self.alive = true;
        self.cover_bbox = Rectangle::default();
        self.cover_valid = false;
    }
}

struct QuadNode {
    entities: Vec<usize>,
    bounding_box: Rectangle,
    extent: RectExtent,
    children: [Option<usize>; 4],
    subdivided: bool,
    depth: usize,
}

impl QuadNode {
    fn new(bounding_box: Rectangle, depth: usize, capacity: usize) -> Self {
        let extent = RectExtent::from_rect(&bounding_box);
        Self {
            entities: Vec::with_capacity(capacity),
            bounding_box,
            extent,
            children: [None; 4],
            subdivided: false,
            depth,
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
    root: usize,
    nodes: Vec<QuadNode>,
    free_nodes: Vec<usize>,
    entities: Vec<Entity>,
    free_entities: Vec<usize>,
    owner_map: FxHashMap<u32, usize>,
    dense_owner: Vec<usize>,
    overflow: Vec<usize>,
    config: Config,
    query_tick: Cell<u32>,
    pair_dedupe: RefCell<PairDedupe>,
    scratch_stack: RefCell<Vec<usize>>,
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

impl QuadTree {
    const DENSE_OWNER_LIMIT: usize = 1_000_000;

    pub fn new_with_config(bounding_box: Rectangle, config: Config) -> Self {
        let mut nodes = Vec::with_capacity(config.pool_size.max(1));
        nodes.push(QuadNode::new(bounding_box, 0, config.node_capacity));
        Self {
            root: 0,
            nodes,
            free_nodes: Vec::new(),
            entities: Vec::new(),
            free_entities: Vec::new(),
            owner_map: FxHashMap::default(),
            dense_owner: Vec::new(),
            overflow: Vec::new(),
            config,
            query_tick: Cell::new(0),
            pair_dedupe: RefCell::new(PairDedupe::new()),
            scratch_stack: RefCell::new(Vec::new()),
        }
    }

    pub fn new(bounding_box: Rectangle) -> Self {
        Self::new_with_config(bounding_box, Config::default())
    }

    fn alloc_node(&mut self, bounding_box: Rectangle, depth: usize) -> usize {
        if let Some(idx) = self.free_nodes.pop() {
            self.nodes[idx] = QuadNode::new(bounding_box, depth, self.config.node_capacity);
            idx
        } else {
            self.nodes
                .push(QuadNode::new(bounding_box, depth, self.config.node_capacity));
            self.nodes.len() - 1
        }
    }

    fn alloc_entity(&mut self, value: u32, shape: ShapeEnum, entity_type: Option<u32>) -> usize {
        if let Some(idx) = self.free_entities.pop() {
            self.entities[idx].reset(value, shape, entity_type);
            idx
        } else {
            self.entities.push(Entity::new(value, shape, entity_type));
            self.entities.len() - 1
        }
    }

    fn owner_lookup(&self, value: u32) -> Option<usize> {
        let idx = value as usize;
        if idx < self.dense_owner.len() {
            let stored = self.dense_owner[idx];
            if stored != usize::MAX {
                return Some(stored);
            }
        }
        self.owner_map.get(&value).copied()
    }

    fn owner_insert(&mut self, value: u32, entity_idx: usize) {
        let idx = value as usize;
        if idx <= Self::DENSE_OWNER_LIMIT {
            if idx >= self.dense_owner.len() {
                self.dense_owner.resize(idx + 1, usize::MAX);
            }
            self.dense_owner[idx] = entity_idx;
        } else {
            self.owner_map.insert(value, entity_idx);
        }
    }

    fn owner_remove(&mut self, value: u32) -> Option<usize> {
        let idx = value as usize;
        if idx < self.dense_owner.len() {
            let stored = self.dense_owner[idx];
            if stored != usize::MAX {
                self.dense_owner[idx] = usize::MAX;
                return Some(stored);
            }
        }
        self.owner_map.remove(&value)
    }

    pub fn insert(&mut self, value: u32, shape: ShapeEnum, entity_type: Option<u32>) {
        if self.owner_lookup(value).is_some() {
            self.relocate(value, shape, entity_type);
            return;
        }

        let entity_idx = self.alloc_entity(value, shape, entity_type);
        self.owner_insert(value, entity_idx);
        self.insert_entity(entity_idx);
    }

    fn insert_entity(&mut self, entity_idx: usize) {
        let extent = self.entities[entity_idx].extent;
        if !extent_intersects_inclusive(&self.nodes[self.root].extent, &extent) {
            self.add_to_overflow(entity_idx);
            return;
        }

        self.insert_into(self.root, entity_idx);
    }

    fn insert_into(&mut self, node: usize, entity_idx: usize) {
        if self.nodes[node].subdivided {
            let extent = self.entities[entity_idx].extent;
            let children = self.nodes[node].children;
            for child in children.iter().flatten() {
                let child_idx = *child;
                if extent_intersects_inclusive(&self.nodes[child_idx].extent, &extent) {
                    self.insert_into(child_idx, entity_idx);
                }
            }
            return;
        }

        self.add_entity_to_node(node, entity_idx);
        if self.nodes[node].entities.len() >= self.config.node_capacity
            && self.nodes[node].depth < self.config.max_depth
        {
            self.subdivide(node);
        }
    }

    fn add_entity_to_node(&mut self, node: usize, entity_idx: usize) {
        self.nodes[node].entities.push(entity_idx);
        self.entities[entity_idx].nodes.push(node);
        let node_bbox = self.nodes[node].bounding_box;
        let entity = &mut self.entities[entity_idx];
        if entity.cover_valid {
            entity.cover_bbox.expand_to_include(&node_bbox);
        } else {
            entity.cover_bbox = node_bbox;
            entity.cover_valid = true;
        }
    }

    fn subdivide(&mut self, node: usize) {
        if self.nodes[node].subdivided {
            return;
        }

        let (bounding_box, depth) = {
            let node_ref = &self.nodes[node];
            (node_ref.bounding_box, node_ref.depth)
        };

        let half_width = bounding_box.width / 2.0;
        let half_height = bounding_box.height / 2.0;

        let center_x = bounding_box.x;
        let center_y = bounding_box.y;
        let west_x = center_x - half_width / 2.0;
        let east_x = center_x + half_width / 2.0;
        let north_y = center_y + half_height / 2.0;
        let south_y = center_y - half_height / 2.0;

        let nw = self.alloc_node(
            Rectangle {
                x: west_x,
                y: north_y,
                width: half_width,
                height: half_height,
            },
            depth + 1,
        );
        let ne = self.alloc_node(
            Rectangle {
                x: east_x,
                y: north_y,
                width: half_width,
                height: half_height,
            },
            depth + 1,
        );
        let sw = self.alloc_node(
            Rectangle {
                x: west_x,
                y: south_y,
                width: half_width,
                height: half_height,
            },
            depth + 1,
        );
        let se = self.alloc_node(
            Rectangle {
                x: east_x,
                y: south_y,
                width: half_width,
                height: half_height,
            },
            depth + 1,
        );

        {
            let node_ref = &mut self.nodes[node];
            node_ref.children = [Some(nw), Some(ne), Some(sw), Some(se)];
            node_ref.subdivided = true;
        }

        let old_entities = std::mem::take(&mut self.nodes[node].entities);
        for entity_idx in old_entities {
            self.remove_node_from_entity(entity_idx, node);
            self.insert_into(node, entity_idx);
        }
    }

    fn remove_entity_from_node(&mut self, node: usize, entity_idx: usize) -> bool {
        if let Some(pos) = self.nodes[node]
            .entities
            .iter()
            .position(|&idx| idx == entity_idx)
        {
            self.nodes[node].entities.swap_remove(pos);
            true
        } else {
            false
        }
    }

    fn remove_node_from_entity(&mut self, entity_idx: usize, node: usize) -> bool {
        if let Some(pos) = self.entities[entity_idx]
            .nodes
            .iter()
            .position(|&idx| idx == node)
        {
            self.entities[entity_idx].nodes.swap_remove(pos);
            true
        } else {
            false
        }
    }

    fn remove_entity_from_nodes(&mut self, entity_idx: usize) {
        let nodes = std::mem::take(&mut self.entities[entity_idx].nodes);
        for node in nodes {
            let _ = self.remove_entity_from_node(node, entity_idx);
        }
        self.entities[entity_idx].cover_valid = false;
    }

    fn add_to_overflow(&mut self, entity_idx: usize) {
        let index = self.overflow.len();
        self.overflow.push(entity_idx);
        let entity = &mut self.entities[entity_idx];
        entity.in_overflow = true;
        entity.overflow_index = Some(index);
    }

    fn remove_from_overflow(&mut self, entity_idx: usize) {
        let index = match self.entities[entity_idx].overflow_index.take() {
            Some(index) => index,
            None => return,
        };
        let actual_index = if index < self.overflow.len() && self.overflow[index] == entity_idx {
            index
        } else {
            match self.overflow.iter().position(|&idx| idx == entity_idx) {
                Some(found) => found,
                None => {
                    self.entities[entity_idx].in_overflow = false;
                    return;
                }
            }
        };

        let last = self.overflow.swap_remove(actual_index);
        if last != entity_idx {
            self.entities[last].overflow_index = Some(actual_index);
        }
        self.entities[entity_idx].in_overflow = false;
    }

    pub fn delete(&mut self, value: u32) {
        let entity_idx = match self.owner_remove(value) {
            Some(idx) => idx,
            None => return,
        };

        if self.entities[entity_idx].in_overflow {
            self.remove_from_overflow(entity_idx);
        } else {
            self.remove_entity_from_nodes(entity_idx);
        }

        let entity = &mut self.entities[entity_idx];
        entity.nodes.clear();
        entity.in_overflow = false;
        entity.overflow_index = None;
        entity.alive = false;
        self.free_entities.push(entity_idx);
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

        let new_bbox = shape.bounding_box();
        let within_cover = {
            let entity = &self.entities[entity_idx];
            entity.cover_valid && rect_contains(&entity.cover_bbox, &new_bbox)
        };

        if self.entities[entity_idx].in_overflow {
            self.remove_from_overflow(entity_idx);
            let entity = &mut self.entities[entity_idx];
            entity.shape = shape;
            entity.entity_type = entity_type;
            entity.bbox = new_bbox;
            entity.extent = RectExtent::from_rect(&new_bbox);
            entity.nodes.clear();
            entity.cover_valid = false;
            entity.in_overflow = false;
            entity.overflow_index = None;
            self.insert_entity(entity_idx);
            return;
        }

        if within_cover {
            {
                let entity = &mut self.entities[entity_idx];
                entity.shape = shape;
                entity.entity_type = entity_type;
                entity.bbox = new_bbox;
                entity.extent = RectExtent::from_rect(&new_bbox);
            }

            let mut index = 0;
            while index < self.entities[entity_idx].nodes.len() {
                let node = self.entities[entity_idx].nodes[index];
                let node_extent = self.nodes[node].extent;
                if extent_intersects_inclusive(&node_extent, &self.entities[entity_idx].extent) {
                    index += 1;
                } else {
                    let _ = self.remove_entity_from_node(node, entity_idx);
                    let _ = self.entities[entity_idx].nodes.swap_remove(index);
                }
            }

            self.recompute_cover_bbox(entity_idx);
            return;
        }

        self.remove_entity_from_nodes(entity_idx);

        let entity = &mut self.entities[entity_idx];
        entity.shape = shape;
        entity.entity_type = entity_type;
        entity.bbox = new_bbox;
        entity.extent = RectExtent::from_rect(&new_bbox);
        entity.nodes.clear();
        entity.in_overflow = false;
        entity.overflow_index = None;

        self.insert_entity(entity_idx);
    }

    fn next_query_tick(&self) -> u32 {
        let mut tick = self.query_tick.get().wrapping_add(1);
        if tick == 0 {
            self.query_tick.set(1);
            for entity in &self.entities {
                if entity.alive {
                    entity.query_tick.set(0);
                }
            }
            tick = 1;
        } else {
            self.query_tick.set(tick);
        }
        tick
    }

    pub fn collisions_batch(&self, shapes: Vec<ShapeEnum>) -> Vec<Vec<u32>> {
        shapes
            .into_iter()
            .map(|shape| {
                let mut collisions = Vec::new();
                self.collisions(shape, &mut collisions);
                collisions
            })
            .collect()
    }

    pub fn collisions_batch_filter(
        &self,
        shapes: Vec<ShapeEnum>,
        filter_entity_types: Option<Vec<u32>>,
    ) -> Vec<Vec<u32>> {
        let filter = filter_entity_types.map(EntityTypeFilter::from_vec);
        shapes
            .into_iter()
            .map(|shape| {
                let mut collisions = Vec::new();
                self.collisions_from(&shape, filter.as_ref(), &mut collisions);
                collisions
            })
            .collect()
    }

    pub fn collisions(&self, shape: ShapeEnum, collisions: &mut Vec<u32>) {
        self.collisions_from(&shape, None, collisions);
    }

    pub fn collisions_filter(
        &self,
        shape: ShapeEnum,
        filter_entity_types: Option<Vec<u32>>,
        collisions: &mut Vec<u32>,
    ) {
        let filter = filter_entity_types.map(EntityTypeFilter::from_vec);
        self.collisions_from(&shape, filter.as_ref(), collisions);
    }

    fn collisions_from(
        &self,
        query_shape: &ShapeEnum,
        filter_entity_types: Option<&EntityTypeFilter>,
        collisions: &mut Vec<u32>,
    ) {
        let query_bbox = query_shape.bounding_box();
        let query_bbox_extent = RectExtent::from_rect(&query_bbox);
        let query_extent = match query_shape {
            ShapeEnum::Rectangle(rect) => Some(RectExtent::from_rect(rect)),
            ShapeEnum::Circle(_) => None,
        };
        let tick = self.next_query_tick();

        for &entity_idx in &self.overflow {
            let entity = &self.entities[entity_idx];
            if entity.query_tick.get() == tick {
                continue;
            }
            entity.query_tick.set(tick);
            if let Some(filter) = filter_entity_types {
                match entity.entity_type {
                    Some(entity_type) if filter.contains(entity_type) => {}
                    _ => continue,
                }
            }

            let hit = match (query_shape, &entity.shape) {
                (ShapeEnum::Rectangle(_), ShapeEnum::Rectangle(_)) => {
                    extent_intersects(&entity.extent, query_extent.as_ref().unwrap())
                }
                (ShapeEnum::Rectangle(rect), ShapeEnum::Circle(circle)) => {
                    collision_detection::circle_rectangle(circle, rect)
                }
                (ShapeEnum::Circle(circle), ShapeEnum::Rectangle(_)) => {
                    collision_detection::circle_rectangle(circle, &entity.bbox)
                }
                (ShapeEnum::Circle(circle_a), ShapeEnum::Circle(circle_b)) => {
                    collision_detection::circle_circle(circle_a, circle_b)
                }
            };

            if hit {
                collisions.push(entity.value);
            }
        }

        if !extent_intersects_inclusive(&self.nodes[self.root].extent, &query_bbox_extent)
        {
            return;
        }

        let mut stack = self.scratch_stack.borrow_mut();
        stack.clear();
        stack.push(self.root);

        while let Some(node_idx) = stack.pop() {
            let node = &self.nodes[node_idx];
            if node.subdivided {
                for child in node.children.iter().flatten() {
                    let child_idx = *child;
                    if extent_intersects_inclusive(&self.nodes[child_idx].extent, &query_bbox_extent)
                    {
                        stack.push(child_idx);
                    }
                }
                continue;
            }

            for &entity_idx in &node.entities {
                let entity = &self.entities[entity_idx];
                if entity.query_tick.get() == tick {
                    continue;
                }
                entity.query_tick.set(tick);
                if let Some(filter) = filter_entity_types {
                    match entity.entity_type {
                        Some(entity_type) if filter.contains(entity_type) => {}
                        _ => continue,
                    }
                }

                let hit = match (query_shape, &entity.shape) {
                    (ShapeEnum::Rectangle(_), ShapeEnum::Rectangle(_)) => {
                        extent_intersects(&entity.extent, query_extent.as_ref().unwrap())
                    }
                    (ShapeEnum::Rectangle(rect), ShapeEnum::Circle(circle)) => {
                        collision_detection::circle_rectangle(circle, rect)
                    }
                    (ShapeEnum::Circle(circle), ShapeEnum::Rectangle(_)) => {
                        collision_detection::circle_rectangle(circle, &entity.bbox)
                    }
                    (ShapeEnum::Circle(circle_a), ShapeEnum::Circle(circle_b)) => {
                        collision_detection::circle_circle(circle_a, circle_b)
                    }
                };

                if hit {
                    collisions.push(entity.value);
                }
            }
        }
    }

    pub fn for_each_collision_pair<F>(&self, mut f: F)
    where
        F: FnMut(u32, u32),
    {
        let mut dedupe = self.pair_dedupe.borrow_mut();
        dedupe.ensure_capacity(self.entities.len().saturating_mul(2).max(1));
        dedupe.clear();

        let mut stack = Vec::with_capacity(32);
        stack.push(self.root);

        while let Some(node_idx) = stack.pop() {
            let node = &self.nodes[node_idx];
            if node.subdivided {
                for child in node.children.iter().flatten() {
                    stack.push(*child);
                }
                continue;
            }

            let entities = &node.entities;
            for i in 0..entities.len() {
                let a_idx = entities[i];
                let a = &self.entities[a_idx];
                for j in (i + 1)..entities.len() {
                    let b_idx = entities[j];
                    let b = &self.entities[b_idx];
                    let hit = match (&a.shape, &b.shape) {
                        (ShapeEnum::Rectangle(_), ShapeEnum::Rectangle(_)) => {
                            extent_intersects(&a.extent, &b.extent)
                        }
                        _ => collision_detection::shape_shape(&a.shape, &b.shape),
                    };
                    if !hit {
                        continue;
                    }

                    let needs_dedupe = a.nodes.len() > 1 || b.nodes.len() > 1;
                    if needs_dedupe {
                        let (min, max) = if a.value < b.value {
                            (a.value, b.value)
                        } else {
                            (b.value, a.value)
                        };
                        let key = (u64::from(min) << 32) | u64::from(max);
                        if !dedupe.insert(key) {
                            continue;
                        }
                    }

                    f(a.value, b.value);
                }
            }

            if !self.overflow.is_empty() {
                for &overflow_idx in &self.overflow {
                    let overflow_entity = &self.entities[overflow_idx];
                    for &entity_idx in entities {
                        if overflow_idx == entity_idx {
                            continue;
                        }
                        let entity = &self.entities[entity_idx];
                        let hit = match (&overflow_entity.shape, &entity.shape) {
                            (ShapeEnum::Rectangle(_), ShapeEnum::Rectangle(_)) => {
                                extent_intersects(&overflow_entity.extent, &entity.extent)
                            }
                            _ => collision_detection::shape_shape(
                                &overflow_entity.shape,
                                &entity.shape,
                            ),
                        };
                        if !hit {
                            continue;
                        }

                        let needs_dedupe = entity.nodes.len() > 1;
                        if needs_dedupe {
                            let (min, max) = if overflow_entity.value < entity.value {
                                (overflow_entity.value, entity.value)
                            } else {
                                (entity.value, overflow_entity.value)
                            };
                            let key = (u64::from(min) << 32) | u64::from(max);
                            if !dedupe.insert(key) {
                                continue;
                            }
                        }

                        f(overflow_entity.value, entity.value);
                    }
                }
            }
        }

        if self.overflow.len() > 1 {
            for i in 0..self.overflow.len() {
                let a_idx = self.overflow[i];
                let a = &self.entities[a_idx];
                for j in (i + 1)..self.overflow.len() {
                    let b_idx = self.overflow[j];
                    let b = &self.entities[b_idx];
                    let hit = match (&a.shape, &b.shape) {
                        (ShapeEnum::Rectangle(_), ShapeEnum::Rectangle(_)) => {
                            extent_intersects(&a.extent, &b.extent)
                        }
                        _ => collision_detection::shape_shape(&a.shape, &b.shape),
                    };
                    if hit {
                        f(a.value, b.value);
                    }
                }
            }
        }
    }

    pub fn all_node_bounding_boxes(&self, bounding_boxes: &mut Vec<Rectangle>) {
        let mut stack = Vec::with_capacity(32);
        stack.push(self.root);

        while let Some(node) = stack.pop() {
            let node_ref = &self.nodes[node];
            bounding_boxes.push(node_ref.bounding_box);

            if node_ref.subdivided {
                for child in node_ref.children.iter().flatten() {
                    stack.push(*child);
                }
            }
        }
    }

    pub fn all_shapes(&self, shapes: &mut Vec<ShapeEnum>) {
        for entity in &self.entities {
            if entity.alive {
                shapes.push(entity.shape.clone());
            }
        }
    }

    fn recompute_cover_bbox(&mut self, entity_idx: usize) {
        let nodes = &self.entities[entity_idx].nodes;
        if nodes.is_empty() {
            self.entities[entity_idx].cover_bbox = Rectangle::default();
            self.entities[entity_idx].cover_valid = false;
            return;
        }

        let mut cover = self.nodes[nodes[0]].bounding_box;
        for &node in nodes.iter().skip(1) {
            cover.expand_to_include(&self.nodes[node].bounding_box);
        }
        self.entities[entity_idx].cover_bbox = cover;
        self.entities[entity_idx].cover_valid = true;
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
            // With a max depth of 6, there could be up to 5461 nodes.
            // Let's set a reasonable max pool size of 4000.
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
