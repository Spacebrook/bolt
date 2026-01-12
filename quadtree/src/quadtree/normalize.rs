impl QuadTreeInner {
    fn normalize(&mut self) {
        if self.normalization == Normalization::Normal {
            return;
        }

        self.normalization = Normalization::Normal;
        let profile = self.profile_detail && self.profile_remaining > 0;
        let mut did_merge = false;

        if !self.node_removals.is_empty() {
            let start = if profile {
                Some(std::time::Instant::now())
            } else {
                None
            };
            for removal in self.node_removals.iter().rev() {
                let node_idx = removal.node_idx;
                let node_entity_idx = removal.node_entity_idx;
                let next = self.node_entities_next[node_entity_idx as usize];
                let was_last = next == 0;
                let node = &mut self.nodes[node_idx as usize];

                if removal.prev_idx != 0 {
                    self.node_entities_next[removal.prev_idx as usize] = next;
                    if was_last {
                        self.node_entities_last[removal.prev_idx as usize] = 1;
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
                if *in_nodes == 0 {
                    self.reinsertions.push(entity_idx as u32);
                } else {
                    *in_nodes -= 1;
                }

                self.node_entities_next[node_entity_idx as usize] = self.free_node_entity;
                self.free_node_entity = node_entity_idx;
            }

            self.node_removals.clear();
            if let Some(start) = start {
                eprintln!(
                    "normalize: node_removals: {:.3}ms",
                    start.elapsed().as_secs_f64() * 1000.0
                );
            }
        }

        if !self.reinsertions.is_empty() {
            let start = if profile {
                Some(std::time::Instant::now())
            } else {
                None
            };
            let mut reinsertions = std::mem::take(&mut self.reinsertions);
            let mut free_node_entity = self.free_node_entity;
            let large_entity_threshold = self.large_entity_threshold;
            let root_half = self.root_half;
            let looseness = self.looseness;

            let mut stack = std::mem::take(&mut self.insert_stack);
            stack.clear();
            let mut touched_node_entities: Vec<u32> = Vec::new();
            let mut touched_nodes: Vec<u32> = Vec::new();


            for entity_idx in reinsertions.iter().copied() {
                touched_node_entities.clear();
                touched_nodes.clear();
                let (alive, extent) = {
                    let idx = entity_idx as usize;
                    let entity = &self.entities[idx];
                    (entity.alive, self.entity_extents.extent(idx))
                };
                if alive == 0 {
                    continue;
                }

                if large_entity_threshold > 0.0 {
                    let w = extent.max_x - extent.min_x;
                    let h = extent.max_y - extent.min_y;
                    if w >= large_entity_threshold || h >= large_entity_threshold {
                        self.update_large_entity_state(entity_idx, extent);
                        self.entities[entity_idx as usize].in_nodes_minus_one = 0;
                        continue;
                    }
                }

                let value = self.entity_values[entity_idx as usize];
                let nodes = &mut self.nodes;
                let node_entities = &mut self.node_entities;
                let node_entity_extents = &mut self.node_entity_extents;
                let node_entity_packed = &mut self.node_entity_packed;
                let node_entity_values = &mut self.node_entity_values;
                let node_entities_next = &mut self.node_entities_next;
                let node_entities_flags = &mut self.node_entities_flags;
                let node_entities_last = &mut self.node_entities_last;
                let entities = &mut self.entities;

                let mut in_nodes = 0u32;
                stack.clear();
                stack.push((0u32, root_half));

                while let Some((node_idx, half)) = stack.pop() {
                    let node_idx_usize = node_idx as usize;
                    if !nodes[node_idx_usize].is_leaf() {
                        let mut targets = [0usize; 4];
                        let targets_len =
                            child_targets_for_extent(half, extent, looseness, &mut targets);
                        if targets_len == 1 {
                            let child_half = Self::child_half_extent(half, targets[0]);
                            if extent_fits_in_loose_half(child_half, extent, looseness) {
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
                    let node_extent = loose_extent_from_half(half, looseness);
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
                            node_entities.push(NodeEntity::new(0));
                            node_entity_extents.push(RectExtent::from_min_max(0.0, 0.0, 0.0, 0.0));
                            node_entity_packed.push(NodeEntityPacked::default());
                            node_entity_values.push(0);
                            node_entities_next.push(0);
                            node_entities_flags.push(0);
                            node_entities_last.push(0);
                            (node_entities.len() - 1) as u32
                        };
                        let head = nodes[node_idx_usize].head();
                        node_entities_next[node_entity_idx as usize] = head;
                        node_entities[node_entity_idx as usize].set_index(entity_idx);
                        node_entity_extents.set(node_entity_idx as usize, extent);
                        node_entity_values[node_entity_idx as usize] = value;
                        node_entity_packed[node_entity_idx as usize] =
                            NodeEntityPacked::from_parts(extent, value, node_entities[node_entity_idx as usize]);
                        node_entities_last[node_entity_idx as usize] = (head == 0) as u8;
                        node_entities_flags[node_entity_idx as usize] =
                            Self::compute_node_entity_flags(node_extent, position_flags, extent);
                        touched_node_entities.push(node_entity_idx);
                        touched_nodes.push(node_idx);
                        let node = &mut nodes[node_idx_usize];
                        node.set_head(node_entity_idx);
                        node.set_count(node.count() + 1);
                    } else {
                        node_entity_extents.set(node_entity_idx as usize, extent);
                        node_entity_values[node_entity_idx as usize] = value;
                        node_entity_packed[node_entity_idx as usize] =
                            NodeEntityPacked::from_parts(extent, value, node_entities[node_entity_idx as usize]);
                        node_entities_flags[node_entity_idx as usize] =
                            Self::compute_node_entity_flags(node_extent, position_flags, extent);
                        touched_node_entities.push(node_entity_idx);
                        touched_nodes.push(node_idx);
                    }
                }

                if in_nodes == 0 {
                    in_nodes = 1;
                }
                let dedupe = in_nodes > 1;
                if !touched_node_entities.is_empty() {
                    for node_entity_idx in touched_node_entities.iter().copied() {
                        let flags = node_entities_flags[node_entity_idx as usize];
                        node_entities_flags[node_entity_idx as usize] = flags;
                        node_entities[node_entity_idx as usize].set_dedupe(dedupe);
                        node_entity_packed[node_entity_idx as usize]
                            .set_entity(node_entities[node_entity_idx as usize]);
                    }
                }

                if !touched_nodes.is_empty() {
                    touched_nodes.sort_unstable();
                    touched_nodes.dedup();
                    for node_idx in touched_nodes.iter().copied() {
                        let node = &mut nodes[node_idx as usize];
                        let mut has_dedupe = false;
                        let mut current = node.head();
                        while current != 0 {
                            if node_entities[current as usize].has_dedupe() {
                                has_dedupe = true;
                                break;
                            }
                            current = node_entities_next[current as usize];
                        }
                        node.set_has_dedupe(has_dedupe);
                    }
                }
                entities[entity_idx as usize].in_nodes_minus_one = in_nodes - 1;
            }

            self.insert_stack = stack;
            self.free_node_entity = free_node_entity;
            reinsertions.clear();
            self.reinsertions = reinsertions;
            if let Some(start) = start {
                eprintln!(
                    "normalize: reinsertions: {:.3}ms",
                    start.elapsed().as_secs_f64() * 1000.0
                );
            }
        }

        if !self.removals.is_empty() {
            let start = if profile {
                Some(std::time::Instant::now())
            } else {
                None
            };
            let mut removals = std::mem::take(&mut self.removals);
            for entity_idx in removals.iter().copied() {
                self.remove_entity(entity_idx);
            }
            removals.clear();
            self.removals = removals;
            if let Some(start) = start {
                eprintln!(
                    "normalize: removals: {:.3}ms",
                    start.elapsed().as_secs_f64() * 1000.0
                );
            }
        }

        if !self.insertions.is_empty() {
            let start = if profile {
                Some(std::time::Instant::now())
            } else {
                None
            };
            let mut insertions = std::mem::take(&mut self.insertions);
            for entity_idx in insertions.iter().copied() {
                if self.entities[entity_idx as usize].alive == 0 {
                    continue;
                }
                let extent = self.entity_extents.extent(entity_idx as usize);
                if self.is_large_extent(extent) {
                    self.update_large_entity_state(entity_idx, extent);
                    self.entities[entity_idx as usize].in_nodes_minus_one = 0;
                    continue;
                }
                self.insert_entity_new(entity_idx);
            }
            insertions.clear();
            self.insertions = insertions;
            if let Some(start) = start {
                eprintln!(
                    "normalize: insertions: {:.3}ms",
                    start.elapsed().as_secs_f64() * 1000.0
                );
            }
        }

        let start = if profile {
            Some(std::time::Instant::now())
        } else {
            None
        };
        if self.rebuild_storage(&mut did_merge) {
            self.normalization = Normalization::Soft;
        }
        if let Some(start) = start {
            eprintln!(
                "normalize: rebuild_storage: {:.3}ms",
                start.elapsed().as_secs_f64() * 1000.0
            );
        }
    }


}
