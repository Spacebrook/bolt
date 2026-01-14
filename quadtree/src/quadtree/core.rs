use super::*;
use common::shapes::{Rectangle, ShapeEnum};
use fxhash::FxHashMap;

impl QuadTreeInner {
    const DENSE_OWNER_LIMIT: usize = 1_000_000;
    #[allow(dead_code)]
    #[inline(always)]
    fn bump_query_calls_ptr(stats: *mut QueryStats) {
        #[cfg(feature = "query_stats")]
        unsafe {
            (*stats).query_calls += 1;
        }
        #[cfg(not(feature = "query_stats"))]
        {
            let _ = stats;
        }
    }
    #[allow(dead_code)]
    #[inline(always)]
    fn bump_query_node_ptr(stats: *mut QueryStats) {
        #[cfg(feature = "query_stats")]
        unsafe {
            (*stats).node_visits += 1;
        }
        #[cfg(not(feature = "query_stats"))]
        {
            let _ = stats;
        }
    }
    #[allow(dead_code)]
    #[inline(always)]
    fn bump_query_entity_ptr(stats: *mut QueryStats) {
        #[cfg(feature = "query_stats")]
        unsafe {
            (*stats).entity_visits += 1;
        }
        #[cfg(not(feature = "query_stats"))]
        {
            let _ = stats;
        }
    }
    pub fn new_with_config(bounding_box: Rectangle, config: Config) -> Self {
        let root_extent = RectExtent::from_rect(&bounding_box);
        let root_half = HalfExtent::from_rect_extent(root_extent);
        let split_threshold = config.node_capacity.max(1) as u32;
        let merge_threshold = split_threshold.saturating_sub(1).max(1);
        let max_depth = config.max_depth as u32;
        let min_size = if config.min_size > 0.0 {
            config.min_size
        } else {
            1.0
        };
        let looseness = if config.looseness >= 1.0 {
            config.looseness
        } else {
            1.0
        };
        let merge_ht_size = (merge_threshold * 2).next_power_of_two().max(8) as usize;
        let large_entity_threshold = if config.large_entity_threshold_factor > 0.0 {
            let root_size = root_half.w.max(root_half.h) * 2.0;
            root_size * config.large_entity_threshold_factor
        } else {
            0.0
        };
        let mut nodes = Vec::new();
        nodes.push(Node::new_leaf(
            FLAG_LEFT | FLAG_RIGHT | FLAG_TOP | FLAG_BOTTOM,
        ));
        let nodes_scratch = Vec::new();
        let mut node_centers = NodeCentersSoa::new();
        node_centers.push(root_half.x, root_half.y);
        let node_centers_scratch = NodeCentersSoa::new();
        let mut node_extents_tight = ExtentAos::default();
        node_extents_tight.push(root_half.to_rect_extent());
        let node_extents_tight_scratch = ExtentAos::default();
        let mut node_extents_loose = ExtentAos::default();
        node_extents_loose.push(loose_extent_from_half(root_half, looseness));
        let node_extents_loose_scratch = ExtentAos::default();
        let mut node_entities = Vec::new();
        node_entities.push(NodeEntity::new(0));
        let node_entities_scratch = Vec::new();
        let mut node_entity_extents = NodeEntityExtentsSoa::default();
        node_entity_extents.push(RectExtent::from_min_max(0.0, 0.0, 0.0, 0.0));
        let node_entity_extents_scratch = NodeEntityExtentsSoa::default();
        let mut node_entity_packed = Vec::new();
        node_entity_packed.push(NodeEntityPacked::default());
        let node_entity_packed_scratch = Vec::new();
        let mut node_entities_next = Vec::new();
        node_entities_next.push(0);
        let node_entities_next_scratch = Vec::new();
        let mut node_entity_values = Vec::new();
        node_entity_values.push(0);
        let node_entity_values_scratch = Vec::new();
        let mut node_entities_flags = Vec::new();
        node_entities_flags.push(0);
        let node_entities_flags_scratch = Vec::new();
        let mut node_entities_last = Vec::new();
        node_entities_last.push(0);
        let node_entities_last_scratch = Vec::new();
        let mut entities = Vec::new();
        entities.push(Entity::sentinel());
        let entities_scratch = Vec::new();
        let mut entity_extents = ExtentAos::default();
        entity_extents.push(RectExtent::from_min_max(0.0, 0.0, 0.0, 0.0));
        let entity_extents_scratch = ExtentAos::default();
        let mut query_marks = Vec::new();
        query_marks.push(0);
        let query_marks_scratch = Vec::new();
        let mut entity_values = Vec::new();
        entity_values.push(0);
        let entity_values_scratch = Vec::new();
        let entity_types = None;
        let entity_types_scratch = None;
        let circle_data = None;
        let circle_data_scratch = None;
        let reserve = config.pool_size.saturating_add(1);
        if reserve > 1 {
            if node_entities.len() < reserve {
                node_entities.reserve(reserve - node_entities.len());
            }
            if node_entity_extents.len() < reserve {
                node_entity_extents.reserve(reserve - node_entity_extents.len());
            }
            if node_entity_packed.len() < reserve {
                node_entity_packed.reserve(reserve - node_entity_packed.len());
            }
            if node_entities_next.len() < reserve {
                node_entities_next.reserve(reserve - node_entities_next.len());
            }
            if node_entity_values.len() < reserve {
                node_entity_values.reserve(reserve - node_entity_values.len());
            }
            if node_entities_flags.len() < reserve {
                node_entities_flags.reserve(reserve - node_entities_flags.len());
            }
            if node_entities_last.len() < reserve {
                node_entities_last.reserve(reserve - node_entities_last.len());
            }
            if entities.len() < reserve {
                entities.reserve(reserve - entities.len());
            }
            if entity_extents.len() < reserve {
                entity_extents.reserve(reserve - entity_extents.len());
            }
            if query_marks.len() < reserve {
                query_marks.reserve(reserve - query_marks.len());
            }
            if entity_values.len() < reserve {
                entity_values.reserve(reserve - entity_values.len());
            }
        }
        let rebuild_stack =
            Vec::with_capacity((max_depth as usize).saturating_mul(3).saturating_add(1));
        let profile_summary = config.profile_summary;
        let profile_detail = config.profile_detail;
        let use_avx2 = {
            #[cfg(target_arch = "x86_64")]
            {
                std::arch::is_x86_feature_detected!("avx2")
            }
            #[cfg(not(target_arch = "x86_64"))]
            {
                false
            }
        };
        let profile_remaining = if profile_summary || profile_detail {
            config.profile_limit.max(1)
        } else {
            0
        };
        Self {
            root_half,
            nodes,
            nodes_scratch,
            node_centers,
            node_centers_scratch,
            node_extents_tight,
            node_extents_tight_scratch,
            node_extents_loose,
            node_extents_loose_scratch,
            free_node: 0,
            node_entities,
            node_entities_scratch,
            node_entity_extents,
            node_entity_extents_scratch,
            node_entity_packed,
            node_entity_packed_scratch,
            node_entities_next,
            node_entities_next_scratch,
            node_entity_values,
            node_entity_values_scratch,
            node_entities_flags,
            node_entities_flags_scratch,
            node_entities_last,
            node_entities_last_scratch,
            free_node_entity: 0,
            entities,
            entities_scratch,
            entity_extents,
            entity_extents_scratch,
            query_marks,
            query_marks_scratch,
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
            query_stats: QueryStats::default(),
            profile_remaining,
            profile_summary,
            profile_detail,
            reorder_counter: 0,
            allow_duplicates: false,
            update_pending: false,
            use_avx2,
            large_entity_threshold,
            large_entities: Vec::new(),
            large_entity_slots: vec![0],
            split_threshold,
            merge_threshold,
            max_depth,
            min_size,
            looseness,
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
                (max_depth as usize).saturating_mul(3).saturating_add(1),
            ),
            circle_count: 0,
            typed_count: 0,
            alive_count: 0,
            max_entity_type: 0,
            max_entity_type_dirty: false,
            entity_reorder_map: Vec::new(),
        }
    }
    pub fn new(bounding_box: Rectangle) -> Self {
        Self::new_with_config(bounding_box, Config::default())
    }
    pub(crate) fn owner_lookup(&self, value: u32) -> Option<u32> {
        let idx = value as usize;
        if idx < self.dense_owner.len() {
            let stored = self.dense_owner[idx];
            if stored != u32::MAX {
                return Some(stored);
            }
        }
        self.owner_map.get(&value).copied()
    }
    pub(crate) fn owner_insert(&mut self, value: u32, entity_idx: u32) {
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
    pub(crate) fn owner_remove(&mut self, value: u32) -> Option<u32> {
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
    pub(crate) fn remap_owner_indices(
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
    pub(crate) fn is_large_extent(&self, extent: RectExtent) -> bool {
        if self.large_entity_threshold <= 0.0 {
            return false;
        }
        let w = extent.max_x - extent.min_x;
        let h = extent.max_y - extent.min_y;
        w >= self.large_entity_threshold || h >= self.large_entity_threshold
    }

    #[inline(always)]
    pub(crate) fn is_entity_large(&self, entity_idx: u32) -> bool {
        if self.large_entity_threshold <= 0.0 {
            return false;
        }
        let idx = entity_idx as usize;
        idx < self.large_entity_slots.len() && self.large_entity_slots[idx] != 0
    }

    pub(crate) fn set_large_entity(&mut self, entity_idx: u32, is_large: bool) -> bool {
        if self.large_entity_threshold <= 0.0 {
            return false;
        }
        let idx = entity_idx as usize;
        if self.large_entity_slots.len() <= idx {
            self.large_entity_slots.resize(idx + 1, 0);
        }
        let slot = self.large_entity_slots[idx];
        if is_large {
            if slot != 0 {
                return false;
            }
            self.large_entities.push(entity_idx);
            self.large_entity_slots[idx] = self.large_entities.len() as u32;
            true
        } else {
            if slot == 0 {
                return false;
            }
            let remove_idx = slot as usize - 1;
            let last = self.large_entities.pop().unwrap();
            if remove_idx < self.large_entities.len() {
                self.large_entities[remove_idx] = last;
                self.large_entity_slots[last as usize] = slot;
            }
            self.large_entity_slots[idx] = 0;
            true
        }
    }

    pub(crate) fn update_large_entity_state(
        &mut self,
        entity_idx: u32,
        extent: RectExtent,
    ) -> (bool, bool) {
        let was_large = self.is_entity_large(entity_idx);
        let is_large = self.is_large_extent(extent);
        if was_large != is_large {
            self.set_large_entity(entity_idx, is_large);
            if is_large {
                self.entities[entity_idx as usize].in_nodes_minus_one = 0;
            }
        }
        (was_large, is_large)
    }

    pub(crate) fn remove_large_entity(&mut self, entity_idx: u32) {
        if self.large_entity_threshold <= 0.0 {
            return;
        }
        let _ = self.set_large_entity(entity_idx, false);
    }

    #[inline(always)]
    pub(crate) fn alloc_entity_with_metadata(
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
            self.entity_values[idx as usize] = value;
            self.query_marks[idx as usize] = 0;
            self.entity_extents.set(idx as usize, extent);
            if shape_kind == SHAPE_CIRCLE {
                let data = self.ensure_circle_data();
                data[idx as usize] = circle_value;
            }
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
            self.entity_extents.push(extent);
            self.query_marks.push(0);
            self.entity_values.push(value);
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
                    data.resize(self.entities.len(), CircleData::new(0.0, 0.0, 0.0));
                }
            }
            idx
        };
        self.alive_count = self.alive_count.saturating_add(1);
        if shape_kind == SHAPE_CIRCLE {
            self.circle_count = self.circle_count.saturating_add(1);
        }
        if self.large_entity_threshold > 0.0 && self.large_entity_slots.len() < self.entities.len()
        {
            self.large_entity_slots.resize(self.entities.len(), 0);
        }
        self.update_large_entity_state(idx, extent);
        if stored_type != u32::MAX {
            self.update_max_entity_type_on_insert(stored_type);
        }
        idx
    }

    #[inline(always)]
    pub(crate) fn update_max_entity_type_on_insert(&mut self, stored_type: u32) {
        if stored_type == u32::MAX {
            return;
        }
        if self.typed_count == 0 {
            self.max_entity_type = stored_type;
            self.max_entity_type_dirty = false;
            return;
        }
        if self.max_entity_type_dirty {
            if stored_type >= self.max_entity_type {
                self.max_entity_type = stored_type;
                self.max_entity_type_dirty = false;
            }
            return;
        }
        if stored_type > self.max_entity_type {
            self.max_entity_type = stored_type;
        }
    }

    #[inline(always)]
    pub(crate) fn mark_max_entity_type_dirty_if_needed(&mut self, stored_type: u32) {
        if stored_type != u32::MAX && stored_type == self.max_entity_type {
            self.max_entity_type_dirty = true;
        }
    }
    pub(crate) fn entity_extent(&self, entity_idx: u32) -> RectExtent {
        let idx = entity_idx as usize;
        self.entity_extents.extent(idx)
    }
    pub(crate) fn shape_metadata(shape: &ShapeEnum) -> (u8, RectExtent, Option<CircleData>) {
        match shape {
            ShapeEnum::Rectangle(rect) => {
                validate_rect_dims(rect.width, rect.height);
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
                validate_circle_radius(radius);
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
    pub(crate) fn ensure_entity_types(&mut self) -> &mut Vec<u32> {
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
        self.entity_types
            .as_mut()
            .expect("entity types not initialized")
    }

    pub(crate) fn ensure_circle_data(&mut self) -> &mut Vec<CircleData> {
        if self.circle_data.is_none() {
            let mut data = Vec::new();
            data.resize(self.entities.len().max(1), CircleData::new(0.0, 0.0, 0.0));
            self.circle_data = Some(data);
            if self.circle_data_scratch.is_none() {
                self.circle_data_scratch = Some(Vec::new());
            }
        } else if let Some(data) = self.circle_data.as_mut() {
            if data.len() < self.entities.len() {
                data.resize(self.entities.len(), CircleData::new(0.0, 0.0, 0.0));
            }
        }
        self.circle_data
            .as_mut()
            .expect("circle data not initialized")
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
        validate_circle_radius(radius);
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
}
