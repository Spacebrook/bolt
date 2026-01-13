impl QuadTreeInner {
    fn update_entities(&mut self) {
        self.update_pending = false;
        self.update_tick ^= 1;
        let update_tick = self.update_tick;

        let nodes_ptr = self.nodes.as_ptr();
        let node_entities_ptr = self.node_entities.as_ptr();
        let node_entities_next_ptr = self.node_entities_next.as_ptr();
        let node_entities_flags_ptr = self.node_entities_flags.as_mut_ptr();
        let large_entity_slots_ptr = self.large_entity_slots.as_ptr();
        let entities_ptr = self.entities.as_mut_ptr();
        let entity_extents_ptr = self.entity_extents.ptr();
        let node_entity_min_x_ptr = self.node_entity_extents.min_x_mut_ptr();
        let node_entity_packed_ptr = self.node_entity_packed.as_mut_ptr();
        let node_entity_min_y_ptr = self.node_entity_extents.min_y_mut_ptr();
        let node_entity_max_x_ptr = self.node_entity_extents.max_x_mut_ptr();
        let node_entity_max_y_ptr = self.node_entity_extents.max_y_mut_ptr();
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
            }

            let head = node.head();
            if head == 0 {
                continue;
            }

            let node_extent = loose_extent_from_half(half, self.looseness);
            let position_flags = node.position_flags();

            let mut current = head as usize;
            let mut prev = 0u32;
            while current != 0 {
                let node_entity = unsafe { &*node_entities_ptr.add(current) };
                let entity_idx = node_entity.index() as usize;
                let entity = unsafe { &mut *entities_ptr.add(entity_idx) };
                if self.large_entity_threshold > 0.0
                    && unsafe { *large_entity_slots_ptr.add(entity_idx) } != 0
                {
                    self.node_removals.push(NodeRemoval {
                        node_idx,
                        prev_idx: prev,
                        node_entity_idx: current as u32,
                        entity_idx: entity_idx as u32,
                    });
                    self.normalization = Normalization::Hard;
                    let next = unsafe { *node_entities_next_ptr.add(current) };
                    prev = current as u32;
                    current = next as usize;
                    continue;
                }
                if entity.update_tick != update_tick {
                    entity.update_tick = update_tick;
                    entity.reinsertion_tick = update_tick ^ 1;
                }

                if entity.status_changed == self.status_tick {
                    let flags_ptr = unsafe { node_entities_flags_ptr.add(current) };
                    let mut flags = unsafe { *flags_ptr };
                    let mut crossed_new_boundary = false;
                    let extent = unsafe { *entity_extents_ptr.add(entity_idx) };
                    unsafe {
                        *node_entity_min_x_ptr.add(current) = extent.min_x;
                        *node_entity_min_y_ptr.add(current) = extent.min_y;
                        *node_entity_max_x_ptr.add(current) = extent.max_x;
                        *node_entity_max_y_ptr.add(current) = extent.max_y;
                        (*node_entity_packed_ptr.add(current)).set_extent(extent);
                    }
                    let min_x = extent.min_x;
                    let min_y = extent.min_y;
                    let max_x = extent.max_x;
                    let max_y = extent.max_y;

                    if max_y > node_extent.max_y && (position_flags & FLAG_TOP) == 0 {
                        if (flags & FLAG_TOP) == 0 {
                            flags |= FLAG_TOP;
                            crossed_new_boundary = true;
                        }
                    } else if (flags & FLAG_TOP) != 0 {
                        flags &= !FLAG_TOP;
                    }

                    if max_x > node_extent.max_x && (position_flags & FLAG_RIGHT) == 0 {
                        if (flags & FLAG_RIGHT) == 0 {
                            flags |= FLAG_RIGHT;
                            crossed_new_boundary = true;
                        }
                    } else if (flags & FLAG_RIGHT) != 0 {
                        flags &= !FLAG_RIGHT;
                    }

                    if min_y < node_extent.min_y && (position_flags & FLAG_BOTTOM) == 0 {
                        if (flags & FLAG_BOTTOM) == 0 {
                            flags |= FLAG_BOTTOM;
                            crossed_new_boundary = true;
                        }
                    } else if (flags & FLAG_BOTTOM) != 0 {
                        flags &= !FLAG_BOTTOM;
                    }

                    if min_x < node_extent.min_x && (position_flags & FLAG_LEFT) == 0 {
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

                    let mut needs_removal = crossed_new_boundary || flags != 0;
                    if (max_x < node_extent.min_x && (position_flags & FLAG_LEFT) == 0)
                        || (max_y < node_extent.min_y
                            && (position_flags & FLAG_BOTTOM) == 0)
                        || (node_extent.max_x < min_x
                            && (position_flags & FLAG_RIGHT) == 0)
                        || (node_extent.max_y < min_y && (position_flags & FLAG_TOP) == 0)
                    {
                        needs_removal = true;
                    }

                    if needs_removal {
                        self.node_removals.push(NodeRemoval {
                            node_idx,
                            prev_idx: prev,
                            node_entity_idx: current as u32,
                            entity_idx: entity_idx as u32,
                        });
                        self.normalization = Normalization::Hard;
                        if entity.reinsertion_tick != update_tick {
                            entity.reinsertion_tick = update_tick;
                            self.reinsertions.push(entity_idx as u32);
                        }
                    } else if crossed_new_boundary && entity.reinsertion_tick != update_tick {
                        entity.reinsertion_tick = update_tick;
                        self.reinsertions.push(entity_idx as u32);
                        self.normalization = Normalization::Hard;
                    }
                }

                let next = unsafe { *node_entities_next_ptr.add(current) };
                prev = current as u32;
                current = next as usize;
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

        if self.is_large_extent(extent) {
            self.update_large_entity_state(entity_idx, extent);
            self.entities[entity_idx as usize].in_nodes_minus_one = 0;
            return;
        }

        let nodes = &mut self.nodes;
        let node_entities = &mut self.node_entities;
        let node_entity_extents = &mut self.node_entity_extents;
        let node_entity_packed = &mut self.node_entity_packed;
        let node_entity_values = &mut self.node_entity_values;
        let node_entities_next = &mut self.node_entities_next;
        let node_entities_flags = &mut self.node_entities_flags;
        let node_entities_last = &mut self.node_entities_last;
        let entities = &mut self.entities;
        let mut free_node_entity = self.free_node_entity;

        let mut stack = std::mem::take(&mut self.insert_stack);
        stack.clear();
        stack.push((0u32, self.root_half));

        let mut inserted = SmallVec::<[(u32, u32); 16]>::new();

        while let Some((node_idx, half)) = stack.pop() {
            let node_idx_usize = node_idx as usize;
            if !nodes[node_idx_usize].is_leaf() {
                let mut targets = [0usize; 4];
                let targets_len =
                    child_targets_for_extent(half, extent, self.looseness, &mut targets);
                if targets_len == 1 {
                    let child_half = Self::child_half_extent(half, targets[0]);
                    if !extent_fits_in_loose_half(child_half, extent, self.looseness) {
                        // Extent does not fit in the child, keep it in the current node.
                    } else {
                    let child = nodes[node_idx_usize].child(targets[0]);
                    if child != 0 {
                        stack.push((child, child_half));
                        continue;
                    }
                    }
                }
                // Multi-child extents stay in this node to avoid duplication.
            }

            in_nodes += 1;
            let node_extent = loose_extent_from_half(half, self.looseness);
            let position_flags = nodes[node_idx_usize].position_flags();
            let node_entity_idx = if free_node_entity != 0 {
                let idx = free_node_entity;
                free_node_entity = node_entities_next[idx as usize];
                idx
            } else {
                node_entities.push(NodeEntity::new(0));
                node_entity_extents.push(RectExtent::from_min_max(0.0, 0.0, 0.0, 0.0));
                node_entity_packed.push(NodeEntityPacked::default());
                node_entity_values.push(0);
                node_entities_next.push(0);
                node_entities_flags.push(0);
                node_entities_last.push(0);
                (node_entities.len() - 1) as u32
            };
            inserted.push((node_entity_idx, node_idx));
            let head = nodes[node_idx_usize].head();
            node_entities_next[node_entity_idx as usize] = head;
            node_entities[node_entity_idx as usize].set_index(entity_idx);
            node_entities[node_entity_idx as usize].set_dedupe(false);
            node_entity_extents.set(node_entity_idx as usize, extent);
            let value = self.entity_values[entity_idx as usize];
            node_entity_values[node_entity_idx as usize] = value;
            node_entity_packed[node_entity_idx as usize] =
                NodeEntityPacked::from_parts(extent, value, node_entities[node_entity_idx as usize]);
            node_entities_last[node_entity_idx as usize] = (head == 0) as u8;
            node_entities_flags[node_entity_idx as usize] =
                Self::compute_node_entity_flags(node_extent, position_flags, extent);
            let node = &mut nodes[node_idx_usize];
            node.set_head(node_entity_idx);
            node.set_count(node.count() + 1);
        }

        if in_nodes > 1 {
            for (idx, node_idx) in inserted {
                node_entities[idx as usize].set_dedupe(true);
                node_entity_packed[idx as usize].set_entity(node_entities[idx as usize]);
                nodes[node_idx as usize].set_has_dedupe(true);
            }
        }
        if in_nodes == 0 {
            in_nodes = 1;
        }
        self.insert_stack = stack;

        entities[entity_idx as usize].in_nodes_minus_one = in_nodes - 1;
        self.free_node_entity = free_node_entity;
    }

    fn remove_entity(&mut self, entity_idx: u32) {
        self.remove_large_entity(entity_idx);
        let remove_all = self.entities[entity_idx as usize].status_changed == self.status_tick;
        let extent = self.entity_extent(entity_idx);
        let mut stack = std::mem::take(&mut self.remove_stack);
        stack.clear();
        stack.push((0u32, self.root_half));

        while let Some((node_idx, half)) = stack.pop() {
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
                            self.node_entities_last[prev as usize] = 1;
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

                    if self.node_entities[current as usize].has_dedupe() && node.has_dedupe() {
                        let mut has_dedupe = false;
                        let mut scan = node.head();
                        while scan != 0 {
                            if self.node_entities[scan as usize].has_dedupe() {
                                has_dedupe = true;
                                break;
                            }
                            scan = self.node_entities_next[scan as usize];
                        }
                        node.set_has_dedupe(has_dedupe);
                    }
                    current = next;
                    continue;
                }
                prev = current;
                current = self.node_entities_next[current as usize];
            }

            if !node.is_leaf() {
                if remove_all {
                    for i in 0..4 {
                        let child = node.child(i);
                        if child != 0 {
                            stack.push((child, Self::child_half_extent(half, i)));
                        }
                    }
                } else {
                    let mut targets = [0usize; 4];
                    let targets_len =
                        child_targets_for_extent(half, extent, self.looseness, &mut targets);
                    for target in targets.iter().take(targets_len) {
                        let child = node.child(*target);
                        if child != 0 {
                            stack.push((child, Self::child_half_extent(half, *target)));
                        }
                    }
                }
            }
        }

        self.remove_stack = stack;

        let entity_alive = self.entities[entity_idx as usize].alive != 0;
        let entity_shape_kind = self.entities[entity_idx as usize].shape_kind;
        if entity_alive {
            self.alive_count = self.alive_count.saturating_sub(1);
            if entity_shape_kind == SHAPE_CIRCLE {
                self.circle_count = self.circle_count.saturating_sub(1);
            }
        }
        self.entities[entity_idx as usize].alive = 0;
        self.entities[entity_idx as usize].status_changed = self.status_tick ^ 1;
        let mut removed_type = None;
        if let Some(types) = self.entity_types.as_mut() {
            let stored_type = types[entity_idx as usize];
            if stored_type != u32::MAX {
                self.typed_count = self.typed_count.saturating_sub(1);
                removed_type = Some(stored_type);
            }
            types[entity_idx as usize] = u32::MAX;
        }
        if let Some(stored_type) = removed_type {
            self.mark_max_entity_type_dirty_if_needed(stored_type);
        }
        if self.typed_count == 0 {
            self.entity_types = None;
            self.entity_types_scratch = None;
            self.max_entity_type = 0;
            self.max_entity_type_dirty = false;
        }
        if self.circle_count == 0 {
            self.circle_data = None;
            self.circle_data_scratch = None;
        }
        self.entities[entity_idx as usize].next_free = self.free_entity;
        self.free_entity = entity_idx;
    }

    #[inline(always)]
    fn compute_node_entity_flags(
        node_extent: RectExtent,
        position_flags: u8,
        entity_extent: RectExtent,
    ) -> u8 {
        let mut flags = 0u8;

        if entity_extent.max_y > node_extent.max_y && (position_flags & FLAG_TOP) == 0 {
            flags |= FLAG_TOP;
        }
        if entity_extent.max_x > node_extent.max_x && (position_flags & FLAG_RIGHT) == 0 {
            flags |= FLAG_RIGHT;
        }
        if entity_extent.min_y < node_extent.min_y && (position_flags & FLAG_BOTTOM) == 0 {
            flags |= FLAG_BOTTOM;
        }
        if entity_extent.min_x < node_extent.min_x && (position_flags & FLAG_LEFT) == 0 {
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

    #[inline(always)]
    fn descend(
        nodes: &[Node],
        node_idx: u32,
        half: HalfExtent,
        extent: RectExtent,
        looseness: f32,
        stack: &mut NodeStack,
    ) {
        let node = &nodes[node_idx as usize];
        if looseness <= 1.0 {
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
            return;
        }

        for i in 0..4 {
            let child = node.child(i);
            if child == 0 {
                continue;
            }
            let child_half = Self::child_half_extent(half, i);
            let node_extent = loose_extent_from_half(child_half, looseness);
            if extent.min_x <= node_extent.max_x
                && extent.max_x >= node_extent.min_x
                && extent.min_y <= node_extent.max_y
                && extent.max_y >= node_extent.min_y
            {
                stack.push((child, child_half));
            }
        }
    }
}
