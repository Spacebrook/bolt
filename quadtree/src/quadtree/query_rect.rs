impl QuadTreeInner {
    #[inline(always)]
    fn collisions_rect_fast_with<F>(
        &mut self,
        query_extent: RectExtent,
        tick: u32,
        f: &mut F,
    ) where
        F: FnMut(u32),
    {
        // Keep in sync with collisions_circle_fast_with; duplicated for perf.
        let q_min_x = query_extent.min_x;
        let q_max_x = query_extent.max_x;
        let q_min_y = query_extent.min_y;
        let q_max_y = query_extent.max_y;
        let nodes_ptr = self.nodes.as_ptr();
        let node_entities_ptr = self.node_entities.as_ptr();
        let looseness = self.looseness;
        let node_center_x_ptr = self.node_centers.x_ptr();
        let node_center_y_ptr = self.node_centers.y_ptr();
        let node_extents_tight_ptr = self.node_extents_tight.ptr();
        let node_extents_loose_ptr = self.node_extents_loose.ptr();
        let node_entity_min_x_ptr = self.node_entity_extents.min_x_ptr();
        let node_entity_min_y_ptr = self.node_entity_extents.min_y_ptr();
        let node_entity_max_x_ptr = self.node_entity_extents.max_x_ptr();
        let node_entity_max_y_ptr = self.node_entity_extents.max_y_ptr();
        let node_entity_values_ptr = self.node_entity_values.as_ptr();
        let query_marks_ptr = self.query_marks.as_mut_ptr();
        let use_avx2 = self.use_avx2;
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
                    if looseness <= 1.0 {
                        let center_x = unsafe { *node_center_x_ptr.add(info.node_idx as usize) };
                        let center_y = unsafe { *node_center_y_ptr.add(info.node_idx as usize) };
                        if q_min_x <= center_x {
                            if q_min_y <= center_y {
                                let child = children[0];
                                if child != 0 {
                                    debug_assert!(node_info < stack_end);
                                    unsafe {
                                        node_info.write(NodeQueryInfo { node_idx: child });
                                    }
                                    node_info = unsafe { node_info.add(1) };
                                }
                            }
                            if q_max_y >= center_y {
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
                        if q_max_x >= center_x {
                            if q_min_y <= center_y {
                                let child = children[2];
                                if child != 0 {
                                    debug_assert!(node_info < stack_end);
                                    unsafe {
                                        node_info.write(NodeQueryInfo { node_idx: child });
                                    }
                                    node_info = unsafe { node_info.add(1) };
                                }
                            }
                            if q_max_y >= center_y {
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
                            let child_extent = unsafe {
                                *node_extents_loose_ptr.add(child as usize)
                            };
                            if q_min_x <= child_extent.max_x
                                && q_max_x >= child_extent.min_x
                                && q_min_y <= child_extent.max_y
                                && q_max_y >= child_extent.min_y
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
                    let node_extent = unsafe {
                        *node_extents_tight_ptr.add(info.node_idx as usize)
                    };
                    let contained = q_min_x <= node_extent.min_x
                        && q_max_x >= node_extent.max_x
                        && q_min_y <= node_extent.min_y
                        && q_max_y >= node_extent.max_y;
                    let has_dedupe = node.has_dedupe();
                    unsafe {
                        if contained {
                            if has_dedupe {
                                let dedupe_start = node.dedupe_start() as usize;
                                if dedupe_start > 0 {
                                    Self::query_rect_leaf_contained_raw_no_dedupe(
                                        node_entity_values_ptr,
                                        node.head,
                                        dedupe_start,
                                        f,
                                        stats,
                                    );
                                }
                                let dedupe_count = count.saturating_sub(dedupe_start);
                                if dedupe_count > 0 {
                                    Self::query_rect_leaf_contained_raw_dedupe_soa(
                                        node_entities_ptr,
                                        node_entity_values_ptr,
                                        node.head + dedupe_start as u32,
                                        dedupe_count,
                                        query_marks_ptr,
                                        tick,
                                        f,
                                        stats,
                                    );
                                }
                            } else {
                                Self::query_rect_leaf_contained_raw_no_dedupe(
                                    node_entity_values_ptr,
                                    node.head,
                                    count,
                                    f,
                                    stats,
                                );
                            }
                        } else if has_dedupe {
                            let dedupe_start = node.dedupe_start() as usize;
                            if dedupe_start > 0 {
                                Self::query_rect_leaf_raw_no_dedupe(
                                    node_entity_min_x_ptr,
                                    node_entity_min_y_ptr,
                                    node_entity_max_x_ptr,
                                    node_entity_max_y_ptr,
                                    node_entity_values_ptr,
                                    node.head,
                                    dedupe_start,
                                    use_avx2,
                                    q_min_x,
                                    q_min_y,
                                    q_max_x,
                                    q_max_y,
                                    f,
                                    stats,
                                );
                            }
                            let dedupe_count = count.saturating_sub(dedupe_start);
                            if dedupe_count > 0 {
                                Self::query_rect_leaf_raw_dedupe_soa(
                                    node_entity_min_x_ptr,
                                    node_entity_min_y_ptr,
                                    node_entity_max_x_ptr,
                                    node_entity_max_y_ptr,
                                    node_entities_ptr,
                                    node_entity_values_ptr,
                                    node.head + dedupe_start as u32,
                                    dedupe_count,
                                    use_avx2,
                                    q_min_x,
                                    q_min_y,
                                    q_max_x,
                                    q_max_y,
                                    query_marks_ptr,
                                    tick,
                                    f,
                                    stats,
                                );
                            }
                        } else {
                            Self::query_rect_leaf_raw_no_dedupe(
                                node_entity_min_x_ptr,
                                node_entity_min_y_ptr,
                                node_entity_max_x_ptr,
                                node_entity_max_y_ptr,
                                node_entity_values_ptr,
                                node.head,
                                count,
                                use_avx2,
                                q_min_x,
                                q_min_y,
                                q_max_x,
                                q_max_y,
                                f,
                                stats,
                            );
                        }
                    }
                }
            }
            unsafe {
                self.query_large_entities_rect(query_extent, f, stats);
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
                if looseness <= 1.0 {
                    let center_x = unsafe { *node_center_x_ptr.add(info.node_idx as usize) };
                    let center_y = unsafe { *node_center_y_ptr.add(info.node_idx as usize) };
                    if q_min_x <= center_x {
                        if q_min_y <= center_y {
                            let child = children[0];
                            if child != 0 {
                                debug_assert!(node_info < stack_end);
                                unsafe {
                                    node_info.write(NodeQueryInfo { node_idx: child });
                                }
                                node_info = unsafe { node_info.add(1) };
                            }
                        }
                        if q_max_y >= center_y {
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
                    if q_max_x >= center_x {
                        if q_min_y <= center_y {
                            let child = children[2];
                            if child != 0 {
                                debug_assert!(node_info < stack_end);
                                unsafe {
                                    node_info.write(NodeQueryInfo { node_idx: child });
                                }
                                node_info = unsafe { node_info.add(1) };
                            }
                        }
                        if q_max_y >= center_y {
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
                        let child_extent = unsafe { *node_extents_loose_ptr.add(child as usize) };
                        if q_min_x <= child_extent.max_x
                            && q_max_x >= child_extent.min_x
                            && q_min_y <= child_extent.max_y
                            && q_max_y >= child_extent.min_y
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
                let node_extent = unsafe { *node_extents_tight_ptr.add(info.node_idx as usize) };
                let contained = q_min_x <= node_extent.min_x
                    && q_max_x >= node_extent.max_x
                    && q_min_y <= node_extent.min_y
                    && q_max_y >= node_extent.max_y;
                let has_dedupe = node.has_dedupe();
                unsafe {
                    if contained {
                        if has_dedupe {
                            let dedupe_start = node.dedupe_start() as usize;
                            if dedupe_start > 0 {
                                Self::query_rect_leaf_contained_raw_no_dedupe(
                                    node_entity_values_ptr,
                                    node.head,
                                    dedupe_start,
                                    f,
                                    stats,
                                );
                            }
                            let dedupe_count = count.saturating_sub(dedupe_start);
                            if dedupe_count > 0 {
                                Self::query_rect_leaf_contained_raw_dedupe_soa(
                                    node_entities_ptr,
                                    node_entity_values_ptr,
                                    node.head + dedupe_start as u32,
                                    dedupe_count,
                                    query_marks_ptr,
                                    tick,
                                    f,
                                    stats,
                                );
                            }
                        } else {
                            Self::query_rect_leaf_contained_raw_no_dedupe(
                                node_entity_values_ptr,
                                node.head,
                                count,
                                f,
                                stats,
                            );
                        }
                    } else if has_dedupe {
                        let dedupe_start = node.dedupe_start() as usize;
                        if dedupe_start > 0 {
                            Self::query_rect_leaf_raw_no_dedupe(
                                node_entity_min_x_ptr,
                                node_entity_min_y_ptr,
                                node_entity_max_x_ptr,
                                node_entity_max_y_ptr,
                                node_entity_values_ptr,
                                node.head,
                                dedupe_start,
                                use_avx2,
                                q_min_x,
                                q_min_y,
                                q_max_x,
                                q_max_y,
                                f,
                                stats,
                            );
                        }
                        let dedupe_count = count.saturating_sub(dedupe_start);
                        if dedupe_count > 0 {
                            Self::query_rect_leaf_raw_dedupe_soa(
                                node_entity_min_x_ptr,
                                node_entity_min_y_ptr,
                                node_entity_max_x_ptr,
                                node_entity_max_y_ptr,
                                node_entities_ptr,
                                node_entity_values_ptr,
                                node.head + dedupe_start as u32,
                                dedupe_count,
                                use_avx2,
                                q_min_x,
                                q_min_y,
                                q_max_x,
                                q_max_y,
                                query_marks_ptr,
                                tick,
                                f,
                                stats,
                            );
                        }
                    } else {
                        Self::query_rect_leaf_raw_no_dedupe(
                            node_entity_min_x_ptr,
                            node_entity_min_y_ptr,
                            node_entity_max_x_ptr,
                            node_entity_max_y_ptr,
                            node_entity_values_ptr,
                            node.head,
                            count,
                            use_avx2,
                            q_min_x,
                            q_min_y,
                            q_max_x,
                            q_max_y,
                            f,
                            stats,
                        );
                    }
                }
            }
        }

        unsafe {
            self.query_large_entities_rect(query_extent, f, stats);
        }

        unsafe {
            stack.set_len(0);
        }
        self.query_info_stack = stack;
    }

    #[inline(always)]
    unsafe fn query_large_entities_rect<F>(
        &mut self,
        query_extent: RectExtent,
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
        let q_min_x = query_extent.min_x;
        let q_max_x = query_extent.max_x;
        let q_min_y = query_extent.min_y;
        let q_max_y = query_extent.max_y;
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
                if circle_extent_raw(circle.x, circle.y, circle.radius_sq, query_extent) {
                    f(*values_ptr.add(entity_idx as usize));
                }
            } else {
                let extent = *extents_ptr.add(entity_idx as usize);
                if !(extent.max_x < q_min_x
                    || q_max_x < extent.min_x
                    || extent.max_y < q_min_y
                    || q_max_y < extent.min_y)
                {
                    f(*values_ptr.add(entity_idx as usize));
                }
            }
        }
    }
}
