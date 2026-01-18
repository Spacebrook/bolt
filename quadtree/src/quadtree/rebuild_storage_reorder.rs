use super::*;

impl QuadTreeInner {
    pub(crate) fn rebuild_storage_with_reorder(
        &mut self,
        did_merge: &mut bool,
        profile: bool,
    ) -> bool {
        let old_alive_count = self.alive_count;
        let all_rectangles = self.circle_count == 0;
        let all_circles = self.circle_count != 0 && self.circle_count == old_alive_count;
        let has_entity_types = self.typed_count != 0;
        let mut dense_owner = std::mem::take(&mut self.dense_owner);
        let mut owner_map = std::mem::take(&mut self.owner_map);
        let mut old_entities = std::mem::take(&mut self.entities);
        let old_large_entity_slots = std::mem::take(&mut self.large_entity_slots);
        let old_extents = std::mem::take(&mut self.entity_extents);
        let old_query_marks = std::mem::take(&mut self.query_marks);
        let old_values = std::mem::take(&mut self.entity_values);
        let mut old_types = std::mem::take(&mut self.entity_types);
        let mut old_circle_data = std::mem::take(&mut self.circle_data);
        if !has_entity_types {
            old_types = None;
        }
        if all_rectangles {
            old_circle_data = None;
        }
        let needed = old_entities.len();
        let mut entity_reorder_map = std::mem::take(&mut self.entity_reorder_map);
        if entity_reorder_map.len() < needed {
            entity_reorder_map.resize(needed, 0);
        }
        entity_reorder_map.fill(0);
        let mut new_entities = std::mem::take(&mut self.entities_scratch);
        new_entities.clear();
        new_entities.reserve(old_entities.len().max(1));
        let mut new_extents = std::mem::take(&mut self.entity_extents_scratch);
        new_extents.clear();
        new_extents.reserve(old_extents.len().max(1));
        let mut new_query_marks = std::mem::take(&mut self.query_marks_scratch);
        new_query_marks.clear();
        let mut new_values = std::mem::take(&mut self.entity_values_scratch);
        new_values.clear();
        new_values.reserve(old_values.len().max(1));
        let mut new_types = std::mem::take(&mut self.entity_types_scratch);
        if !has_entity_types {
            new_types = None;
        }
        let mut new_circle_data = std::mem::take(&mut self.circle_data_scratch);
        if all_rectangles {
            new_circle_data = None;
        }
        let old_types_vec = if has_entity_types {
            old_types
                .take()
                .expect("entity types missing while typed_count > 0")
        } else {
            Vec::new()
        };
        let old_circle_data_vec = if all_rectangles {
            Vec::new()
        } else {
            old_circle_data
                .take()
                .expect("circle data missing while circle_count > 0")
        };
        let mut new_types_vec = if has_entity_types {
            let mut vec = new_types.take().unwrap_or_default();
            vec.clear();
            vec.reserve(old_types_vec.len().max(1));
            vec
        } else {
            Vec::new()
        };
        let mut new_circle_data_vec = if all_rectangles {
            Vec::new()
        } else {
            let mut vec = new_circle_data.take().unwrap_or_default();
            vec.clear();
            vec.reserve(old_circle_data_vec.len().max(1));
            vec
        };
        // Safety: reserve ensures capacity, we manually write initialized values and set_len after.
        unsafe {
            new_entities.set_len(1);
            *new_entities.as_mut_ptr() = Entity::sentinel();
            if has_entity_types {
                new_types_vec.set_len(1);
                *new_types_vec.as_mut_ptr() = u32::MAX;
            }
            if !all_rectangles {
                new_circle_data_vec.set_len(1);
                *new_circle_data_vec.as_mut_ptr() = CircleData::new(0.0, 0.0, 0.0);
            }
        }
        new_extents.push(RectExtent::from_min_max_unchecked(0.0, 0.0, 0.0, 0.0));
        let start_reorder = if profile {
            Some(std::time::Instant::now())
        } else {
            None
        };
        let (new_len, alive_count, circle_count);
        {
            let mut reorder = EntityReorder {
                old_entities: old_entities.as_ptr(),
                new_entities: new_entities.as_mut_ptr(),
                old_extents: old_extents.extents.as_ptr(),
                new_extents: new_extents.extents.as_mut_ptr(),
                old_values: old_values.as_ptr(),
                new_values: new_values.as_mut_ptr(),
                old_types: if has_entity_types {
                    old_types_vec.as_ptr()
                } else {
                    std::ptr::null()
                },
                new_types: if has_entity_types {
                    new_types_vec.as_mut_ptr()
                } else {
                    std::ptr::null_mut()
                },
                old_circle_data: if all_rectangles {
                    std::ptr::null()
                } else {
                    old_circle_data_vec.as_ptr()
                },
                new_circle_data: if all_rectangles {
                    std::ptr::null_mut()
                } else {
                    new_circle_data_vec.as_mut_ptr()
                },
                entity_map: entity_reorder_map.as_mut_ptr(),
                entity_map_len: entity_reorder_map.len(),
                new_len: 1,
                circle_count: 0,
                alive_count: 0,
                all_rectangles,
                all_circles,
                has_entity_types,
            };
            let mut old_nodes = std::mem::take(&mut self.nodes);
            let old_node_centers = std::mem::take(&mut self.node_centers);
            let old_node_extents_tight = std::mem::take(&mut self.node_extents_tight);
            let old_node_extents_loose = std::mem::take(&mut self.node_extents_loose);
            let mut old_node_entities = std::mem::take(&mut self.node_entities);
            let old_node_entity_extents = std::mem::take(&mut self.node_entity_extents);
            let old_node_entity_packed = std::mem::take(&mut self.node_entity_packed);
            let mut old_node_entities_next = std::mem::take(&mut self.node_entities_next);
            let old_node_entity_values = std::mem::take(&mut self.node_entity_values);
            let mut old_node_entities_flags = std::mem::take(&mut self.node_entities_flags);
            let mut new_nodes = std::mem::take(&mut self.nodes_scratch);
            new_nodes.clear();
            new_nodes.reserve(old_nodes.len().max(1));
            let mut new_node_centers = std::mem::take(&mut self.node_centers_scratch);
            new_node_centers.clear();
            new_node_centers.reserve(old_nodes.len().max(1));
            let mut new_node_extents_tight = std::mem::take(&mut self.node_extents_tight_scratch);
            let mut new_node_extents_loose = std::mem::take(&mut self.node_extents_loose_scratch);
            new_node_extents_tight.clear();
            new_node_extents_loose.clear();
            new_node_extents_tight.reserve(old_nodes.len().max(1));
            new_node_extents_loose.reserve(old_nodes.len().max(1));
            let mut new_node_entities = std::mem::take(&mut self.node_entities_scratch);
            new_node_entities.clear();
            new_node_entities.reserve(old_node_entities.len().max(1));

            let mut new_node_entity_extents = std::mem::take(&mut self.node_entity_extents_scratch);
            let mut new_node_entity_packed = std::mem::take(&mut self.node_entity_packed_scratch);
            new_node_entity_extents.clear();
            new_node_entity_packed.clear();
            new_node_entity_extents.reserve(old_node_entities.len().max(1));
            new_node_entity_packed.reserve(old_node_entities.len().max(1));

            let mut new_node_entities_next = std::mem::take(&mut self.node_entities_next_scratch);
            let mut new_node_entity_values = std::mem::take(&mut self.node_entity_values_scratch);
            new_node_entity_values.clear();
            new_node_entity_values.reserve(old_node_entity_values.len().max(1));
            new_node_entities_next.clear();
            new_node_entities_next.reserve(old_node_entities_next.len().max(1));

            let mut new_node_entities_flags = std::mem::take(&mut self.node_entities_flags_scratch);
            new_node_entities_flags.clear();
            new_node_entities_flags.reserve(old_node_entities_flags.len().max(1));


            new_node_entities.push(NodeEntity::new(0));
            new_node_entity_extents
                .push(RectExtent::from_min_max_unchecked(0.0, 0.0, 0.0, 0.0));
            new_node_entity_packed.push(NodeEntityPacked::default());
            new_node_entities_next.push(0);
            new_node_entity_values.push(0);
            new_node_entities_flags.push(0);

            let start_rebuild = if profile {
                Some(std::time::Instant::now())
            } else {
                None
            };
            let old_extents = EntityExtents {
                extents: &old_extents.extents,
            };
            self.rebuild_nodes_iterative(
                &mut old_nodes,
                &mut old_node_entities,
                &mut old_node_entities_next,
                &mut old_node_entities_flags,
                &old_large_entity_slots,
                &mut new_node_extents_tight,
                &mut new_node_extents_loose,
                &mut new_node_centers,
                &mut new_node_entity_extents,
                &mut new_node_entity_packed,
                &mut new_node_entity_values,
                &old_values,
                &mut old_entities,
                &old_extents,
                &mut reorder,
                &mut new_nodes,
                &mut new_node_entities,
                &mut new_node_entities_next,
                &mut new_node_entities_flags,
                did_merge,
            );
            if let Some(start) = start_rebuild {
                eprintln!(
                    "rebuild_nodes_iterative: {:.3}ms",
                    start.elapsed().as_secs_f64() * 1000.0
                );
            }

            self.nodes_scratch = old_nodes;
            self.node_centers_scratch = old_node_centers;
            self.node_extents_tight_scratch = old_node_extents_tight;
            self.node_extents_loose_scratch = old_node_extents_loose;
            self.node_entities_scratch = old_node_entities;
            self.node_entity_extents_scratch = old_node_entity_extents;
            self.node_entity_packed_scratch = old_node_entity_packed;
            self.node_entities_next_scratch = old_node_entities_next;
            self.node_entity_values_scratch = old_node_entity_values;
            self.node_entities_flags_scratch = old_node_entities_flags;

            self.nodes = new_nodes;
            self.node_centers = new_node_centers;
            self.node_extents_tight = new_node_extents_tight;
            self.node_extents_loose = new_node_extents_loose;
            self.node_entities = new_node_entities;
            self.node_entity_extents = new_node_entity_extents;
            self.node_entity_packed = new_node_entity_packed;
            self.node_entities_next = new_node_entities_next;
            self.node_entity_values = new_node_entity_values;
            self.node_entities_flags = new_node_entities_flags;
            self.free_node = 0;
            self.free_node_entity = 0;

            if reorder.alive_count < old_alive_count {
                for (old_idx, entity) in old_entities.iter().enumerate().skip(1) {
                    if entity.alive == 0 {
                        continue;
                    }
                    if unsafe { *reorder.entity_map.add(old_idx) } != 0 {
                        continue;
                    }
                    let in_nodes = entity.in_nodes_minus_one;
                    reorder.map_entity(old_idx as u32, in_nodes);
                }
            }

            new_len = reorder.new_len;
            alive_count = reorder.alive_count;
            circle_count = reorder.circle_count;
        }

        unsafe {
            new_entities.set_len(new_len);
            new_extents.extents.set_len(new_len);
            new_values.set_len(new_len);
        }
        if has_entity_types {
            unsafe {
                new_types_vec.set_len(new_len);
            }
        }
        if !all_rectangles {
            unsafe {
                new_circle_data_vec.set_len(new_len);
            }
        }

        new_query_marks.resize(new_len, 0);
        for (old_idx, &mapped) in entity_reorder_map.iter().enumerate().skip(1) {
            if mapped == 0 {
                continue;
            }
            if old_idx < old_query_marks.len() {
                new_query_marks[mapped as usize] = old_query_marks[old_idx];
            }
        }

        self.entity_reorder_map = entity_reorder_map;

        if let Some(start) = start_reorder {
            eprintln!(
                "rebuild_entities: {:.3}ms",
                start.elapsed().as_secs_f64() * 1000.0
            );
        }

        self.entities_scratch = old_entities;
        self.entities = new_entities;
        self.entity_extents_scratch = old_extents;
        self.entity_extents = new_extents;
        self.query_marks_scratch = old_query_marks;
        self.query_marks = new_query_marks;
        self.entity_values_scratch = old_values;
        self.entity_values = new_values;
        if has_entity_types {
            self.entity_types = Some(new_types_vec);
            self.entity_types_scratch = Some(old_types_vec);
        } else {
            self.entity_types = None;
            self.entity_types_scratch = None;
        }
        if all_rectangles {
            self.circle_data = None;
            self.circle_data_scratch = None;
        } else {
            self.circle_data = Some(new_circle_data_vec);
            self.circle_data_scratch = Some(old_circle_data_vec);
        }
        self.free_entity = 0;
        self.alive_count = alive_count;
        self.circle_count = circle_count;

        Self::remap_owner_indices(&self.entity_reorder_map, &mut dense_owner, &mut owner_map);
        self.dense_owner = dense_owner;
        self.owner_map = owner_map;
        if self.large_entity_threshold > 0.0 {
            let mut new_large_entity_slots = vec![0u32; self.entities.len()];
            let mut new_large_entities = Vec::new();
            for (old_idx, slot) in old_large_entity_slots.iter().enumerate().skip(1) {
                if *slot == 0 {
                    continue;
                }
                let mapped = self.entity_reorder_map[old_idx];
                if mapped == 0 {
                    continue;
                }
                new_large_entities.push(mapped);
                new_large_entity_slots[mapped as usize] = new_large_entities.len() as u32;
            }
            self.large_entities = new_large_entities;
            self.large_entity_slots = new_large_entity_slots;
        } else {
            self.large_entities.clear();
            self.large_entity_slots.clear();
            self.large_entity_slots.push(0);
        }

        *did_merge
    }
}
