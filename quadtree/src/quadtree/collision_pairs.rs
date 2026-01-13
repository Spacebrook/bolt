impl QuadTreeInner {
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
        let allow_duplicates = self.allow_duplicates;
        let nodes = &self.nodes;
        let entities_ptr = self.entities.as_ptr();
        let entity_extents_ptr = self.entity_extents.ptr();
        let entity_values_ptr = self.entity_values.as_ptr();
        let mut stack = std::mem::take(&mut self.query_stack);
        stack.clear();
        stack.push((0u32, self.root_half));

        while let Some((node_idx, half)) = stack.pop() {
            let node = &nodes[node_idx as usize];
            let count = node.count() as usize;
            if count >= 2 {
                let start = node.head() as usize;
                let end = start + count;
                debug_assert!(end <= node_entities_len);


                let mut i = start;
                while i + 1 < end {
                    let a_idx = unsafe { (*node_entities_ptr.add(i)).index() };
                    let a_idx_usize = a_idx as usize;
                    let a_entity = unsafe { &*entities_ptr.add(a_idx_usize) };
                    let a_in_nodes = if allow_duplicates {
                        a_entity.in_nodes_minus_one
                    } else {
                        0
                    };
                    let a_extent = unsafe { *entity_extents_ptr.add(a_idx_usize) };
                    let a_min_x = a_extent.min_x;
                    let a_min_y = a_extent.min_y;
                    let a_max_x = a_extent.max_x;
                    let a_max_y = a_extent.max_y;

                    let mut j = i + 1;
                    while j < end {
                        let b_idx = unsafe { (*node_entities_ptr.add(j)).index() };
                        let b_idx_usize = b_idx as usize;
                        let b_entity = unsafe { &*entities_ptr.add(b_idx_usize) };
                        let b_extent = unsafe { *entity_extents_ptr.add(b_idx_usize) };
                        let b_min_x = b_extent.min_x;
                        let b_min_y = b_extent.min_y;
                        let b_max_x = b_extent.max_x;
                        let b_max_y = b_extent.max_y;

                        if a_max_x >= b_min_x
                            && a_max_y >= b_min_y
                            && b_max_x >= a_min_x
                            && b_max_y >= a_min_y
                        {
                            if allow_duplicates {
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
                                        j += 1;
                                        continue;
                                    }
                                }
                            }

                            let a_value = unsafe { *entity_values_ptr.add(a_idx_usize) };
                            let b_value = unsafe { *entity_values_ptr.add(b_idx_usize) };
                            f(a_value, b_value);
                        }
                        j += 1;
                    }
                    i += 1;
                }
            }

            if !node.is_leaf() {
                for i in 0..4 {
                    let child = node.child(i);
                    if child != 0 {
                        stack.push((child, Self::child_half_extent(half, i)));
                    }
                }
            }
        }

        self.query_stack = stack;
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
        let allow_duplicates = self.allow_duplicates;
        let nodes = &self.nodes;
        let entities_ptr = self.entities.as_ptr();
        let entity_values_ptr = self.entity_values.as_ptr();
        let circle_data_ptr = self
            .circle_data
            .as_ref()
            .expect("circle data missing for circle entities")
            .as_ptr();
        let mut stack = std::mem::take(&mut self.query_stack);
        stack.clear();
        stack.push((0u32, self.root_half));

        while let Some((node_idx, half)) = stack.pop() {
            let node = &nodes[node_idx as usize];
            let count = node.count() as usize;
            if count >= 2 {
                let start = node.head() as usize;
                let end = start + count;
                debug_assert!(end <= node_entities_len);


                let mut i = start;
                while i + 1 < end {
                    let a_idx = unsafe { (*node_entities_ptr.add(i)).index() };
                    let a_idx_usize = a_idx as usize;
                    let a_entity = unsafe { &*entities_ptr.add(a_idx_usize) };
                    let a_in_nodes = if allow_duplicates {
                        a_entity.in_nodes_minus_one
                    } else {
                        0
                    };
                    let a_circle = unsafe { *circle_data_ptr.add(a_idx_usize) };

                    let mut j = i + 1;
                    while j < end {
                        let b_idx = unsafe { (*node_entities_ptr.add(j)).index() };
                        let b_idx_usize = b_idx as usize;
                        let b_entity = unsafe { &*entities_ptr.add(b_idx_usize) };
                        let b_circle = unsafe { *circle_data_ptr.add(b_idx_usize) };

                        if circle_circle_raw(
                            a_circle.x,
                            a_circle.y,
                            a_circle.radius,
                            b_circle.x,
                            b_circle.y,
                            b_circle.radius,
                        ) {
                            if allow_duplicates {
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
                                        j += 1;
                                        continue;
                                    }
                                }
                            }

                            let a_value = unsafe { *entity_values_ptr.add(a_idx_usize) };
                            let b_value = unsafe { *entity_values_ptr.add(b_idx_usize) };
                            f(a_value, b_value);
                        }
                        j += 1;
                    }
                    i += 1;
                }
            }

            if !node.is_leaf() {
                for i in 0..4 {
                    let child = node.child(i);
                    if child != 0 {
                        stack.push((child, Self::child_half_extent(half, i)));
                    }
                }
            }
        }

        self.query_stack = stack;
    }

    fn for_each_collision_pair_mixed<F>(&mut self, f: &mut F)
    where
        F: FnMut(u32, u32),
    {
        let node_entities = &self.node_entities;
        if node_entities.len() <= 1 {
            return;
        }

        let pair_dedupe = &mut self.pair_dedupe;
        let allow_duplicates = self.allow_duplicates;
        let nodes = &self.nodes;
        let entities = &self.entities;
        let entity_extents_ptr = self.entity_extents.ptr();
        let entity_values_ptr = self.entity_values.as_ptr();
        let circle_data = self
            .circle_data
            .as_ref()
            .expect("circle data missing for circle entities");
        let circle_data_ptr = circle_data.as_ptr();
        let mut stack = std::mem::take(&mut self.query_stack);
        stack.clear();
        stack.push((0u32, self.root_half));

        while let Some((node_idx, half)) = stack.pop() {
            let node = &nodes[node_idx as usize];
            let count = node.count() as usize;
            if count >= 2 {
                let start = node.head() as usize;
                let end = start + count;
                debug_assert!(end <= node_entities.len());


                let mut i = start;
                while i + 1 < end {
                    let node_entity = node_entities[i];
                    let a_idx = node_entity.index();
                    let a_idx_usize = a_idx as usize;
                    let a = &entities[a_idx_usize];
                    let a_extent = unsafe { *entity_extents_ptr.add(a_idx_usize) };
                    let a_min_x = a_extent.min_x;
                    let a_min_y = a_extent.min_y;
                    let a_max_x = a_extent.max_x;
                    let a_max_y = a_extent.max_y;
                    let a_is_circle = a.shape_kind == SHAPE_CIRCLE;
                    let a_circle = unsafe { *circle_data_ptr.add(a_idx_usize) };

                    let mut j = i + 1;
                    while j < end {
                        let other_node_entity = node_entities[j];
                        let b_idx = other_node_entity.index();
                        let b_idx_usize = b_idx as usize;
                        let b = &entities[b_idx_usize];
                        let b_is_circle = b.shape_kind == SHAPE_CIRCLE;
                        let b_circle = unsafe { *circle_data_ptr.add(b_idx_usize) };
                        let b_extent = unsafe { *entity_extents_ptr.add(b_idx_usize) };
                        let b_min_x = b_extent.min_x;
                        let b_min_y = b_extent.min_y;
                        let b_max_x = b_extent.max_x;
                        let b_max_y = b_extent.max_y;

                        let hit = if !a_is_circle && !b_is_circle {
                            a_max_x > b_min_x
                                && a_max_y > b_min_y
                                && b_max_x > a_min_x
                                && b_max_y > a_min_y
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
                                a_circle.radius,
                                a_circle.radius_sq,
                                RectExtent::from_min_max(b_min_x, b_min_y, b_max_x, b_max_y),
                            )
                        } else {
                            circle_extent_raw(
                                b_circle.x,
                                b_circle.y,
                                b_circle.radius,
                                b_circle.radius_sq,
                                RectExtent::from_min_max(a_min_x, a_min_y, a_max_x, a_max_y),
                            )
                        };
                        if hit {
                            if allow_duplicates {
                                let needs_dedupe =
                                    a.in_nodes_minus_one > 0 || b.in_nodes_minus_one > 0;
                                if needs_dedupe {
                                    let (min, max) = if a_idx < b_idx {
                                        (a_idx, b_idx)
                                    } else {
                                        (b_idx, a_idx)
                                    };
                                    let key = (u64::from(min) << 32) | u64::from(max);
                                    if !pair_dedupe.insert(key) {
                                        j += 1;
                                        continue;
                                    }
                                }
                            }

                            let a_value = unsafe { *entity_values_ptr.add(a_idx_usize) };
                            let b_value = unsafe { *entity_values_ptr.add(b_idx_usize) };
                            f(a_value, b_value);
                        }

                        j += 1;
                    }

                    i += 1;
                }
            }

            if !node.is_leaf() {
                for i in 0..4 {
                    let child = node.child(i);
                    if child != 0 {
                        stack.push((child, Self::child_half_extent(half, i)));
                    }
                }
            }
        }

        self.query_stack = stack;
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
                    let extent = self.entity_extents.extent(idx);
                    let min_x = extent.min_x;
                    let min_y = extent.min_y;
                    let max_x = extent.max_x;
                    let max_y = extent.max_y;
                    shapes.push(ShapeEnum::Rectangle(Rectangle {
                        x: (min_x + max_x) * 0.5,
                        y: (min_y + max_y) * 0.5,
                        width: max_x - min_x,
                        height: max_y - min_y,
                    }));
                }
            }
        }
    }

    fn for_each_collision_pair<F>(&mut self, mut f: F)
    where
        F: FnMut(u32, u32),
    {
        self.normalize_hard();
        let desired = (self.node_entities.len() + self.large_entities.len()).max(1024);
        self.pair_dedupe.ensure_capacity(desired);
        self.pair_dedupe.clear();
        if self.circle_count == 0 {
            self.for_each_collision_pair_rect_fast(&mut f);
        } else if self.circle_count == self.alive_count {
            self.for_each_collision_pair_circle_fast(&mut f);
        } else {
            self.for_each_collision_pair_mixed(&mut f);
        }

        if self.large_entity_threshold > 0.0 && !self.large_entities.is_empty() {
            let entities = &self.entities;
            let extents_ptr = self.entity_extents.ptr();
            let values_ptr = self.entity_values.as_ptr();
            let circle_data_ptr = self
                .circle_data
                .as_ref()
                .map(|data| data.as_ptr())
                .unwrap_or(std::ptr::null());
            for &a_idx in self.large_entities.iter() {
                let a_entity = &entities[a_idx as usize];
                if a_entity.alive == 0 {
                    continue;
                }
                let a_is_circle = a_entity.shape_kind == SHAPE_CIRCLE;
                let a_extent = unsafe { *extents_ptr.add(a_idx as usize) };
                let a_circle = if a_is_circle && !circle_data_ptr.is_null() {
                    unsafe { *circle_data_ptr.add(a_idx as usize) }
                } else {
                    CircleData::new(0.0, 0.0, 0.0)
                };

                for (b_idx, b_entity) in entities.iter().enumerate().skip(1) {
                    let b_idx = b_idx as u32;
                    if b_idx == a_idx || b_entity.alive == 0 {
                        continue;
                    }
                    let b_is_circle = b_entity.shape_kind == SHAPE_CIRCLE;
                    let b_extent = unsafe { *extents_ptr.add(b_idx as usize) };
                    let b_circle = if b_is_circle && !circle_data_ptr.is_null() {
                        unsafe { *circle_data_ptr.add(b_idx as usize) }
                    } else {
                        CircleData::new(0.0, 0.0, 0.0)
                    };

                    let hits = if a_is_circle {
                        if b_is_circle {
                            circle_circle_raw(
                                a_circle.x,
                                a_circle.y,
                                a_circle.radius,
                                b_circle.x,
                                b_circle.y,
                                b_circle.radius,
                            )
                        } else {
                            circle_extent_raw(
                                a_circle.x,
                                a_circle.y,
                                a_circle.radius,
                                a_circle.radius_sq,
                                b_extent,
                            )
                        }
                    } else if b_is_circle {
                        circle_extent_raw(
                            b_circle.x,
                            b_circle.y,
                            b_circle.radius,
                            b_circle.radius_sq,
                            a_extent,
                        )
                    } else {
                        a_extent.max_x > b_extent.min_x
                            && a_extent.max_y > b_extent.min_y
                            && b_extent.max_x > a_extent.min_x
                            && b_extent.max_y > a_extent.min_y
                    };

                    if hits {
                        let (min, max) = if a_idx < b_idx { (a_idx, b_idx) } else { (b_idx, a_idx) };
                        let key = (u64::from(min) << 32) | u64::from(max);
                        if !self.pair_dedupe.insert(key) {
                            continue;
                        }
                        let a_value = unsafe { *values_ptr.add(a_idx as usize) };
                        let b_value = unsafe { *values_ptr.add(b_idx as usize) };
                        f(a_value, b_value);
                    }
                }
            }
        }
    }
}
