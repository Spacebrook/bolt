use super::{Config, EntityTypeUpdate, QuadTree, QuadTreeInner, QueryStats, RelocationRequest};
use crate::error::QuadtreeResult;
use common::shapes::{Rectangle, ShapeEnum};
use std::cell::RefCell;

impl QuadTree {
    pub fn new_with_config(bounding_box: Rectangle, config: Config) -> QuadtreeResult<Self> {
        Ok(Self {
            inner: RefCell::new(QuadTreeInner::new_with_config(bounding_box, config)?),
        })
    }

    pub fn storage_counts(&self) -> (usize, usize, usize) {
        let inner = self.inner.borrow();
        (
            inner.nodes.len(),
            inner.node_entities.len().saturating_sub(1),
            inner.entities.len().saturating_sub(1),
        )
    }

    pub fn new(bounding_box: Rectangle) -> QuadtreeResult<Self> {
        Ok(Self {
            inner: RefCell::new(QuadTreeInner::new(bounding_box)?),
        })
    }

    pub fn insert(
        &mut self,
        value: u32,
        shape: ShapeEnum,
        entity_type: Option<u32>,
    ) -> QuadtreeResult<()> {
        self.inner.get_mut().insert(value, shape, entity_type)
    }

    pub fn insert_rect_extent(
        &mut self,
        value: u32,
        min_x: f32,
        min_y: f32,
        max_x: f32,
        max_y: f32,
        entity_type: Option<u32>,
    ) -> QuadtreeResult<()> {
        self.inner
            .get_mut()
            .insert_rect_extent(value, min_x, min_y, max_x, max_y, entity_type)
    }

    pub fn insert_circle_raw(
        &mut self,
        value: u32,
        x: f32,
        y: f32,
        radius: f32,
        entity_type: Option<u32>,
    ) -> QuadtreeResult<()> {
        self.inner
            .get_mut()
            .insert_circle_raw(value, x, y, radius, entity_type)
    }

    pub fn delete(&mut self, value: u32) {
        self.inner.get_mut().delete(value);
    }

    pub fn relocate_batch(
        &mut self,
        relocation_requests: Vec<RelocationRequest>,
    ) -> QuadtreeResult<()> {
        self.inner.get_mut().relocate_batch(relocation_requests)
    }

    pub fn relocate(
        &mut self,
        value: u32,
        shape: ShapeEnum,
        entity_type: EntityTypeUpdate,
    ) -> QuadtreeResult<()> {
        self.inner.get_mut().relocate(value, shape, entity_type)
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
        self.inner
            .get_mut()
            .relocate_rect_extent(value, min_x, min_y, max_x, max_y, entity_type)
    }

    pub fn relocate_circle_raw(
        &mut self,
        value: u32,
        x: f32,
        y: f32,
        radius: f32,
        entity_type: EntityTypeUpdate,
    ) -> QuadtreeResult<()> {
        self.inner
            .get_mut()
            .relocate_circle_raw(value, x, y, radius, entity_type)
    }

    pub fn update(&mut self) {
        self.inner.get_mut().update();
    }

    /// Note: touching edges are not treated as collisions.
    pub fn collisions_batch(&mut self, shapes: Vec<ShapeEnum>) -> QuadtreeResult<Vec<Vec<u32>>> {
        self.inner.get_mut().collisions_batch(shapes)
    }

    /// Note: touching edges are not treated as collisions.
    pub fn collisions_batch_filter(
        &mut self,
        shapes: Vec<ShapeEnum>,
        filter_entity_types: Option<Vec<u32>>,
    ) -> QuadtreeResult<Vec<Vec<u32>>> {
        self.inner
            .get_mut()
            .collisions_batch_filter(shapes, filter_entity_types)
    }

    /// Note: touching edges are not treated as collisions.
    pub fn collisions(
        &mut self,
        shape: ShapeEnum,
        collisions: &mut Vec<u32>,
    ) -> QuadtreeResult<()> {
        self.inner.get_mut().collisions(shape, collisions)
    }

    /// Note: touching edges are not treated as collisions.
    pub fn collisions_rect_extent(
        &mut self,
        min_x: f32,
        min_y: f32,
        max_x: f32,
        max_y: f32,
        collisions: &mut Vec<u32>,
    ) -> QuadtreeResult<()> {
        self.inner
            .get_mut()
            .collisions_rect_extent(min_x, min_y, max_x, max_y, collisions)
    }

    /// Note: touching edges are not treated as collisions.
    pub fn collisions_circle_raw(
        &mut self,
        x: f32,
        y: f32,
        radius: f32,
        collisions: &mut Vec<u32>,
    ) -> QuadtreeResult<()> {
        self.inner
            .get_mut()
            .collisions_circle_raw(x, y, radius, collisions)
    }

    /// Note: touching edges are not treated as collisions.
    pub fn collisions_filter(
        &mut self,
        shape: ShapeEnum,
        filter_entity_types: Option<Vec<u32>>,
        collisions: &mut Vec<u32>,
    ) -> QuadtreeResult<()> {
        self.inner
            .get_mut()
            .collisions_filter(shape, filter_entity_types, collisions)
    }

    pub fn take_query_stats(&mut self) -> QueryStats {
        self.inner.get_mut().take_query_stats_inner()
    }

    #[cfg(feature = "query_stats")]
    pub fn entity_node_stats(&self) -> (f64, u32) {
        self.inner.borrow().entity_node_stats()
    }

    /// Note: touching edges are not treated as collisions.
    pub fn collisions_with<F>(&mut self, shape: ShapeEnum, f: F) -> QuadtreeResult<()>
    where
        F: FnMut(u32),
    {
        self.inner.get_mut().collisions_with(shape, f)
    }

    /// Note: touching edges are not treated as collisions.
    pub fn collisions_rect_extent_with<F>(
        &mut self,
        min_x: f32,
        min_y: f32,
        max_x: f32,
        max_y: f32,
        f: F,
    ) -> QuadtreeResult<()>
    where
        F: FnMut(u32),
    {
        self.inner
            .get_mut()
            .collisions_rect_extent_with(min_x, min_y, max_x, max_y, f)
    }

    /// Note: touching edges are not treated as collisions.
    pub fn collisions_circle_raw_with<F>(
        &mut self,
        x: f32,
        y: f32,
        radius: f32,
        f: F,
    ) -> QuadtreeResult<()>
    where
        F: FnMut(u32),
    {
        self.inner
            .get_mut()
            .collisions_circle_raw_with(x, y, radius, f)
    }

    /// Note: touching edges are not treated as collisions.
    pub fn collisions_with_filter<F>(
        &mut self,
        shape: ShapeEnum,
        filter_entity_types: Option<Vec<u32>>,
        f: F,
    ) -> QuadtreeResult<()>
    where
        F: FnMut(u32),
    {
        self.inner
            .get_mut()
            .collisions_with_filter(shape, filter_entity_types, f)
    }

    pub fn for_each_collision_pair<F>(&mut self, f: F)
    where
        F: FnMut(u32, u32),
    {
        self.inner.get_mut().for_each_collision_pair(f);
    }

    pub fn all_node_bounding_boxes(&mut self, bounding_boxes: &mut Vec<Rectangle>) {
        self.inner.get_mut().all_node_bounding_boxes(bounding_boxes);
    }

    pub fn all_shapes(&self, shapes: &mut Vec<ShapeEnum>) {
        self.inner.borrow().all_shapes(shapes);
    }
}
