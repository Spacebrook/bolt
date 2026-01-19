use super::*;
use crate::error::QuadtreeResult;
use common::shapes::ShapeEnum;

impl QuadTreeInner {
    pub fn relocate_batch(
        &mut self,
        relocation_requests: &[RelocationRequest],
    ) -> QuadtreeResult<()> {
        for request in relocation_requests {
            self.relocate(request.value, request.shape.clone(), request.entity_type)?;
        }
        Ok(())
    }

    pub fn relocate(
        &mut self,
        value: u32,
        shape: ShapeEnum,
        entity_type: EntityTypeUpdate,
    ) -> QuadtreeResult<()> {
        let (shape_kind, extent, circle_data) = Self::shape_metadata(&shape)?;
        self.ensure_extent_in_bounds(extent)?;
        self.relocate_with_metadata(value, shape_kind, extent, circle_data, entity_type);
        Ok(())
    }

    pub fn relocate_rect_extent(
        &mut self,
        value: u32,
        min_x: f32,
        min_y: f32,
        max_x: f32,
        max_y: f32,
        entity_type: EntityTypeUpdate,
    ) -> QuadtreeResult<()> {
        let extent = RectExtent::from_min_max(min_x, min_y, max_x, max_y)?;
        self.ensure_extent_in_bounds(extent)?;
        self.relocate_with_metadata(value, SHAPE_RECT, extent, None, entity_type);
        Ok(())
    }

    pub fn relocate_circle_raw(
        &mut self,
        value: u32,
        x: f32,
        y: f32,
        radius: f32,
        entity_type: EntityTypeUpdate,
    ) -> QuadtreeResult<()> {
        validate_circle_radius(radius)?;
        let extent = RectExtent::from_min_max(x - radius, y - radius, x + radius, y + radius)?;
        self.ensure_extent_in_bounds(extent)?;
        let circle = CircleData::new(x, y, radius);
        self.relocate_with_metadata(value, SHAPE_CIRCLE, extent, Some(circle), entity_type);
        Ok(())
    }

    fn relocate_with_metadata(
        &mut self,
        value: u32,
        shape_kind: u8,
        extent: RectExtent,
        circle_data: Option<CircleData>,
        entity_type: EntityTypeUpdate,
    ) {
        let entity_idx = match self.owner_lookup(value) {
            Some(idx) => idx,
            None => {
                let entity_type = match entity_type {
                    EntityTypeUpdate::Set(value) => Some(value),
                    EntityTypeUpdate::Clear | EntityTypeUpdate::Preserve => None,
                };
                let entity_idx = self.alloc_entity_with_metadata(
                    value,
                    shape_kind,
                    extent,
                    circle_data,
                    entity_type,
                );
                self.owner_insert(value, entity_idx);
                self.insertions.push(entity_idx);
                self.normalization = Normalization::Hard;
                return;
            }
        };

        self.update_entity_with_metadata(entity_idx, shape_kind, extent, circle_data, entity_type);
    }

    fn update_entity_with_metadata(
        &mut self,
        entity_idx: u32,
        shape_kind: u8,
        extent: RectExtent,
        circle_data: Option<CircleData>,
        entity_type: EntityTypeUpdate,
    ) {
        debug_assert!(shape_kind != SHAPE_CIRCLE || circle_data.is_some());
        let prev_kind = self.entities[entity_idx as usize].shape_kind;
        if prev_kind == shape_kind
            && matches!(entity_type, EntityTypeUpdate::Preserve)
            && self.entity_types.is_none()
        {
            let idx = entity_idx as usize;
            if shape_kind == SHAPE_RECT {
                self.entity_extents.set(idx, extent);
                let entity = &mut self.entities[idx];
                entity.status_changed = self.status_tick;
                self.update_pending = true;
                if self.large_entity_threshold <= 0.0 {
                    return;
                }
                let (was_large, is_large) = self.update_large_entity_state(entity_idx, extent);
                if was_large && !is_large {
                    self.reinsertions.push(entity_idx);
                    self.normalization = Normalization::Hard;
                }
                return;
            }
            if shape_kind == SHAPE_CIRCLE {
                if let Some(data) = circle_data {
                    if let Some(circle_data) = self.circle_data.as_mut() {
                        circle_data[idx] = data;
                    } else {
                        let circle_data = self.ensure_circle_data();
                        circle_data[idx] = data;
                    }
                }
                self.entity_extents.set(idx, extent);
                let entity = &mut self.entities[idx];
                entity.status_changed = self.status_tick;
                self.update_pending = true;
                if self.large_entity_threshold <= 0.0 {
                    return;
                }
                let (was_large, is_large) = self.update_large_entity_state(entity_idx, extent);
                if was_large && !is_large {
                    self.reinsertions.push(entity_idx);
                    self.normalization = Normalization::Hard;
                }
                return;
            }
        }
        if prev_kind != shape_kind {
            if prev_kind == SHAPE_CIRCLE {
                self.circle_count = self.circle_count.saturating_sub(1);
            } else if shape_kind == SHAPE_CIRCLE {
                self.circle_count = self.circle_count.saturating_add(1);
            }
        }
        self.entities[entity_idx as usize].shape_kind = shape_kind;
        if shape_kind == SHAPE_CIRCLE {
            if let Some(data) = circle_data {
                let circle_data = self.ensure_circle_data();
                circle_data[entity_idx as usize] = data;
            }
        }
        if !matches!(entity_type, EntityTypeUpdate::Preserve) {
            let new_type = match entity_type {
                EntityTypeUpdate::Clear => u32::MAX,
                EntityTypeUpdate::Set(value) => value,
                EntityTypeUpdate::Preserve => u32::MAX,
            };
            let mut old_type = u32::MAX;
            if new_type != u32::MAX || self.entity_types.is_some() {
                let types = self.ensure_entity_types();
                old_type = types[entity_idx as usize];
                types[entity_idx as usize] = new_type;
            }
            if old_type == u32::MAX && new_type != u32::MAX {
                self.typed_count = self.typed_count.saturating_add(1);
            } else if old_type != u32::MAX && new_type == u32::MAX {
                self.typed_count = self.typed_count.saturating_sub(1);
                self.mark_max_entity_type_dirty_if_needed(old_type);
            } else if old_type != u32::MAX && old_type != new_type {
                self.mark_max_entity_type_dirty_if_needed(old_type);
            }
            if new_type != u32::MAX {
                self.update_max_entity_type_on_insert(new_type);
            }
            if self.typed_count == 0 {
                self.entity_types = None;
                self.entity_types_scratch = None;
                self.max_entity_type = 0;
                self.max_entity_type_dirty = false;
            }
        }
        if self.circle_count == 0 {
            self.circle_data = None;
            self.circle_data_scratch = None;
        }
        let idx = entity_idx as usize;
        self.entity_extents.set(idx, extent);
        let entity = &mut self.entities[entity_idx as usize];
        entity.status_changed = self.status_tick;
        self.update_pending = true;
        let (was_large, is_large) = self.update_large_entity_state(entity_idx, extent);
        if was_large && !is_large {
            self.reinsertions.push(entity_idx);
            self.normalization = Normalization::Hard;
        }
    }

    pub fn update(&mut self) {
        self.normalize_full();
    }

    pub(crate) fn take_profile_summary(&mut self) -> bool {
        if self.profile_remaining == 0 {
            return false;
        }
        let summary = self.profile_summary;
        if summary || self.profile_detail {
            self.profile_remaining = self.profile_remaining.saturating_sub(1);
        }
        summary
    }

    pub(crate) fn take_query_stats_inner(&mut self) -> QueryStats {
        std::mem::take(&mut self.query_stats)
    }

    #[cfg(feature = "query_stats")]
    fn entity_node_stats(&self) -> (f64, u32) {
        let mut total = 0u64;
        let mut max_nodes = 0u32;
        let mut count = 0u64;
        for entity in self.entities.iter().skip(1) {
            if entity.alive == 0 {
                continue;
            }
            let in_nodes = entity.in_nodes_minus_one.saturating_add(1);
            total += in_nodes as u64;
            count += 1;
            if in_nodes > max_nodes {
                max_nodes = in_nodes;
            }
        }
        let avg = if count > 0 {
            total as f64 / count as f64
        } else {
            0.0
        };
        (avg, max_nodes)
    }

    pub(crate) fn normalize_hard(&mut self) {
        if matches!(
            self.normalization,
            Normalization::Normal | Normalization::Soft
        ) && !self.update_pending
        {
            return;
        }

        let summary = self.take_profile_summary();
        let normalize_hard_start = if summary {
            Some(std::time::Instant::now())
        } else {
            None
        };
        let has_queued_ops = !self.insertions.is_empty()
            || !self.removals.is_empty()
            || !self.node_removals.is_empty()
            || !self.reinsertions.is_empty();

        if !self.update_pending {
            if self.normalization == Normalization::Hard {
                let start = normalize_hard_start.map(|_| std::time::Instant::now());
                self.normalize();
                if let Some(start) = start {
                    eprintln!(
                        "normalize_hard: normalize(no update): {:.3}ms",
                        start.elapsed().as_secs_f64() * 1000.0
                    );
                }
            }
            if let Some(start) = normalize_hard_start {
                eprintln!(
                    "normalize_hard total (no update): {:.3}ms",
                    start.elapsed().as_secs_f64() * 1000.0
                );
            }
            return;
        }

        let mut did_pre_normalize = false;
        let mut normalize_ms = 0.0;
        if has_queued_ops {
            let start = normalize_hard_start.map(|_| std::time::Instant::now());
            self.normalize();
            if let Some(start) = start {
                let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                normalize_ms += elapsed;
                if self.profile_detail {
                    eprintln!("normalize_hard: pre-normalize: {:.3}ms", elapsed);
                }
            }
            did_pre_normalize = true;
        }

        let update_start = normalize_hard_start.map(|_| std::time::Instant::now());
        self.update_entities();
        let mut update_ms = 0.0;
        if let Some(start) = update_start {
            let elapsed = start.elapsed().as_secs_f64() * 1000.0;
            update_ms = elapsed;
            if self.profile_detail {
                eprintln!("normalize_hard: update_entities: {:.3}ms", elapsed);
            }
        }

        if self.normalization == Normalization::Hard {
            let start = normalize_hard_start.map(|_| std::time::Instant::now());
            self.normalize();
            if let Some(start) = start {
                let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                normalize_ms += elapsed;
                if self.profile_detail {
                    eprintln!("normalize_hard: post-normalize(hard): {:.3}ms", elapsed);
                }
            }
        } else if self.normalization == Normalization::Soft && !did_pre_normalize {
            let start = normalize_hard_start.map(|_| std::time::Instant::now());
            self.normalize();
            if let Some(start) = start {
                let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                normalize_ms += elapsed;
                if self.profile_detail {
                    eprintln!("normalize_hard: post-normalize(soft): {:.3}ms", elapsed);
                }
            }
        }

        if let Some(start) = normalize_hard_start {
            let total_ms = start.elapsed().as_secs_f64() * 1000.0;
            if summary {
                eprintln!(
                    "qt_profile tick: update={:.3}ms normalize={:.3}ms total={:.3}ms \
ops(i={}, r={}, nr={}, re={})",
                    update_ms,
                    normalize_ms,
                    total_ms,
                    self.insertions.len(),
                    self.removals.len(),
                    self.node_removals.len(),
                    self.reinsertions.len()
                );
            } else if self.profile_detail {
                eprintln!("normalize_hard total: {:.3}ms", total_ms);
            }
        }
    }

    fn normalize_full(&mut self) {
        if self.normalization == Normalization::Normal && !self.update_pending {
            return;
        }

        let has_queued_ops = !self.insertions.is_empty()
            || !self.removals.is_empty()
            || !self.node_removals.is_empty()
            || !self.reinsertions.is_empty();

        if !self.update_pending {
            self.normalize();
            return;
        }

        if has_queued_ops {
            self.normalize();
        }

        self.update_entities();

        if self.normalization == Normalization::Hard {
            self.normalize();
        }
    }
}
