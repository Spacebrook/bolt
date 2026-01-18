use super::*;

impl QuadTreeInner {
    pub(crate) fn rebuild_storage(&mut self, did_merge: &mut bool) -> bool {
        if self.nodes.is_empty() {
            return false;
        }
        let profile = self.profile_detail && self.profile_remaining > 0;
        if self.typed_count == 0 {
            self.entity_types = None;
            self.entity_types_scratch = None;
        }
        if self.circle_count == 0 {
            self.circle_data = None;
            self.circle_data_scratch = None;
        }
        let total_entities = self.entities.len().saturating_sub(1);
        let dead_entities = total_entities.saturating_sub(self.alive_count as usize);
        let reorder_dead_threshold = (total_entities / 8).max(1);
        let reorder_due = (self.reorder_counter & (ENTITY_REORDER_INTERVAL - 1)) == 0;
        self.reorder_counter = self.reorder_counter.wrapping_add(1);
        let do_entity_reorder =
            total_entities > 0 && (dead_entities >= reorder_dead_threshold || reorder_due);
        if do_entity_reorder {
            return self.rebuild_storage_with_reorder(did_merge, profile);
        } else {
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
            new_node_entities_next.clear();
            new_node_entities_next.reserve(old_node_entities_next.len().max(1));

            let mut new_node_entity_values = std::mem::take(&mut self.node_entity_values_scratch);
            new_node_entity_values.clear();
            new_node_entity_values.reserve(old_node_entity_values.len().max(1));

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
            let mut entities = std::mem::take(&mut self.entities);
            let old_large_entity_slots = std::mem::take(&mut self.large_entity_slots);
            let old_large_entities = std::mem::take(&mut self.large_entities);
            let entity_extents = std::mem::take(&mut self.entity_extents);
            let query_marks = std::mem::take(&mut self.query_marks);
            let entity_values = std::mem::take(&mut self.entity_values);
            let entity_extents_view = EntityExtents {
                extents: &entity_extents.extents,
            };
            let mut mapper = IdentityMapper;
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
                &entity_values,
                &mut entities,
                &entity_extents_view,
                &mut mapper,
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
            self.entities = entities;
            self.entity_extents = entity_extents;
            self.query_marks = query_marks;
            self.entity_values = entity_values;
            if self.large_entity_threshold > 0.0 {
                self.large_entity_slots = old_large_entity_slots;
                self.large_entities = old_large_entities;
            } else {
                self.large_entity_slots.clear();
                self.large_entity_slots.push(0);
                self.large_entities.clear();
            }
        }

        *did_merge
    }
}
