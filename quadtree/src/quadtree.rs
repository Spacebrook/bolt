use crate::collision_detection;
use crate::object_pool::{ObjectPool, Resettable};
use crate::shapes::{Rectangle, Shape};

use std::cell::Ref;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Deref;
use std::rc::Rc;
use std::rc::Weak;

struct QuadNode {
    items: HashMap<u32, Box<dyn Shape>>,
    bounding_box: Rectangle,
    nw: Option<Rc<RefCell<QuadNode>>>,
    ne: Option<Rc<RefCell<QuadNode>>>,
    sw: Option<Rc<RefCell<QuadNode>>>,
    se: Option<Rc<RefCell<QuadNode>>>,
    parent: Option<Weak<RefCell<QuadNode>>>,
    subdivided: bool,
    depth: i32,
}

impl QuadNode {
    const CAPACITY: usize = 4;
    const MAX_DEPTH: i32 = 6;

    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
            bounding_box: Rectangle::default(),
            nw: None,
            ne: None,
            sw: None,
            se: None,
            parent: None,
            subdivided: false,
            depth: 0,
        }
    }

    pub fn initialize(
        &mut self,
        bounding_box: Rectangle,
        parent: Option<Weak<RefCell<QuadNode>>>,
        depth: i32,
    ) {
        self.bounding_box = bounding_box;
        self.parent = parent;
        self.depth = depth;
        self.items.clear();
        self.nw = None;
        self.ne = None;
        self.sw = None;
        self.se = None;
        self.subdivided = false;
    }

    // Returns an iterator over all items in the QuadNode, including child nodes
    pub fn all_items(&self) -> Box<dyn Iterator<Item = (u32, Box<dyn Shape>)> + '_> {
        let items = self
            .items
            .iter()
            .map(|(id, shape)| (*id, shape.box_clone()));
        if !self.subdivided {
            return Box::new(items);
        }

        let child_items = self.child_items();
        Box::new(items.chain(child_items))
    }

    // Returns an iterator over items in child nodes
    fn child_items(&self) -> Box<dyn Iterator<Item = (u32, Box<dyn Shape>)> + '_> {
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
                        .map(|(id, shape)| (id, shape.box_clone()))
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
        let mut count = self.items.len();
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
}

impl QuadTree {
    pub fn new(bounding_box: Rectangle) -> Self {
        // With a max depth of 6, there could be up to 5461 nodes.
        // Let's set a reasonable max pool size of 4000.
        let mut quad_node_pool = ObjectPool::<QuadNode>::new(4000);
        let root = Rc::new(RefCell::new(quad_node_pool.get()));
        root.borrow_mut().initialize(bounding_box, None, 0);

        let owner_map = HashMap::new();
        QuadTree {
            quad_node_pool,
            root,
            owner_map,
        }
    }

    // Insert a shape with a given value into the quadtree
    pub fn insert(&mut self, value: u32, shape: Box<dyn Shape>) {
        self.insert_into(self.root.clone(), value, shape);
    }

    // Insert a shape into a given node or its children
    fn insert_into(
        &mut self,
        mut node: Rc<RefCell<QuadNode>>,
        value: u32,
        shape: Box<dyn Shape>,
    ) -> Rc<RefCell<QuadNode>> {
        loop {
            let mut need_subdivide = false;
            {
                let node_borrow = node.borrow_mut();

                // Check if node has room or reached max depth
                if (node_borrow.items.len() < QuadNode::CAPACITY && !node_borrow.subdivided)
                    || node_borrow.depth == QuadNode::MAX_DEPTH
                {
                    drop(node_borrow);
                    self.add(&node, value, shape.box_clone());
                    return node.clone();
                }

                // Subdivide node if needed
                if !node_borrow.subdivided && node_borrow.depth < QuadNode::MAX_DEPTH {
                    need_subdivide = true;
                } else {
                    // Determine which child node the shape belongs to
                    let subdivided = node_borrow.subdivided;
                    let nw = node_borrow.nw.clone();
                    let ne = node_borrow.ne.clone();
                    let sw = node_borrow.sw.clone();
                    let se = node_borrow.se.clone();

                    let destination =
                        self.get_destination_node(&node, subdivided, nw, ne, sw, se, &*shape);
                    if Rc::ptr_eq(&destination, &node) {
                        drop(node_borrow);
                        self.add(&node, value, shape.box_clone());
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
    fn get_destination_node(
        &self,
        node: &Rc<RefCell<QuadNode>>,
        subdivided: bool,
        nw: Option<Rc<RefCell<QuadNode>>>,
        ne: Option<Rc<RefCell<QuadNode>>>,
        sw: Option<Rc<RefCell<QuadNode>>>,
        se: Option<Rc<RefCell<QuadNode>>>,
        shape: &dyn Shape,
    ) -> Rc<RefCell<QuadNode>> {
        if !subdivided {
            return node.clone();
        }

        assert!(nw.is_some(), "nw should be set when subdivided is true");
        assert!(ne.is_some(), "ne should be set when subdivided is true");
        assert!(sw.is_some(), "sw should be set when subdivided is true");
        assert!(se.is_some(), "se should be set when subdivided is true");

        let bounding_box = shape.bounding_box();
        if collision_detection::rectangle_contains_rectangle(
            &nw.as_ref().unwrap().borrow().bounding_box,
            &bounding_box,
        ) {
            return nw.unwrap();
        }
        if collision_detection::rectangle_contains_rectangle(
            &ne.as_ref().unwrap().borrow().bounding_box,
            &bounding_box,
        ) {
            return ne.unwrap();
        }
        if collision_detection::rectangle_contains_rectangle(
            &sw.as_ref().unwrap().borrow().bounding_box,
            &bounding_box,
        ) {
            return sw.unwrap();
        }
        if collision_detection::rectangle_contains_rectangle(
            &se.as_ref().unwrap().borrow().bounding_box,
            &bounding_box,
        ) {
            return se.unwrap();
        }

        node.clone()
    }

    fn add(&mut self, node: &Rc<RefCell<QuadNode>>, value: u32, shape: Box<dyn Shape>) {
        {
            // Limit the scope of the mutable borrow using a block
            let mut node_borrow_mut = node.borrow_mut();
            node_borrow_mut.items.insert(value, shape);
        }
        // The mutable borrow is released here
        self.owner_map.insert(value, Rc::downgrade(&node));
    }

    pub fn delete(&mut self, value: u32) {
        if let Some(node_weak) = self.owner_map.remove(&value) {
            let node_rc = node_weak
                .upgrade()
                .expect("Failed to upgrade Weak reference to Rc");
            self.delete_from(node_rc, value);
        }
    }

    fn delete_from(&mut self, node: Rc<RefCell<QuadNode>>, value: u32) {
        // Remove the item from the QuadNode's items
        let mut node_borrow = node.borrow_mut();
        node_borrow
            .items
            .retain(|item_value, _| *item_value != value);
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

        node_borrow.subdivided = true;

        // Redistribute the items to the appropriate quadrants
        let old_items = node_borrow
            .items
            .drain()
            .collect::<Vec<(u32, Box<dyn Shape>)>>();
        drop(node_borrow);
        for (value, shape) in old_items {
            self.owner_map.remove(&value);
            self.insert_into(node.clone(), value, shape);
        }
    }

    pub fn collisions(&self, shape: &dyn Shape, collisions: &mut Vec<u32>) {
        self.collisions_from(&self.root, shape, collisions);
    }

    // Find collisions with a given shape in the QuadTree
    // Helper method to recursively find collisions in the tree
    fn collisions_from(
        &self,
        node: &Rc<RefCell<QuadNode>>,
        query_shape: &dyn Shape,
        collisions: &mut Vec<u32>,
    ) {
        // Compute the bounding box of the query shape
        let query_shape_bounding_box = query_shape.bounding_box();

        // Check for collisions with shapes in the current node
        for (&value, shape) in node.borrow().items.iter() {
            if collision_detection::shape_shape(query_shape, shape.as_ref()) {
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
                self.collisions_from(&nw, query_shape, collisions);
            }
            if collision_detection::rectangle_rectangle(
                &ne.borrow().bounding_box,
                &query_shape_bounding_box,
            ) {
                self.collisions_from(&ne, query_shape, collisions);
            }
            if collision_detection::rectangle_rectangle(
                &sw.borrow().bounding_box,
                &query_shape_bounding_box,
            ) {
                self.collisions_from(&sw, query_shape, collisions);
            }
            if collision_detection::rectangle_rectangle(
                &se.borrow().bounding_box,
                &query_shape_bounding_box,
            ) {
                self.collisions_from(&se, query_shape, collisions);
            }
        }
    }

    pub fn relocate(&mut self, value: u32, shape: Box<dyn Shape>) {
        if let Some(node_weak) = self.owner_map.get(&value) {
            let node = node_weak
                .upgrade()
                .expect("Failed to upgrade Weak reference to node");
            self.delete_from(node.clone(), value);
            self.relocate_in(node, value, shape);
        } else {
            self.insert(value, shape);
        }
    }

    fn relocate_in(&mut self, node: Rc<RefCell<QuadNode>>, value: u32, shape: Box<dyn Shape>) {
        let bounding_box = shape.bounding_box();
        let node_borrow = node.borrow();
        if collision_detection::rectangle_contains_rectangle(
            &node_borrow.bounding_box,
            &bounding_box,
        ) {
            // Check if the item belongs to one of the child nodes (if they exist)
            let child = self.get_destination_node(
                &node,
                node_borrow.subdivided,
                node_borrow.nw.clone(),
                node_borrow.ne.clone(),
                node_borrow.sw.clone(),
                node_borrow.se.clone(),
                &*shape,
            );
            if !Rc::ptr_eq(&child, &node) {
                // Add the item to the child node
                self.add(&child, value, shape.box_clone());
                return;
            }

            // Add the item to the current node
            drop(node_borrow);
            self.add(&node, value, shape.box_clone());
            return;
        }

        if let Some(parent_weak) = node_borrow.parent.as_ref() {
            let parent = parent_weak
                .upgrade()
                .expect("Failed to upgrade Weak reference to parent");
            drop(node_borrow);
            self.relocate_in(parent, value, shape);
        } else {
            node.borrow_mut().items.insert(value, shape);
            self.clean_upwards(node.clone());
        }
    }

    fn clean(&mut self, node: Rc<RefCell<QuadNode>>) {
        let child_items: Vec<_> = {
            let node_borrow = node.borrow();
            if node_borrow.count_all_items() <= QuadNode::CAPACITY {
                node_borrow.child_items().collect()
            } else {
                Vec::new()
            }
        };

        if !child_items.is_empty() {
            let mut node_borrow_mut = node.borrow_mut();
            for (value, shape) in child_items {
                self.owner_map.insert(value, Rc::downgrade(&node));
                node_borrow_mut.items.insert(value, shape);
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
    pub fn all_shapes(&self, shapes: &mut Vec<Box<dyn Shape>>) {
        self.shapes(&self.root, shapes);
    }

    // Helper method to recursively retrieve shapes
    fn shapes(&self, node: &Rc<RefCell<QuadNode>>, shapes: &mut Vec<Box<dyn Shape>>) {
        let node_ref: Ref<QuadNode> = node.as_ref().borrow();
        // Add the shapes in the current node to the list
        for (_, shape) in node_ref.deref().items.iter() {
            shapes.push(shape.box_clone());
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

// Implement the Resettable trait for QuadNode
impl Resettable for QuadNode {
    fn reset(&mut self) {
        self.bounding_box = Rectangle::default();
        self.parent = None;
        self.depth = 0;
        self.items.clear();
        self.nw = None;
        self.ne = None;
        self.sw = None;
        self.se = None;
        self.subdivided = false;
    }
}
