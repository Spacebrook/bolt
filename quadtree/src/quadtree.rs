use crate::collision_detection;
use common::shapes::{Rectangle, Shape, ShapeEnum};

use fxhash::{FxHashMap, FxHashSet};

#[derive(Clone)]
struct Entity {
    shape: ShapeEnum,
    entity_type: Option<u32>,
}

#[derive(Clone, Copy)]
struct NodeChildren {
    nw: Option<usize>,
    ne: Option<usize>,
    sw: Option<usize>,
    se: Option<usize>,
}

impl NodeChildren {
    fn none() -> Self {
        Self {
            nw: None,
            ne: None,
            sw: None,
            se: None,
        }
    }

    fn as_array(&self) -> [Option<usize>; 4] {
        [self.nw, self.ne, self.sw, self.se]
    }
}

struct QuadNode {
    entities: FxHashMap<u32, Entity>,
    bounding_box: Rectangle,
    children: NodeChildren,
    parent: Option<usize>,
    subdivided: bool,
    depth: usize,
}

impl QuadNode {
    pub fn new() -> Self {
        Self {
            entities: FxHashMap::default(),
            bounding_box: Rectangle::default(),
            children: NodeChildren::none(),
            parent: None,
            subdivided: false,
            depth: 0,
        }
    }

    pub fn reset(&mut self) {
        self.bounding_box = Rectangle::default();
        self.parent = None;
        self.depth = 0;
        self.entities.clear();
        self.children = NodeChildren::none();
        self.subdivided = false;
    }

    pub fn initialize(&mut self, bounding_box: Rectangle, parent: Option<usize>, depth: usize) {
        self.bounding_box = bounding_box;
        self.parent = parent;
        self.depth = depth;
        self.entities.clear();
        self.children = NodeChildren::none();
        self.subdivided = false;
    }
}

pub struct QuadTree {
    root: usize,
    owner_map: FxHashMap<u32, usize>,
    nodes: Vec<QuadNode>,
    free_list: Vec<usize>,
    config: Config,
}

struct EntityTypeFilter {
    small: Option<Vec<u32>>,
    bitset: Option<Vec<bool>>,
    set: Option<FxHashSet<u32>>,
}

impl EntityTypeFilter {
    fn from_vec(values: Vec<u32>) -> Self {
        const SMALL_LIMIT: usize = 16;
        const BITSET_MAX: usize = 4096;
        const BITSET_DENSITY_NUM: usize = 1;
        const BITSET_DENSITY_DEN: usize = 4;
        if values.len() <= SMALL_LIMIT {
            Self {
                small: Some(values),
                bitset: None,
                set: None,
            }
        } else {
            let max_value = values.iter().copied().max().unwrap_or(0) as usize;
            if max_value <= BITSET_MAX
                && values.len() * BITSET_DENSITY_DEN >= (max_value + 1) * BITSET_DENSITY_NUM
            {
                let mut bitset = vec![false; max_value + 1];
                for value in values {
                    bitset[value as usize] = true;
                }
                Self {
                    small: None,
                    bitset: Some(bitset),
                    set: None,
                }
            } else {
                let set = values.into_iter().collect();
                Self {
                    small: None,
                    bitset: None,
                    set: Some(set),
                }
            }
        }
    }

    fn contains(&self, value: u32) -> bool {
        if let Some(list) = &self.small {
            list.contains(&value)
        } else if let Some(bitset) = &self.bitset {
            bitset.get(value as usize).copied().unwrap_or(false)
        } else if let Some(set) = &self.set {
            set.contains(&value)
        } else {
            false
        }
    }
}

impl QuadTree {
    pub fn new_with_config(bounding_box: Rectangle, config: Config) -> Self {
        let mut tree = QuadTree {
            root: 0,
            nodes: Vec::new(),
            free_list: Vec::new(),
            owner_map: FxHashMap::default(),
            config,
        };

        let root = tree.alloc_node();
        tree.nodes[root].initialize(bounding_box, None, 0);
        tree.root = root;

        tree
    }

    pub fn new(bounding_box: Rectangle) -> Self {
        Self::new_with_config(bounding_box, Config::default())
    }

    fn alloc_node(&mut self) -> usize {
        if let Some(index) = self.free_list.pop() {
            self.nodes[index].reset();
            index
        } else {
            self.nodes.push(QuadNode::new());
            self.nodes.len() - 1
        }
    }

    fn free_node(&mut self, index: usize) {
        if self.free_list.len() < self.config.pool_size {
            self.free_list.push(index);
        }
    }

    // Insert a shape with a given value into the quadtree
    pub fn insert(&mut self, value: u32, shape: ShapeEnum, entity_type: Option<u32>) {
        self.insert_into(self.root, value, shape, entity_type);
    }

    // Insert a shape into a given node or its children
    fn insert_into(
        &mut self,
        mut node: usize,
        value: u32,
        shape: ShapeEnum,
        entity_type: Option<u32>,
    ) -> usize {
        let shape_bounding_box = shape.bounding_box();
        loop {
            let need_subdivide;
            {
                let node_ref = &self.nodes[node];
                if (node_ref.entities.len() < self.config.node_capacity && !node_ref.subdivided)
                    || node_ref.depth == self.config.max_depth
                {
                    self.add(node, value, shape, entity_type);
                    return node;
                }

                if !node_ref.subdivided && node_ref.depth < self.config.max_depth {
                    need_subdivide = true;
                } else {
                    let destination =
                        self.get_destination_node_by_bbox(node, &shape_bounding_box);
                    if destination == node {
                        self.add(node, value, shape, entity_type);
                        return node;
                    }

                    node = destination;
                    need_subdivide = false;
                }
            }

            if need_subdivide {
                self.subdivide(node);
            }
        }
    }

    fn get_destination_node_by_bbox(&self, node: usize, bounding_box: &Rectangle) -> usize {
        let node_ref = &self.nodes[node];
        if !node_ref.subdivided {
            return node;
        }

        let children = node_ref.children.as_array();
        for child in children {
            if let Some(child_idx) = child {
                if collision_detection::rectangle_contains_rectangle(
                    &self.nodes[child_idx].bounding_box,
                    bounding_box,
                ) {
                    return child_idx;
                }
            }
        }

        node
    }

    fn add(&mut self, node: usize, value: u32, shape: ShapeEnum, entity_type: Option<u32>) {
        self.nodes[node]
            .entities
            .insert(value, Entity { shape, entity_type });
        self.owner_map.insert(value, node);
    }

    pub fn delete(&mut self, value: u32) {
        if let Some(node) = self.owner_map.remove(&value) {
            self.delete_from(node, value);
            self.clean_upwards(node);
        }
    }

    fn delete_from(&mut self, node: usize, value: u32) {
        self.nodes[node].entities.remove(&value);
    }

    // Subdivide a node into quadrants
    fn subdivide(&mut self, node: usize) {
        let (bounding_box, depth) = {
            let node_ref = &self.nodes[node];
            (node_ref.bounding_box, node_ref.depth)
        };

        let half_width = bounding_box.width / 2.0;
        let half_height = bounding_box.height / 2.0;

        let center_x = bounding_box.x;
        let center_y = bounding_box.y;
        let west_x = center_x - half_width / 2.0;
        let east_x = center_x + half_width / 2.0;
        let north_y = center_y + half_height / 2.0;
        let south_y = center_y - half_height / 2.0;

        let nw = self.alloc_node();
        let ne = self.alloc_node();
        let sw = self.alloc_node();
        let se = self.alloc_node();

        self.nodes[nw].initialize(
            Rectangle {
                x: west_x,
                y: north_y,
                width: half_width,
                height: half_height,
            },
            Some(node),
            depth + 1,
        );
        self.nodes[ne].initialize(
            Rectangle {
                x: east_x,
                y: north_y,
                width: half_width,
                height: half_height,
            },
            Some(node),
            depth + 1,
        );
        self.nodes[sw].initialize(
            Rectangle {
                x: west_x,
                y: south_y,
                width: half_width,
                height: half_height,
            },
            Some(node),
            depth + 1,
        );
        self.nodes[se].initialize(
            Rectangle {
                x: east_x,
                y: south_y,
                width: half_width,
                height: half_height,
            },
            Some(node),
            depth + 1,
        );

        let old_items = {
            let node_ref = &mut self.nodes[node];
            node_ref.children = NodeChildren {
                nw: Some(nw),
                ne: Some(ne),
                sw: Some(sw),
                se: Some(se),
            };
            node_ref.subdivided = true;
            node_ref.entities.drain().collect::<Vec<(u32, Entity)>>()
        };

        for (value, entity) in old_items {
            self.owner_map.remove(&value);
            self.insert_into(node, value, entity.shape, entity.entity_type);
        }
    }

    pub fn collisions_batch(&self, shapes: Vec<ShapeEnum>) -> Vec<Vec<u32>> {
        shapes
            .into_iter()
            .map(|shape| {
                let mut collisions = Vec::new();
                self.collisions(shape, &mut collisions);
                collisions
            })
            .collect()
    }

    pub fn collisions_batch_filter(
        &self,
        shapes: Vec<ShapeEnum>,
        filter_entity_types: Option<Vec<u32>>,
    ) -> Vec<Vec<u32>> {
        let filter = filter_entity_types.map(EntityTypeFilter::from_vec);
        shapes
            .into_iter()
            .map(|shape| {
                let mut collisions = Vec::new();
                self.collisions_from(self.root, &shape, filter.as_ref(), &mut collisions);
                collisions
            })
            .collect()
    }

    pub fn collisions(&self, shape: ShapeEnum, collisions: &mut Vec<u32>) {
        self.collisions_from(self.root, &shape, None, collisions);
    }

    pub fn collisions_filter(
        &self,
        shape: ShapeEnum,
        filter_entity_types: Option<Vec<u32>>,
        collisions: &mut Vec<u32>,
    ) {
        let filter = filter_entity_types.map(EntityTypeFilter::from_vec);
        self.collisions_from(self.root, &shape, filter.as_ref(), collisions);
    }

    fn collisions_from(
        &self,
        node: usize,
        query_shape: &ShapeEnum,
        filter_entity_types: Option<&EntityTypeFilter>,
        collisions: &mut Vec<u32>,
    ) {
        let query_shape_bounding_box = query_shape.bounding_box();
        if !collision_detection::rectangle_rectangle(
            &self.nodes[node].bounding_box,
            &query_shape_bounding_box,
        ) {
            return;
        }

        let mut stack = Vec::with_capacity(32);
        stack.push(node);

        while let Some(current) = stack.pop() {
            let node_ref = &self.nodes[current];
            for (&value, entity) in node_ref.entities.iter() {
                if let Some(filter) = &filter_entity_types {
                    if let Some(entity_type) = entity.entity_type {
                        if !filter.contains(entity_type) {
                            continue;
                        }
                    } else {
                        continue;
                    }
                }

                if collision_detection::shape_shape(query_shape, &entity.shape) {
                    collisions.push(value);
                }
            }

            if node_ref.subdivided {
                let children = node_ref.children.as_array();
                for child in children {
                    if let Some(child_idx) = child {
                        if collision_detection::rectangle_rectangle(
                            &self.nodes[child_idx].bounding_box,
                            &query_shape_bounding_box,
                        ) {
                            stack.push(child_idx);
                        }
                    }
                }
            }
        }
    }

    pub fn relocate_batch(&mut self, relocation_requests: Vec<RelocationRequest>) {
        for request in relocation_requests {
            self.relocate(request.value, request.shape, request.entity_type);
        }
    }

    pub fn relocate(&mut self, value: u32, shape: ShapeEnum, entity_type: Option<u32>) {
        if let Some(node) = self.owner_map.get(&value).copied() {
            let bounding_box = shape.bounding_box();
            if collision_detection::rectangle_contains_rectangle(
                &self.nodes[node].bounding_box,
                &bounding_box,
            ) {
                self.nodes[node]
                    .entities
                    .insert(value, Entity { shape, entity_type });
                return;
            }

            self.delete_from(node, value);
            self.relocate_in(node, value, shape, entity_type);
        } else {
            self.insert(value, shape, entity_type);
        }
    }

    fn relocate_in(
        &mut self,
        mut node: usize,
        value: u32,
        shape: ShapeEnum,
        entity_type: Option<u32>,
    ) {
        let bounding_box = shape.bounding_box();
        let root_node = self.root;
        loop {
            if collision_detection::rectangle_contains_rectangle(
                &self.nodes[node].bounding_box,
                &bounding_box,
            ) {
                let destination = self.get_destination_node_by_bbox(node, &bounding_box);
                if destination == node {
                    self.add(node, value, shape, entity_type);
                    return;
                }
                node = destination;
            } else if let Some(parent) = self.nodes[node].parent {
                node = parent;
            } else {
                self.add(root_node, value, shape, entity_type);
                self.clean_upwards(root_node);
                return;
            }
        }
    }

    fn count_all_items_limit(&self, node: usize, limit: usize) -> usize {
        let mut count = 0usize;
        let mut stack = Vec::with_capacity(16);
        stack.push(node);

        while let Some(current) = stack.pop() {
            let node_ref = &self.nodes[current];
            count += node_ref.entities.len();
            if count > limit {
                return count;
            }

            if node_ref.subdivided {
                for child in node_ref.children.as_array() {
                    if let Some(child_idx) = child {
                        stack.push(child_idx);
                    }
                }
            }
        }

        count
    }

    fn drain_subtree_items(&mut self, node: usize, items: &mut Vec<(u32, Entity)>) {
        let mut stack = Vec::with_capacity(16);
        stack.push(node);

        while let Some(current) = stack.pop() {
            let children = {
                let node_ref = &mut self.nodes[current];
                items.extend(node_ref.entities.drain());
                if node_ref.subdivided {
                    node_ref.children.as_array()
                } else {
                    [None; 4]
                }
            };

            for child in children {
                if let Some(child_idx) = child {
                    stack.push(child_idx);
                }
            }

            self.free_node(current);
        }
    }

    fn clean(&mut self, node: usize) {
        if !self.nodes[node].subdivided {
            return;
        }

        if self.count_all_items_limit(node, self.config.node_capacity) > self.config.node_capacity {
            return;
        }

        let children = {
            let node_ref = &mut self.nodes[node];
            let children = node_ref.children;
            node_ref.children = NodeChildren::none();
            node_ref.subdivided = false;
            children
        };

        let mut child_items = Vec::new();
        for child in children.as_array() {
            if let Some(child_idx) = child {
                self.drain_subtree_items(child_idx, &mut child_items);
            }
        }

        if !child_items.is_empty() {
            let node_ref = &mut self.nodes[node];
            for (value, entity) in child_items {
                self.owner_map.insert(value, node);
                node_ref.entities.insert(value, entity);
            }
        }
    }

    fn clean_upwards(&mut self, mut node: usize) {
        loop {
            self.clean(node);
            if let Some(parent) = self.nodes[node].parent {
                node = parent;
            } else {
                break;
            }
        }
    }

    pub fn all_node_bounding_boxes(&self, bounding_boxes: &mut Vec<Rectangle>) {
        let mut stack = Vec::with_capacity(32);
        stack.push(self.root);

        while let Some(node) = stack.pop() {
            let node_ref = &self.nodes[node];
            bounding_boxes.push(node_ref.bounding_box);

            if node_ref.subdivided {
                for child in node_ref.children.as_array() {
                    if let Some(child_idx) = child {
                        stack.push(child_idx);
                    }
                }
            }
        }
    }

    pub fn all_shapes(&self, shapes: &mut Vec<ShapeEnum>) {
        let mut stack = Vec::with_capacity(32);
        stack.push(self.root);

        while let Some(node) = stack.pop() {
            let node_ref = &self.nodes[node];
            for entity in node_ref.entities.values() {
                shapes.push(entity.shape.clone());
            }

            if node_ref.subdivided {
                for child in node_ref.children.as_array() {
                    if let Some(child_idx) = child {
                        stack.push(child_idx);
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub pool_size: usize,
    pub node_capacity: usize,
    pub max_depth: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            // With a max depth of 6, there could be up to 5461 nodes.
            // Let's set a reasonable max pool size of 4000.
            pool_size: 4000,
            node_capacity: 4,
            max_depth: 6,
        }
    }
}

#[derive(Clone)]
pub struct RelocationRequest {
    pub value: u32,
    pub shape: ShapeEnum,
    pub entity_type: Option<u32>,
}
