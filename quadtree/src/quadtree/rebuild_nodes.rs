impl QuadTreeInner {
    #[allow(clippy::too_many_arguments)]
    fn rebuild_nodes_iterative<M: EntityMapper>(
        &mut self,
        old_nodes: &mut Vec<Node>,
        old_node_entities: &mut Vec<NodeEntity>,
        old_node_entities_next: &mut Vec<u32>,
        old_node_entities_flags: &mut Vec<u8>,
        old_node_entities_last: &mut Vec<u8>,
        large_entity_slots: &[u32],
        new_node_extents_tight: &mut ExtentAos,
        new_node_extents_loose: &mut ExtentAos,
        new_node_centers: &mut NodeCentersSoa,
        new_node_entity_extents: &mut NodeEntityExtentsSoa,
        new_node_entity_packed: &mut Vec<NodeEntityPacked>,
        new_node_entity_values: &mut Vec<u32>,
        entity_values: &[u32],
        entities: &mut [Entity],
        old_entity_extents: &EntityExtents<'_>,
        mapper: &mut M,
        new_nodes: &mut Vec<Node>,
        new_node_entities: &mut Vec<NodeEntity>,
        new_node_entities_next: &mut Vec<u32>,
        new_node_entities_flags: &mut Vec<u8>,
        new_node_entities_last: &mut Vec<u8>,
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
            new_nodes.push(Node::new_leaf(0));
            new_nodes[info.parent_idx as usize].children[info.child_slot as usize] = new_node_idx;
            let node_extent_tight = info.half.to_rect_extent();
            new_node_extents_tight.push(node_extent_tight);
            new_node_extents_loose.push(loose_extent_from_half(info.half, self.looseness));
            new_node_centers.push(info.half.x, info.half.y);
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
                                        let extent = old_entity_extents.extent(entity_idx as usize);
                                        let flags = Self::compute_node_entity_flags(
                                            node_extent,
                                            position_flags,
                                            extent,
                                        );
                                        let dedupe =
                                            entities[entity_idx as usize].in_nodes_minus_one > 0;
                                        old_node_entities_flags[current as usize] = flags;
                                        old_node_entities[current as usize].set_dedupe(dedupe);
                                        old_node_entities_next[current as usize] = head;
                                        old_node_entities_last[current as usize] = (head == 0) as u8;
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
                            old_nodes[node_idx].set_head(0);
                            old_nodes[node_idx].set_count(0);
                            
                            let mut node_entity_idx = head;
                            while node_entity_idx != 0 {
                            let next_node_entity_idx =
                                old_node_entities_next[node_entity_idx as usize];
                            let entity_idx = old_node_entities[node_entity_idx as usize].index();
                            let extent = old_entity_extents.extent(entity_idx as usize);
                            let mut targets = [0usize; 4];
                            let targets_len =
                                child_targets_for_extent(info.half, extent, self.looseness, &mut targets);

                            debug_assert!(targets_len > 0);

                            let flags = old_node_entities_flags[node_entity_idx as usize];
                            let dedupe =
                                entities[entity_idx as usize].in_nodes_minus_one > 0;
                            old_node_entities_flags[node_entity_idx as usize] = flags;
                            old_node_entities[node_entity_idx as usize].set_dedupe(dedupe);

                            if targets_len == 1 {
                                let child_idx = child_indices[targets[0]];
                                let child_head = old_nodes[child_idx as usize].head();
                                old_node_entities_next[node_entity_idx as usize] = child_head;
                                old_node_entities_last[node_entity_idx as usize] = (child_head == 0) as u8;
                                old_nodes[child_idx as usize].set_head(node_entity_idx);
                                let child_count = old_nodes[child_idx as usize].count();
                                old_nodes[child_idx as usize].set_count(child_count + 1);
                            } else {
                                let parent_head = old_nodes[node_idx].head();
                                old_node_entities_next[node_entity_idx as usize] = parent_head;
                                old_node_entities_last[node_entity_idx as usize] =
                                    (parent_head == 0) as u8;
                                old_nodes[node_idx].set_head(node_entity_idx);
                                let parent_count = old_nodes[node_idx].count();
                                old_nodes[node_idx].set_count(parent_count + 1);
                            }

                            node_entity_idx = next_node_entity_idx;
                            }
                            
                            is_leaf = false;
                            }
                            }
                            
                            {
                            let old_node = &old_nodes[node_idx];
                            let position_flags = old_node.position_flags();
                            let head = old_node.head();
                            let count = old_node.count() as usize;
                            let mut new_head = 0u32;
                            let mut new_count = 0u32;
                            let mut has_dedupe = false;
                            if head != 0 && count != 0 {
                            unsafe {
                            let old_node_entities_ptr = old_node_entities.as_ptr();
                            let old_node_entities_next_ptr = old_node_entities_next.as_ptr();
                            let old_node_entities_flags_ptr = old_node_entities_flags.as_ptr();
                            let large_entity_slots_ptr = large_entity_slots.as_ptr();
                            let entities_ptr = entities.as_ptr();
                            let large_entities_enabled = self.large_entity_threshold > 0.0;
                            let fast_no_dedupe = !large_entities_enabled && !old_node.has_dedupe();
                            let start = new_node_entities.len();
                            if fast_no_dedupe {
                            let total_count = count;
                            if total_count != 0 {
                            new_node_entities.reserve(total_count);
                            new_node_entities_next.reserve(total_count);
                            new_node_entities_flags.reserve(total_count);
                            new_node_entities_last.reserve(total_count);
                            new_head = start as u32;
                            new_count = total_count as u32;

                            let needed = start + total_count;
                            if new_node_entities.capacity() < needed {
                                new_node_entities.reserve(needed - new_node_entities.capacity());
                            }
                            if new_node_entities_next.capacity() < needed {
                                new_node_entities_next.reserve(needed - new_node_entities_next.capacity());
                            }
                            if new_node_entities_flags.capacity() < needed {
                                new_node_entities_flags.reserve(needed - new_node_entities_flags.capacity());
                            }
                            if new_node_entities_last.capacity() < needed {
                                new_node_entities_last.reserve(needed - new_node_entities_last.capacity());
                            }
                            if new_node_entity_values.len() < needed {
                                new_node_entity_values.resize(needed, 0);
                            }
                            if new_node_entity_packed.len() < needed {
                                new_node_entity_packed.resize(needed, NodeEntityPacked::default());
                            }
                            if new_node_entity_extents.len() < needed {
                                new_node_entity_extents.resize(needed);
                            }

                            new_node_entities.set_len(start + total_count);
                            new_node_entities_next.set_len(start + total_count);
                            new_node_entities_flags.set_len(start + total_count);
                            new_node_entities_last.set_len(start + total_count);

                            let node_entities_ptr = new_node_entities.as_mut_ptr().add(start);
                            let node_entities_min_x_ptr =
                                new_node_entity_extents.min_x_mut_ptr().add(start);
                            let node_entities_min_y_ptr =
                                new_node_entity_extents.min_y_mut_ptr().add(start);
                            let node_entities_max_x_ptr =
                                new_node_entity_extents.max_x_mut_ptr().add(start);
                            let node_entities_max_y_ptr =
                                new_node_entity_extents.max_y_mut_ptr().add(start);
                            let node_entities_values_ptr = new_node_entity_values.as_mut_ptr().add(start);
                            let node_entities_flags_ptr = new_node_entities_flags.as_mut_ptr().add(start);
                            let node_entities_packed_ptr = new_node_entity_packed.as_mut_ptr().add(start);
                            let node_entities_next_ptr = new_node_entities_next.as_mut_ptr().add(start);
                            let node_entities_last_ptr = new_node_entities_last.as_mut_ptr().add(start);

                            let mut write_offset = 0usize;
                            let mut current = head;
                            while current != 0 {
                                let node_entity = *old_node_entities_ptr.add(current as usize);
                                let entity_idx = node_entity.index();
                                let in_nodes =
                                    (*entities_ptr.add(entity_idx as usize)).in_nodes_minus_one;
                                let mapped_idx = mapper.map_entity(entity_idx, in_nodes);
                                let out = write_offset;
                                let mut new_entity = NodeEntity::new(mapped_idx);
                                new_entity.set_dedupe(false);
                                *node_entities_ptr.add(out) = new_entity;
                                let extent = old_entity_extents.extent(entity_idx as usize);
                                *node_entities_min_x_ptr.add(out) = extent.min_x;
                                *node_entities_min_y_ptr.add(out) = extent.min_y;
                                *node_entities_max_x_ptr.add(out) = extent.max_x;
                                *node_entities_max_y_ptr.add(out) = extent.max_y;
                                *node_entities_values_ptr.add(out) =
                                    *entity_values.get_unchecked(entity_idx as usize);
                                let flags = *old_node_entities_flags_ptr.add(current as usize);
                                *node_entities_flags_ptr.add(out) = flags;
                                *node_entities_packed_ptr.add(out) =
                                    NodeEntityPacked::from_parts(
                                        extent,
                                        *entity_values.get_unchecked(entity_idx as usize),
                                        new_entity,
                                    );
                                write_offset += 1;
                                current = *old_node_entities_next_ptr.add(current as usize);
                            }

                            debug_assert_eq!(write_offset, total_count);

                            let mut offset = 0usize;
                            while offset < total_count {
                                *node_entities_next_ptr.add(offset) = if offset + 1 == total_count {
                                    0
                                } else {
                                    (start + offset + 1) as u32
                                };
                                *node_entities_last_ptr.add(offset) = (offset + 1 == total_count) as u8;
                                offset += 1;
                            }

                            new_nodes[new_node_idx as usize].set_dedupe_start(total_count as u32);
                            has_dedupe = false;
                        }
                        } else {
                            let mut non_dedupe_count = 0usize;
                            let mut dedupe_count = 0usize;
                            let mut current = head;
                            while current != 0 {
                            let node_entity = *old_node_entities_ptr.add(current as usize);
                            let entity_idx = node_entity.index();
                            if large_entities_enabled && *large_entity_slots_ptr.add(entity_idx as usize) != 0 {
                                current = *old_node_entities_next_ptr.add(current as usize);
                                continue;
                            }
                            if node_entity.has_dedupe() {
                                dedupe_count += 1;
                                has_dedupe = true;
                            } else {
                                non_dedupe_count += 1;
                            }
                            current = *old_node_entities_next_ptr.add(current as usize);
                        }
                        let total_count = non_dedupe_count + dedupe_count;
                        if total_count != 0 {
                            new_node_entities.reserve(total_count);
                            new_node_entities_next.reserve(total_count);
                            new_node_entities_flags.reserve(total_count);
                            new_node_entities_last.reserve(total_count);
                            new_head = start as u32;
                            new_count = total_count as u32;
                            
                            let needed = start + total_count;
                            if new_node_entities.capacity() < needed {
                                new_node_entities.reserve(needed - new_node_entities.capacity());
                            }
                            if new_node_entities_next.capacity() < needed {
                                new_node_entities_next.reserve(needed - new_node_entities_next.capacity());
                            }
                            if new_node_entities_flags.capacity() < needed {
                                new_node_entities_flags.reserve(needed - new_node_entities_flags.capacity());
                            }
                            if new_node_entities_last.capacity() < needed {
                                new_node_entities_last.reserve(needed - new_node_entities_last.capacity());
                            }
                            if new_node_entity_values.len() < needed {
                                new_node_entity_values.resize(needed, 0);
                            }
                            if new_node_entity_packed.len() < needed {
                                new_node_entity_packed.resize(needed, NodeEntityPacked::default());
                            }
                            if new_node_entity_extents.len() < needed {
                                new_node_entity_extents.resize(needed);
                            }
                            
                            new_node_entities.set_len(start + total_count);
                            new_node_entities_next.set_len(start + total_count);
                            new_node_entities_flags.set_len(start + total_count);
                            new_node_entities_last.set_len(start + total_count);
                            
                            let node_entities_ptr = new_node_entities.as_mut_ptr().add(start);
                            let node_entities_min_x_ptr =
                                new_node_entity_extents.min_x_mut_ptr().add(start);
                            let node_entities_min_y_ptr =
                                new_node_entity_extents.min_y_mut_ptr().add(start);
                            let node_entities_max_x_ptr =
                                new_node_entity_extents.max_x_mut_ptr().add(start);
                            let node_entities_max_y_ptr =
                                new_node_entity_extents.max_y_mut_ptr().add(start);
                            let node_entities_values_ptr = new_node_entity_values.as_mut_ptr().add(start);
                            let node_entities_flags_ptr = new_node_entities_flags.as_mut_ptr().add(start);
                            let node_entities_packed_ptr = new_node_entity_packed.as_mut_ptr().add(start);
                            let node_entities_next_ptr = new_node_entities_next.as_mut_ptr().add(start);
                            let node_entities_last_ptr = new_node_entities_last.as_mut_ptr().add(start);
                            
                            let mut write_offset = 0usize;
                            current = head;
                            while current != 0 {
                                let node_entity = *old_node_entities_ptr.add(current as usize);
                                if !node_entity.has_dedupe() {
                                    let entity_idx = node_entity.index();
                                    if !large_entities_enabled || *large_entity_slots_ptr.add(entity_idx as usize) == 0 {
                                        let in_nodes =
                                            (*entities_ptr.add(entity_idx as usize)).in_nodes_minus_one;
                                        let mapped_idx = mapper.map_entity(entity_idx, in_nodes);
                                        let out = write_offset;
                                        let mut new_entity = NodeEntity::new(mapped_idx);
                                        new_entity.set_dedupe(false);
                                        *node_entities_ptr.add(out) = new_entity;
                                        let extent = old_entity_extents.extent(entity_idx as usize);
                                        *node_entities_min_x_ptr.add(out) = extent.min_x;
                                        *node_entities_min_y_ptr.add(out) = extent.min_y;
                                        *node_entities_max_x_ptr.add(out) = extent.max_x;
                                        *node_entities_max_y_ptr.add(out) = extent.max_y;
                                        *node_entities_values_ptr.add(out) =
                                            *entity_values.get_unchecked(entity_idx as usize);
                                        let flags = *old_node_entities_flags_ptr.add(current as usize);
                                        *node_entities_flags_ptr.add(out) = flags;
                                        *node_entities_packed_ptr.add(out) =
                                            NodeEntityPacked::from_parts(
                                                extent,
                                                *entity_values.get_unchecked(entity_idx as usize),
                                                new_entity,
                                            );
                                        write_offset += 1;
                                    }
                                }
                                current = *old_node_entities_next_ptr.add(current as usize);
                            }
                            
                            let dedupe_start = write_offset as u32;
                            
                            current = head;
                            while current != 0 {
                                let node_entity = *old_node_entities_ptr.add(current as usize);
                                if node_entity.has_dedupe() {
                                    let entity_idx = node_entity.index();
                                    if !large_entities_enabled || *large_entity_slots_ptr.add(entity_idx as usize) == 0 {
                                        let in_nodes =
                                            (*entities_ptr.add(entity_idx as usize)).in_nodes_minus_one;
                                        let mapped_idx = mapper.map_entity(entity_idx, in_nodes);
                                        let out = write_offset;
                                        let mut new_entity = NodeEntity::new(mapped_idx);
                                        new_entity.set_dedupe(true);
                                        *node_entities_ptr.add(out) = new_entity;
                                        let extent = old_entity_extents.extent(entity_idx as usize);
                                        *node_entities_min_x_ptr.add(out) = extent.min_x;
                                        *node_entities_min_y_ptr.add(out) = extent.min_y;
                                        *node_entities_max_x_ptr.add(out) = extent.max_x;
                                        *node_entities_max_y_ptr.add(out) = extent.max_y;
                                        *node_entities_values_ptr.add(out) =
                                            *entity_values.get_unchecked(entity_idx as usize);
                                        let flags = *old_node_entities_flags_ptr.add(current as usize);
                                        *node_entities_flags_ptr.add(out) = flags;
                                        *node_entities_packed_ptr.add(out) =
                                            NodeEntityPacked::from_parts(
                                                extent,
                                                *entity_values.get_unchecked(entity_idx as usize),
                                                new_entity,
                                            );
                                        write_offset += 1;
                                    }
                                }
                                current = *old_node_entities_next_ptr.add(current as usize);
                            }
                            
                            debug_assert_eq!(write_offset, total_count);
                            
                            let mut offset = 0usize;
                            while offset < total_count {
                                *node_entities_next_ptr.add(offset) = if offset + 1 == total_count {
                                    0
                                } else {
                                    (start + offset + 1) as u32
                                };
                                *node_entities_last_ptr.add(offset) = (offset + 1 == total_count) as u8;
                                offset += 1;
                            }
                            
                            new_nodes[new_node_idx as usize].set_dedupe_start(dedupe_start);
                        }
                        }
                    }
                }

                let new_node = &mut new_nodes[new_node_idx as usize];
                new_node.position_flags = position_flags;
                new_node.set_head(new_head);
                new_node.set_count(new_count);
                new_node.set_has_dedupe(has_dedupe);
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
            }
        }

        unsafe {
            stack.set_len(0);
        }
        self.rebuild_stack = stack;
    }

}
