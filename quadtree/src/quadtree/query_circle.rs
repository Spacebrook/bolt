impl QuadTreeInner {
    fn next_query_tick(&mut self) -> u32 {
        self.query_tick = self.query_tick.wrapping_add(1);
        if self.query_tick == 0 {
            self.query_tick = 1;
            self.query_marks.fill(0);
        }
        self.query_tick
    }

    #[inline(always)]
    fn collisions_circle_fast_with<F>(&mut self, query: Query, tick: u32, f: &mut F)
    where
        F: FnMut(u32),
    {
        let query_extent = query.extent;
        let query_kind = query.kind;
        let looseness = self.looseness;
        let nodes_ptr = self.nodes.as_ptr();
        let node_extents_loose_ptr = self.node_extents_loose.ptr();
        let node_center_x_ptr = self.node_centers.x_ptr();
        let node_center_y_ptr = self.node_centers.y_ptr();
        let query_marks_ptr = self.query_marks.as_mut_ptr();
        let node_entity_packed_ptr = self.node_entity_packed.as_ptr();
        let circle_data_ptr = self
            .circle_data
            .as_ref()
            .expect("circle data missing for circle entities")
            .as_ptr();
        #[cfg(feature = "query_stats")]
        let stats = &mut self.query_stats as *mut QueryStats;
        #[cfg(not(feature = "query_stats"))]
        let stats: *mut QueryStats = std::ptr::null_mut();

        let stack_cap = (self.max_depth as usize)
            .saturating_mul(3)
            .saturating_add(1);
        if stack_cap <= QUERY_STACK_INLINE {
            let mut stack = std::mem::MaybeUninit::<[NodeQueryInfo; QUERY_STACK_INLINE]>::uninit();
            let stack_ptr = stack.as_mut_ptr() as *mut NodeQueryInfo;
            let mut node_info = stack_ptr;
            let stack_end = unsafe { stack_ptr.add(QUERY_STACK_INLINE) };
            unsafe {
                node_info.write(NodeQueryInfo { node_idx: 0 });
            }
            node_info = unsafe { node_info.add(1) };

            while node_info != stack_ptr {
                node_info = unsafe { node_info.sub(1) };
                let info = unsafe { *node_info };
                let node = unsafe { &*nodes_ptr.add(info.node_idx as usize) };
                let children = node.children;
                if children[3] != 0 {
                    if let QueryKind::Circle { x, y, radius_sq, .. } = query_kind {
                        let node_extent = unsafe {
                            *node_extents_loose_ptr.add(info.node_idx as usize)
                        };
                        let distance = point_to_extent_distance_sq(x, y, node_extent);
                        if distance > radius_sq {
                            continue;
                        }
                    }

                    if looseness <= 1.0 {
                        let center_x = unsafe { *node_center_x_ptr.add(info.node_idx as usize) };
                        let center_y = unsafe { *node_center_y_ptr.add(info.node_idx as usize) };
                        if query_extent.min_x <= center_x {
                            if query_extent.min_y <= center_y {
                                let child = children[0];
                                if child != 0 {
                                    debug_assert!(node_info < stack_end);
                                    unsafe {
                                        node_info.write(NodeQueryInfo { node_idx: child });
                                    }
                                    node_info = unsafe { node_info.add(1) };
                                }
                            }
                            if query_extent.max_y >= center_y {
                                let child = children[1];
                                if child != 0 {
                                    debug_assert!(node_info < stack_end);
                                    unsafe {
                                        node_info.write(NodeQueryInfo { node_idx: child });
                                    }
                                    node_info = unsafe { node_info.add(1) };
                                }
                            }
                        }
                        if query_extent.max_x >= center_x {
                            if query_extent.min_y <= center_y {
                                let child = children[2];
                                if child != 0 {
                                    debug_assert!(node_info < stack_end);
                                    unsafe {
                                        node_info.write(NodeQueryInfo { node_idx: child });
                                    }
                                    node_info = unsafe { node_info.add(1) };
                                }
                            }
                            if query_extent.max_y >= center_y {
                                let child = children[3];
                                if child != 0 {
                                    debug_assert!(node_info < stack_end);
                                    unsafe {
                                        node_info.write(NodeQueryInfo { node_idx: child });
                                    }
                                    node_info = unsafe { node_info.add(1) };
                                }
                            }
                        }
                    } else {
                        for i in 0..4 {
                            let child = children[i];
                            if child == 0 {
                                continue;
                            }
                            let node_extent = unsafe {
                                *node_extents_loose_ptr.add(child as usize)
                            };
                            if query_extent.min_x <= node_extent.max_x
                                && query_extent.max_x >= node_extent.min_x
                                && query_extent.min_y <= node_extent.max_y
                                && query_extent.max_y >= node_extent.min_y
                            {
                                debug_assert!(node_info < stack_end);
                                unsafe {
                                    node_info.write(NodeQueryInfo { node_idx: child });
                                }
                                node_info = unsafe { node_info.add(1) };
                            }
                        }
                    }
                }

                #[cfg(feature = "query_stats")]
                Self::bump_query_node_ptr(stats);
                let count = node.count as usize;
                if count != 0 {
                    unsafe {
                        if node.has_dedupe() {
                            let dedupe_start = node.dedupe_start() as usize;
                            if dedupe_start > 0 {
                                Self::query_circle_leaf_no_dedupe(
                                    node_entity_packed_ptr,
                                    node.head,
                                    dedupe_start,
                                    circle_data_ptr,
                                    query_extent,
                                    query_kind,
                                    f,
                                    stats,
                                );
                            }
                            let dedupe_count = count.saturating_sub(dedupe_start);
                            if dedupe_count > 0 {
                                Self::query_circle_leaf(
                                    node_entity_packed_ptr,
                                    node.head + dedupe_start as u32,
                                    dedupe_count,
                                    query_marks_ptr,
                                    circle_data_ptr,
                                    query_extent,
                                    query_kind,
                                    tick,
                                    f,
                                    stats,
                                );
                            }
                        } else {
                            Self::query_circle_leaf_no_dedupe(
                                node_entity_packed_ptr,
                                node.head,
                                count,
                                circle_data_ptr,
                                query_extent,
                                query_kind,
                                f,
                                stats,
                            );
                        }
                    }
                }
            }
            unsafe {
                self.query_large_entities_circle(query_extent, query_kind, f, stats);
            }
            return;
        }

        let mut stack = std::mem::take(&mut self.query_info_stack);
        if stack.capacity() < stack_cap {
            stack.reserve(stack_cap - stack.capacity());
        }
        unsafe {
            stack.set_len(stack_cap);
        }
        let stack_ptr = stack.as_mut_ptr();
        let mut node_info = stack_ptr;
        let stack_end = unsafe { stack_ptr.add(stack_cap) };
        unsafe {
            node_info.write(NodeQueryInfo { node_idx: 0 });
        }
        node_info = unsafe { node_info.add(1) };

        while node_info != stack_ptr {
            node_info = unsafe { node_info.sub(1) };
            let info = unsafe { *node_info };
            let node = unsafe { &*nodes_ptr.add(info.node_idx as usize) };
            let children = node.children;
            if children[3] != 0 {
                if let QueryKind::Circle { x, y, radius_sq, .. } = query_kind {
                    let node_extent = unsafe {
                        *node_extents_loose_ptr.add(info.node_idx as usize)
                    };
                    let distance = point_to_extent_distance_sq(x, y, node_extent);
                    if distance > radius_sq {
                        continue;
                    }
                }

                if looseness <= 1.0 {
                    let center_x = unsafe { *node_center_x_ptr.add(info.node_idx as usize) };
                    let center_y = unsafe { *node_center_y_ptr.add(info.node_idx as usize) };
                    if query_extent.min_x <= center_x {
                        if query_extent.min_y <= center_y {
                            let child = children[0];
                            if child != 0 {
                                debug_assert!(node_info < stack_end);
                                unsafe {
                                    node_info.write(NodeQueryInfo { node_idx: child });
                                }
                                node_info = unsafe { node_info.add(1) };
                            }
                        }
                        if query_extent.max_y >= center_y {
                            let child = children[1];
                            if child != 0 {
                                debug_assert!(node_info < stack_end);
                                unsafe {
                                    node_info.write(NodeQueryInfo { node_idx: child });
                                }
                                node_info = unsafe { node_info.add(1) };
                            }
                        }
                    }
                    if query_extent.max_x >= center_x {
                        if query_extent.min_y <= center_y {
                            let child = children[2];
                            if child != 0 {
                                debug_assert!(node_info < stack_end);
                                unsafe {
                                    node_info.write(NodeQueryInfo { node_idx: child });
                                }
                                node_info = unsafe { node_info.add(1) };
                            }
                        }
                        if query_extent.max_y >= center_y {
                            let child = children[3];
                            if child != 0 {
                                debug_assert!(node_info < stack_end);
                                unsafe {
                                    node_info.write(NodeQueryInfo { node_idx: child });
                                }
                                node_info = unsafe { node_info.add(1) };
                            }
                        }
                    }
                } else {
                    for i in 0..4 {
                        let child = children[i];
                        if child == 0 {
                            continue;
                        }
                        let node_extent = unsafe { *node_extents_loose_ptr.add(child as usize) };
                        if query_extent.min_x <= node_extent.max_x
                            && query_extent.max_x >= node_extent.min_x
                            && query_extent.min_y <= node_extent.max_y
                            && query_extent.max_y >= node_extent.min_y
                        {
                            debug_assert!(node_info < stack_end);
                            unsafe {
                                node_info.write(NodeQueryInfo { node_idx: child });
                            }
                            node_info = unsafe { node_info.add(1) };
                        }
                    }
                }
            }

            #[cfg(feature = "query_stats")]
            Self::bump_query_node_ptr(stats);
            let count = node.count as usize;
            if count != 0 {
                unsafe {
                    if node.has_dedupe() {
                        let dedupe_start = node.dedupe_start() as usize;
                        if dedupe_start > 0 {
                            Self::query_circle_leaf_no_dedupe(
                                node_entity_packed_ptr,
                                node.head,
                                dedupe_start,
                                circle_data_ptr,
                                query_extent,
                                query_kind,
                                f,
                                stats,
                            );
                        }
                        let dedupe_count = count.saturating_sub(dedupe_start);
                        if dedupe_count > 0 {
                            Self::query_circle_leaf(
                                node_entity_packed_ptr,
                                node.head + dedupe_start as u32,
                                dedupe_count,
                                query_marks_ptr,
                                circle_data_ptr,
                                query_extent,
                                query_kind,
                                tick,
                                f,
                                stats,
                            );
                        }
                    } else {
                        Self::query_circle_leaf_no_dedupe(
                            node_entity_packed_ptr,
                            node.head,
                            count,
                            circle_data_ptr,
                            query_extent,
                            query_kind,
                            f,
                            stats,
                        );
                    }
                }
            }
        }

        unsafe {
            self.query_large_entities_circle(query_extent, query_kind, f, stats);
        }

        unsafe {
            stack.set_len(0);
        }
        self.query_info_stack = stack;
    }

    #[inline(always)]
    unsafe fn query_large_entities_circle<F>(
        &mut self,
        query_extent: RectExtent,
        query_kind: QueryKind,
        f: &mut F,
        stats: *mut QueryStats,
    ) where
        F: FnMut(u32),
    {
        #[cfg(not(feature = "query_stats"))]
        let _ = stats;
        if self.large_entity_threshold <= 0.0 || self.large_entities.is_empty() {
            return;
        }
        let entities_ptr = self.entities.as_ptr();
        let values_ptr = self.entity_values.as_ptr();
        let extents_ptr = self.entity_extents.ptr();
        let circle_data_ptr = self
            .circle_data
            .as_ref()
            .map(|data| data.as_ptr())
            .unwrap_or(std::ptr::null());
        for &entity_idx in self.large_entities.iter() {
            let entity = &*entities_ptr.add(entity_idx as usize);
            if entity.alive == 0 {
                continue;
            }
            #[cfg(feature = "query_stats")]
            Self::bump_query_entity_ptr(stats);
            if entity.shape_kind == SHAPE_CIRCLE {
                if circle_data_ptr.is_null() {
                    continue;
                }
                let circle = *circle_data_ptr.add(entity_idx as usize);
                match query_kind {
                    QueryKind::Circle { x, y, radius, .. } => {
                        if circle_circle_raw(x, y, radius, circle.x, circle.y, circle.radius) {
                            f(*values_ptr.add(entity_idx as usize));
                        }
                    }
                    QueryKind::Rect { .. } => {
                        if circle_extent_raw(circle.x, circle.y, circle.radius_sq, query_extent) {
                            f(*values_ptr.add(entity_idx as usize));
                        }
                    }
                }
            } else {
                let extent = *extents_ptr.add(entity_idx as usize);
                match query_kind {
                    QueryKind::Circle { x, y, radius_sq, .. } => {
                        if circle_extent_raw(x, y, radius_sq, extent) {
                            f(*values_ptr.add(entity_idx as usize));
                        }
                    }
                    QueryKind::Rect { .. } => {
                        if !(extent.max_x < query_extent.min_x
                            || query_extent.max_x < extent.min_x
                            || extent.max_y < query_extent.min_y
                            || query_extent.max_y < extent.min_y)
                        {
                            f(*values_ptr.add(entity_idx as usize));
                        }
                    }
                }
            }
        }
    }
}
