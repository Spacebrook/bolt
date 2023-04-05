use crate::collision_detection;
use crate::object_pool::{ObjectPool, Resettable};
use crate::shapes::{Rectangle, Shape, ShapeEnum};

use std::cell::Ref;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Deref;
use std::rc::Rc;
use std::rc::Weak;

#[derive(Clone)]
struct Entity {
    shape: ShapeEnum,
    entity_type: Option<u32>,
}

struct QuadNode {
    entities: HashMap<u32, Entity>,
    bounding_box: Rectangle,
    nw: Option<Rc<RefCell<QuadNode>>>,
    ne: Option<Rc<RefCell<QuadNode>>>,
    sw: Option<Rc<RefCell<QuadNode>>>,
    se: Option<Rc<RefCell<QuadNode>>>,
    parent: Option<Weak<RefCell<QuadNode>>>,
    subdivided: bool,
    depth: usize,
    self_rc: Option<Weak<RefCell<QuadNode>>>,
}

// Implement the Resettable trait for QuadNode
impl Resettable for QuadNode {
    fn reset(&mut self) {
        self.bounding_box = Rectangle::default();
        self.parent = None;
        self.depth = 0;
        self.entities.clear();
        self.nw = None;
        self.ne = None;
        self.sw = None;
        self.se = None;
        self.subdivided = false;
    }
}

impl QuadNode {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            bounding_box: Rectangle::default(),
            nw: None,
            ne: None,
            sw: None,
            se: None,
            parent: None,
            subdivided: false,
            depth: 0,
            self_rc: None,
        }
    }

    pub fn initialize(
        &mut self,
        bounding_box: Rectangle,
        parent: Option<Weak<RefCell<QuadNode>>>,
        depth: usize,
    ) {
        self.bounding_box = bounding_box;
        self.parent = parent;
        self.depth = depth;
        self.entities.clear();
        self.nw = None;
        self.ne = None;
        self.sw = None;
        self.se = None;
        self.subdivided = false;
    }

    // New method to initialize the self_rc field.
    pub fn set_self_rc(&mut self, self_rc: Weak<RefCell<QuadNode>>) {
        self.self_rc = Some(self_rc);
    }

    // Returns an iterator over all items in the QuadNode, including child nodes
    pub fn all_items(&self) -> Box<dyn Iterator<Item=(u32, Entity)> + '_> {
        let items = self.entities.iter().map(|(id, entity)| (*id, entity.clone()));
        if !self.subdivided {
            return Box::new(items);
        }

        let child_items = self.child_items();
        Box::new(items.chain(child_items))
    }

    // Returns an iterator over items in child nodes
    fn child_items(&self) -> Box<dyn Iterator<Item=(u32, Entity)> + '_> {
        if !self.subdivided {
            return Box::new(std::iter::empty());
        }

        let items: Vec<_> = [
            self.nw.as_ref(),
            self.ne.as_ref(),
            self.sw.as_ref(),
            self.se.as_ref(),
        ]
            .iter()
            .flat_map(|opt_node| {
                opt_node
                    .map(|node_rc| {
                        node_rc
                            .borrow()
                            .all_items()
                            .map(|(id, entity)| (id, entity.clone()))
                            .collect::<Vec<_>>()
                            .into_iter()
                    })
                    .into_iter()
                    .flatten()
            })
            .collect();

        Box::new(items.into_iter())
    }

    // Counts all items in the QuadNode, including child nodes
    pub fn count_all_items(&self) -> usize {
        let mut count = self.entities.len();
        if !self.subdivided {
            return count;
        }

        count += self
            .nw
            .as_ref()
            .map_or(0, |nw| nw.borrow().count_all_items());
        count += self
            .ne
            .as_ref()
            .map_or(0, |ne| ne.borrow().count_all_items());
        count += self
            .sw
            .as_ref()
            .map_or(0, |sw| sw.borrow().count_all_items());
        count += self
            .se
            .as_ref()
            .map_or(0, |se| se.borrow().count_all_items());
        return count;
    }
}

impl Default for QuadNode {
    fn default() -> Self {
        Self::new()
    }
}

pub struct QuadTree {
    root: Rc<RefCell<QuadNode>>,
    owner_map: HashMap<u32, Weak<RefCell<QuadNode>>>,
    quad_node_pool: ObjectPool<QuadNode>,

    config: Config,
}

impl QuadTree {
    pub fn new_with_config(bounding_box: Rectangle, config: Config) -> Self {
        let mut quad_node_pool = ObjectPool::<QuadNode>::new(config.pool_size);
        let root = Rc::new(RefCell::new(quad_node_pool.get()));
        root.borrow_mut().initialize(bounding_box, None, 0);
        root.borrow_mut().set_self_rc(Rc::downgrade(&root));

        let owner_map = HashMap::new();
        QuadTree {
            quad_node_pool,
            root,
            owner_map,
            config,
        }
    }

    pub fn new(bounding_box: Rectangle) -> Self {
        Self::new_with_config(bounding_box, Config::default())
    }

    // Insert a shape with a given value into the quadtree
    pub fn insert(&mut self, value: u32, shape: ShapeEnum, entity_type: Option<u32>) {
        self.insert_into(self.root.clone(), value, shape, entity_type);
    }

    // Insert a shape into a given node or its children
    fn insert_into(
        &mut self,
        mut node: Rc<RefCell<QuadNode>>,
        value: u32,
        shape: ShapeEnum,
        entity_type: Option<u32>,
    ) -> Rc<RefCell<QuadNode>> {
        loop {
            let mut need_subdivide = false;
            {
                let node_borrow = node.borrow_mut();

                // Check if node has room or reached max depth
                if (node_borrow.entities.len() < self.config.node_capacity && !node_borrow.subdivided)
                    || node_borrow.depth == self.config.max_depth
                {
                    drop(node_borrow);
                    self.add(&node, value, shape, entity_type);
                    return node.clone();
                }

                // Subdivide node if needed
                if !node_borrow.subdivided && node_borrow.depth < self.config.max_depth {
                    need_subdivide = true;
                } else {
                    let destination = self.get_destination_node(&node_borrow, shape.clone());
                    if Rc::ptr_eq(&destination, &node) {
                        drop(node_borrow);
                        self.add(&node, value, shape, entity_type);
                        return node.clone();
                    }

                    // Move to the next node for insertion
                    drop(node_borrow);
                    node = destination;
                }
            }

            // Perform subdivision outside the borrow scope
            if need_subdivide {
                self.subdivide(node.clone());
            }
        }
    }

    // Determine which child node the shape belongs to
    fn get_destination_node(&self, node: &QuadNode, shape: ShapeEnum) -> Rc<RefCell<QuadNode>> {
        if !node.subdivided {
            return node
                .self_rc
                .as_ref()
                .expect("Failed to upgrade Weak reference to Rc")
                .upgrade()
                .expect("Failed to upgrade Weak reference to Rc");
        }

        assert!(
            node.nw.is_some(),
            "nw should be set when subdivided is true"
        );
        assert!(
            node.ne.is_some(),
            "ne should be set when subdivided is true"
        );
        assert!(
            node.sw.is_some(),
            "sw should be set when subdivided is true"
        );
        assert!(
            node.se.is_some(),
            "se should be set when subdivided is true"
        );

        let bounding_box = shape.bounding_box();
        // Extract child nodes as references to avoid moving the values
        let (nw, ne, sw, se) = (&node.nw, &node.ne, &node.sw, &node.se);

        // Iterate over references to child nodes
        for child in &[nw, ne, sw, se] {
            if let Some(child_rc) = child.as_ref() {
                if collision_detection::rectangle_contains_rectangle(
                    &child_rc.borrow().bounding_box,
                    &bounding_box,
                ) {
                    return child_rc.clone();
                }
            }
        }

        node.self_rc
            .as_ref()
            .expect("Failed to upgrade Weak reference to Rc")
            .upgrade()
            .expect("Failed to upgrade Weak reference to Rc")
    }

    fn add(
        &mut self,
        node: &Rc<RefCell<QuadNode>>,
        value: u32,
        shape: ShapeEnum,
        entity_type: Option<u32>,
    ) {
        {
            // Limit the scope of the mutable borrow using a block
            let mut node_borrow_mut = node.borrow_mut();
            node_borrow_mut.entities.insert(value, Entity { shape, entity_type });
        }
        // The mutable borrow is released here
        self.owner_map.insert(value, Rc::downgrade(&node));
    }

    pub fn delete(&mut self, value: u32) {
        if let Some(node_weak) = self.owner_map.remove(&value) {
            let node_rc = node_weak
                .upgrade()
                .expect("Failed to upgrade Weak reference to Rc");
            self.delete_from(node_rc.clone(), value);
            // Clean up the node and its ancestors after deleting an item
            self.clean_upwards(node_rc);
        }
    }

    fn delete_from(&mut self, node: Rc<RefCell<QuadNode>>, value: u32) {
        // Remove the item from the QuadNode's items
        let mut node_borrow = node.borrow_mut();
        node_borrow.entities.remove(&value);
    }

    // Subdivide a node into quadrants
    fn subdivide(&mut self, node: Rc<RefCell<QuadNode>>) {
        let mut node_borrow = node.borrow_mut();

        let half_width = node_borrow.bounding_box.width / 2.0;
        let half_height = node_borrow.bounding_box.height / 2.0;

        // Compute coordinates for the new quadrants
        let nw_x = node_borrow.bounding_box.x;
        let nw_y = node_borrow.bounding_box.y;
        let ne_x = nw_x + half_width;
        let sw_y = nw_y + half_height;

        // Create a weak reference to the parent node
        let parent_weak = Rc::downgrade(&node);

        // Create new quadrants
        node_borrow.nw = Some(Rc::new(RefCell::new(self.quad_node_pool.get())));
        node_borrow.nw.as_ref().unwrap().borrow_mut().initialize(
            Rectangle {
                x: nw_x,
                y: nw_y,
                width: half_width,
                height: half_height,
            },
            Some(parent_weak.clone()),
            node_borrow.depth + 1,
        );
        node_borrow
            .nw
            .as_ref()
            .unwrap()
            .borrow_mut()
            .set_self_rc(Rc::downgrade(&node_borrow.nw.as_ref().unwrap()));

        node_borrow.ne = Some(Rc::new(RefCell::new(self.quad_node_pool.get())));
        node_borrow.ne.as_ref().unwrap().borrow_mut().initialize(
            Rectangle {
                x: ne_x,
                y: nw_y,
                width: half_width,
                height: half_height,
            },
            Some(parent_weak.clone()),
            node_borrow.depth + 1,
        );
        node_borrow
            .ne
            .as_ref()
            .unwrap()
            .borrow_mut()
            .set_self_rc(Rc::downgrade(&node_borrow.ne.as_ref().unwrap()));

        node_borrow.sw = Some(Rc::new(RefCell::new(self.quad_node_pool.get())));
        node_borrow.sw.as_ref().unwrap().borrow_mut().initialize(
            Rectangle {
                x: nw_x,
                y: sw_y,
                width: half_width,
                height: half_height,
            },
            Some(parent_weak.clone()),
            node_borrow.depth + 1,
        );
        node_borrow
            .sw
            .as_ref()
            .unwrap()
            .borrow_mut()
            .set_self_rc(Rc::downgrade(&node_borrow.sw.as_ref().unwrap()));

        node_borrow.se = Some(Rc::new(RefCell::new(self.quad_node_pool.get())));
        node_borrow.se.as_ref().unwrap().borrow_mut().initialize(
            Rectangle {
                x: ne_x,
                y: sw_y,
                width: half_width,
                height: half_height,
            },
            Some(parent_weak.clone()),
            node_borrow.depth + 1,
        );
        node_borrow
            .se
            .as_ref()
            .unwrap()
            .borrow_mut()
            .set_self_rc(Rc::downgrade(&node_borrow.se.as_ref().unwrap()));

        node_borrow.subdivided = true;

        // Redistribute the items to the appropriate quadrants
        let old_items = node_borrow.entities.drain().collect::<Vec<(u32, Entity)>>();
        drop(node_borrow);
        for (value, entity) in old_items {
            self.owner_map.remove(&value);
            self.insert_into(node.clone(), value, entity.shape, entity.entity_type);
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
        shapes
            .into_iter()
            .map(|shape| {
                let mut collisions = Vec::new();
                self.collisions_filter(shape, filter_entity_types.clone(), &mut collisions);
                collisions
            })
            .collect()
    }

    pub fn collisions(&self, shape: ShapeEnum, collisions: &mut Vec<u32>) {
        self.collisions_from(&self.root, &shape, None, collisions);
    }

    pub fn collisions_filter(
        &self,
        shape: ShapeEnum,
        filter_entity_types: Option<Vec<u32>>,
        collisions: &mut Vec<u32>,
    ) {
        self.collisions_from(&self.root, &shape, filter_entity_types, collisions);
    }

    // Find collisions with a given shape in the QuadTree
    // Helper method to recursively find collisions in the tree
    fn collisions_from(
        &self,
        node: &Rc<RefCell<QuadNode>>,
        query_shape: &ShapeEnum,
        filter_entity_types: Option<Vec<u32>>,
        collisions: &mut Vec<u32>,
    ) {
        // Compute the bounding box of the query shape
        let query_shape_bounding_box = query_shape.bounding_box();

        // Check for collisions with shapes in the current node
        let node_borrow = node.borrow();
        for (&value, entity) in node_borrow.entities.iter() {
            // Apply the entity type filter, if provided
            if let Some(filter) = &filter_entity_types {
                if let Some(entity_type) = entity.entity_type {
                    if !filter.contains(&entity_type) {
                        continue; // Skip items not matching the filter
                    }
                } else {
                    continue; // Skip items not found in the entity_types map
                }
            }

            if collision_detection::shape_shape(&query_shape, &entity.shape) {
                collisions.push(value);
            }
        }

        // Extract child nodes to avoid multiple borrows in pattern matching
        let (nw, ne, sw, se) = {
            let node_ref = node.borrow();
            (
                node_ref.nw.as_ref().map(|rc| rc.clone()),
                node_ref.ne.as_ref().map(|rc| rc.clone()),
                node_ref.sw.as_ref().map(|rc| rc.clone()),
                node_ref.se.as_ref().map(|rc| rc.clone()),
            )
        };

        // Continue with child nodes if the node has been subdivided
        if let (Some(nw), Some(ne), Some(sw), Some(se)) = (nw, ne, sw, se) {
            if collision_detection::rectangle_rectangle(
                &nw.borrow().bounding_box,
                &query_shape_bounding_box,
            ) {
                self.collisions_from(&nw, query_shape, filter_entity_types.clone(), collisions);
            }
            if collision_detection::rectangle_rectangle(
                &ne.borrow().bounding_box,
                &query_shape_bounding_box,
            ) {
                self.collisions_from(&ne, query_shape, filter_entity_types.clone(), collisions);
            }
            if collision_detection::rectangle_rectangle(
                &sw.borrow().bounding_box,
                &query_shape_bounding_box,
            ) {
                self.collisions_from(&sw, query_shape, filter_entity_types.clone(), collisions);
            }
            if collision_detection::rectangle_rectangle(
                &se.borrow().bounding_box,
                &query_shape_bounding_box,
            ) {
                self.collisions_from(&se, query_shape, filter_entity_types, collisions);
            }
        }
    }

    pub fn relocate_batch(&mut self, relocation_requests: Vec<RelocationRequest>) {
        for request in relocation_requests {
            self.relocate(request.value, request.shape, request.entity_type);
        }
    }

    pub fn relocate(&mut self, value: u32, shape: ShapeEnum, entity_type: Option<u32>) {
        if let Some(node_weak) = self.owner_map.get(&value) {
            let node = node_weak
                .upgrade()
                .expect("Failed to upgrade Weak reference to node");

            // Check if the item still fits in the current node
            let bounding_box = shape.bounding_box();
            if collision_detection::rectangle_contains_rectangle(
                &node.borrow().bounding_box,
                &bounding_box,
            ) {
                // Item is still in the correct node, no need to relocate
                self.add(&node, value, shape, entity_type);
                return;
            }

            // Delete the item from the current node and relocate to the appropriate node
            self.delete_from(node.clone(), value);
            self.relocate_in(node, value, shape, entity_type);
        } else {
            // If the object is not found in the owner_map, insert it into the quadtree
            self.insert(value, shape, entity_type);
        }
    }

    fn relocate_in(
        &mut self,
        mut node: Rc<RefCell<QuadNode>>,
        value: u32,
        shape: ShapeEnum,
        entity_type: Option<u32>,
    ) {
        let bounding_box = shape.bounding_box();
        let root_node = self.root.clone();
        loop {
            // Check if the shape fits within the current node's bounding box
            let node_bounding_box = node.borrow().bounding_box.clone();
            if collision_detection::rectangle_contains_rectangle(&node_bounding_box, &bounding_box)
            {
                // Find the appropriate child node or keep the current node
                let destination = self.get_destination_node(&node.borrow(), shape.clone());
                if Rc::ptr_eq(&destination, &node) {
                    self.add(&node, value, shape, entity_type);
                    return;
                }
                node = destination;
            } else {
                // Move up to the parent node
                let next_node = node
                    .borrow()
                    .parent
                    .as_ref()
                    .and_then(|weak| weak.upgrade());
                if let Some(parent) = next_node {
                    node = parent;
                } else {
                    // Item is outside the bounds of the QuadTree, add it to the root
                    self.add(&root_node, value, shape, entity_type);
                    // Clean up the root node and its ancestors
                    self.clean_upwards(root_node);
                    return;
                }
            }
        }
    }

    fn clean(&mut self, node: Rc<RefCell<QuadNode>>) {
        let should_collect_items = {
            let node_borrow = node.borrow();
            node_borrow.count_all_items() <= self.config.node_capacity
        };

        let child_items: Vec<_> = if should_collect_items {
            let node_borrow = node.borrow();
            node_borrow
                .child_items()
                .map(|(id, shape)| (id, shape.clone()))
                .collect()
        } else {
            Vec::new()
        };

        if !child_items.is_empty() {
            let mut node_borrow_mut = node.borrow_mut();
            for (value, shape) in child_items {
                self.owner_map.insert(value, Rc::downgrade(&node));
                node_borrow_mut.entities.insert(value, shape.clone());
            }

            // Helper function to return the child node to the object pool
            fn return_child_to_pool(
                pool: &mut ObjectPool<QuadNode>,
                child: Option<Rc<RefCell<QuadNode>>>,
            ) {
                if let Some(child_rc) = child {
                    if let Ok(node) = Rc::try_unwrap(child_rc) {
                        pool.return_object(node.into_inner());
                    }
                }
            }

            // Return the child nodes to the object pool
            return_child_to_pool(&mut self.quad_node_pool, node_borrow_mut.nw.take());
            return_child_to_pool(&mut self.quad_node_pool, node_borrow_mut.ne.take());
            return_child_to_pool(&mut self.quad_node_pool, node_borrow_mut.sw.take());
            return_child_to_pool(&mut self.quad_node_pool, node_borrow_mut.se.take());

            // Set subdivided flag to false as all child nodes are removed
            node_borrow_mut.subdivided = false;
        }
    }

    // Clean up the QuadNode and its ancestors
    fn clean_upwards(&mut self, mut node: Rc<RefCell<QuadNode>>) {
        loop {
            self.clean(node.clone());
            let next_node = {
                let node_borrow = node.borrow();
                if let Some(parent_weak) = node_borrow.parent.as_ref() {
                    // Move to the parent node for the next iteration
                    parent_weak.upgrade()
                } else {
                    // No more parent nodes, break out of the loop
                    None
                }
            };
            if let Some(parent) = next_node {
                node = parent;
            } else {
                break;
            }
        }
    }

    // Retrieve all node bounding boxes from the QuadTree
    pub fn all_node_bounding_boxes(&self, bounding_boxes: &mut Vec<Rectangle>) {
        self.node_bounding_boxes(&self.root, bounding_boxes);
    }

    // Helper method to recursively retrieve node bounding boxes
    fn node_bounding_boxes(
        &self,
        node: &Rc<RefCell<QuadNode>>,
        bounding_boxes: &mut Vec<Rectangle>,
    ) {
        // Add the bounding box of the current node to the list
        bounding_boxes.push(node.borrow().bounding_box);

        // Extract child nodes to avoid multiple borrows in pattern matching
        let (nw, ne, sw, se) = {
            let node_ref = node.borrow();
            (
                node_ref.nw.as_ref().map(|rc| rc.clone()),
                node_ref.ne.as_ref().map(|rc| rc.clone()),
                node_ref.sw.as_ref().map(|rc| rc.clone()),
                node_ref.se.as_ref().map(|rc| rc.clone()),
            )
        };

        // Continue with child nodes if the node has been subdivided
        if let (Some(nw), Some(ne), Some(sw), Some(se)) = (nw, ne, sw, se) {
            self.node_bounding_boxes(&nw, bounding_boxes);
            self.node_bounding_boxes(&ne, bounding_boxes);
            self.node_bounding_boxes(&sw, bounding_boxes);
            self.node_bounding_boxes(&se, bounding_boxes);
        }
    }

    // Retrieve all shapes from the QuadTree
    pub fn all_shapes(&self, shapes: &mut Vec<ShapeEnum>) {
        self.shapes(&self.root, shapes);
    }

    // Helper method to recursively retrieve shapes
    fn shapes(&self, node: &Rc<RefCell<QuadNode>>, shapes: &mut Vec<ShapeEnum>) {
        let node_ref: Ref<QuadNode> = node.as_ref().borrow();
        // Add the shapes in the current node to the list
        for (_, entities) in node_ref.deref().entities.iter() {
            shapes.push(entities.shape.clone());
        }
        // Extract child nodes to avoid multiple borrows in pattern matching
        let (nw, ne, sw, se) = (
            node_ref.nw.clone(),
            node_ref.ne.clone(),
            node_ref.sw.clone(),
            node_ref.se.clone(),
        );

        // Explicitly drop the borrow so that we can borrow again in the recursive calls
        drop(node_ref);

        // Continue with child nodes if the node has been subdivided
        if let (Some(nw), Some(ne), Some(sw), Some(se)) = (nw, ne, sw, se) {
            self.shapes(&nw, shapes);
            self.shapes(&ne, shapes);
            self.shapes(&sw, shapes);
            self.shapes(&se, shapes);
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub pool_size: usize,
    pub node_capacity: usize,
    pub max_depth: usize,
}

// Implement Default trait for Config
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
