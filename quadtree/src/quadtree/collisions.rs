impl QuadTreeInner {
    pub fn collisions_batch(&mut self, shapes: Vec<ShapeEnum>) -> Vec<Vec<u32>> {
        self.normalize_hard();
        shapes
            .into_iter()
            .map(|shape| {
                let mut collisions = Vec::new();
                self.collisions_from_with_normalized(&shape, None, &mut |value| {
                    collisions.push(value);
                });
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
        let filter = self.resolve_filter(filter.as_ref());
        self.normalize_hard();
        shapes
            .into_iter()
            .map(|shape| {
                let mut collisions = Vec::new();
                self.collisions_from_with_normalized(&shape, filter, &mut |value| {
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
        let filter = self.resolve_filter(filter.as_ref());
        self.collisions_from(&shape, filter, collisions);
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
        let filter = self.resolve_filter(filter.as_ref());
        self.collisions_from_with(&shape, filter, &mut f);
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

    #[inline(always)]
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
        let extent = RectExtent::from_min_max(min_x, min_y, max_x, max_y);
        let tick = self.next_query_tick();
        if self.circle_count == 0 {
            #[cfg(feature = "query_stats")]
            {
                let stats = &mut self.query_stats as *mut QueryStats;
                Self::bump_query_calls_ptr(stats);
            }
            self.collisions_rect_fast_with(extent, tick, &mut f);
            return;
        }
        let query = Query::from_rect_extent(extent);
        self.collisions_inner_with(query, None, &mut f);
    }

    /// Fast path: requires rectangle-only storage and no pending updates.
    #[inline(always)]
    pub fn collisions_rect_extent_fast_with<F>(
        &mut self,
        min_x: f32,
        min_y: f32,
        max_x: f32,
        max_y: f32,
        mut f: F,
    ) where
        F: FnMut(u32),
    {
        debug_assert!(self.circle_count == 0);
        let extent = RectExtent::from_min_max(min_x, min_y, max_x, max_y);
        let tick = self.next_query_tick();
        #[cfg(feature = "query_stats")]
        {
            let stats = &mut self.query_stats as *mut QueryStats;
            Self::bump_query_calls_ptr(stats);
        }
        self.collisions_rect_fast_with(extent, tick, &mut f);
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
        self.collisions_from_with_normalized(query_shape, filter_entity_types, f);
    }

    fn collisions_from_with_normalized<F>(
        &mut self,
        query_shape: &ShapeEnum,
        filter_entity_types: Option<&EntityTypeFilter>,
        f: &mut F,
    ) where
        F: FnMut(u32),
    {
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
        #[cfg(feature = "query_stats")]
        let stats = &mut self.query_stats as *mut QueryStats;
        #[cfg(not(feature = "query_stats"))]
        let _stats: *mut QueryStats = std::ptr::null_mut();
        #[cfg(feature = "query_stats")]
        Self::bump_query_calls_ptr(stats);

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
        let node_entity_extents = &self.node_entity_extents;
        let node_entity_values = &self.node_entity_values;
        let node_entity_packed = &self.node_entity_packed;
        let entities = &self.entities;
        let query_marks = &mut self.query_marks;
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
        let entity_types = self.entity_types.as_ref();

        while let Some((node_idx, half)) = stack.pop() {
            let node = &nodes[node_idx as usize];
            if let QueryKind::Circle { x, y, radius_sq, .. } = query_kind {
                let node_extent = loose_extent_from_half(half, self.looseness);
                let distance = point_to_extent_distance_sq(x, y, node_extent);
                if distance > radius_sq {
                    continue;
                }
            }
            #[cfg(feature = "query_stats")]
            Self::bump_query_node_ptr(stats);
            let count = node.count() as usize;
            if count == 0 {
                if !node.is_leaf() {
                    Self::descend(nodes, node_idx, half, query_extent, self.looseness, &mut stack);
                }
                continue;
            }
            let mut current = node.head() as usize;
            let end = current + count;
            let dedupe_start = if node.has_dedupe() {
                node.dedupe_start() as usize
            } else {
                count
            };
            let dedupe_split = current + dedupe_start.min(count);

            while current < dedupe_split {
                #[cfg(feature = "query_stats")]
                Self::bump_query_entity_ptr(stats);
                let entity_idx = self.node_entities[current].index();
                let entity_idx_usize = entity_idx as usize;
                let entity = &entities[entity_idx_usize];
                let extent = node_entity_extents.extent(current);
                let min_x = extent.min_x;
                let min_y = extent.min_y;
                let max_x = extent.max_x;
                let max_y = extent.max_y;
                let circle = circle_data
                    .map(|data| data[entity_idx_usize])
                    .unwrap_or(default_circle);

                if let Some(filter) = filter_entity_types {
                    let entity_type = entity_types
                        .expect("entity types missing for type filter")[entity_idx_usize];
                    if entity_type == u32::MAX || !filter.contains(entity_type) {
                        current += 1;
                        continue;
                    }
                }

                let hit = match query_kind {
                    QueryKind::Rect => {
                        if all_rectangles || entity.shape_kind == SHAPE_RECT {
                            max_x >= query_extent.min_x
                                && max_y >= query_extent.min_y
                                && query_extent.max_x >= min_x
                                && query_extent.max_y >= min_y
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
                            circle_extent_raw(
                                x,
                                y,
                                radius_sq,
                                RectExtent::from_min_max(min_x, min_y, max_x, max_y),
                            )
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
                    f(node_entity_values[current]);
                }

                current += 1;
            }

            while current < end {
                #[cfg(feature = "query_stats")]
                Self::bump_query_entity_ptr(stats);
                let packed = node_entity_packed[current];
                let entity = packed.entity();
                let entity_idx = entity.index();
                let entity_idx_usize = entity_idx as usize;
                let entity_ref = &entities[entity_idx_usize];
                let min_x = packed.min_x;
                let min_y = packed.min_y;
                let max_x = packed.max_x;
                let max_y = packed.max_y;
                let circle = circle_data
                    .map(|data| data[entity_idx_usize])
                    .unwrap_or(default_circle);

                let has_dedupe = entity.has_dedupe();
                if has_dedupe && query_marks[entity_idx_usize] == tick {
                    current += 1;
                    continue;
                }
                if has_dedupe {
                    query_marks[entity_idx_usize] = tick;
                }

                if let Some(filter) = filter_entity_types {
                    let entity_type = entity_types
                        .expect("entity types missing for type filter")[entity_idx_usize];
                    if entity_type == u32::MAX || !filter.contains(entity_type) {
                        current += 1;
                        continue;
                    }
                }

                let hit = match query_kind {
                    QueryKind::Rect => {
                        if all_rectangles || entity_ref.shape_kind == SHAPE_RECT {
                            max_x >= query_extent.min_x
                                && max_y >= query_extent.min_y
                                && query_extent.max_x >= min_x
                                && query_extent.max_y >= min_y
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
                        if entity_ref.shape_kind == SHAPE_RECT {
                            circle_extent_raw(
                                x,
                                y,
                                radius_sq,
                                RectExtent::from_min_max(min_x, min_y, max_x, max_y),
                            )
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
                    f(packed.value());
                }

                current += 1;
            }
            if !node.is_leaf() {
                Self::descend(nodes, node_idx, half, query_extent, self.looseness, &mut stack);
            }
        }

        self.query_stack = stack;
    }

    #[inline(always)]
    fn resolve_filter<'a>(
        &mut self,
        filter: Option<&'a EntityTypeFilter>,
    ) -> Option<&'a EntityTypeFilter> {
        if let Some(filter) = filter {
            if self.filter_is_universal(filter) {
                None
            } else {
                Some(filter)
            }
        } else {
            None
        }
    }

    fn filter_is_universal(&mut self, filter: &EntityTypeFilter) -> bool {
        if self.typed_count == 0
            || self.typed_count != self.alive_count
            || self.entity_types.is_none()
        {
            return false;
        }
        let max_type = match self.max_entity_type() {
            Some(max_type) => max_type,
            None => return false,
        };
        filter.is_universal_for(max_type)
    }

    fn max_entity_type(&mut self) -> Option<u32> {
        if self.typed_count == 0 || self.entity_types.is_none() {
            self.max_entity_type = 0;
            self.max_entity_type_dirty = false;
            return None;
        }
        if self.max_entity_type_dirty {
            self.recompute_max_entity_type();
        }
        Some(self.max_entity_type)
    }

    fn recompute_max_entity_type(&mut self) {
        if self.typed_count == 0 {
            self.max_entity_type = 0;
            self.max_entity_type_dirty = false;
            return;
        }
        let types = match self.entity_types.as_ref() {
            Some(types) => types,
            None => {
                self.max_entity_type = 0;
                self.max_entity_type_dirty = false;
                return;
            }
        };
        let mut max_value = 0u32;
        for (idx, &value) in types.iter().enumerate().skip(1) {
            if value == u32::MAX {
                continue;
            }
            if self.entities[idx].alive == 0 {
                continue;
            }
            if value > max_value {
                max_value = value;
            }
        }
        self.max_entity_type = max_value;
        self.max_entity_type_dirty = false;
    }
}
