#[cfg(test)]
extern crate rand;

use std::collections::HashMap;

mod pool;


#[cfg(test)]
mod test;

pub struct QuadTree {
    root: usize,
    nodes: Vec<QuadNode>,
    node_capacity: usize,
    index_pool: pool::IntegerPool,
    id_to_node_index: HashMap<usize, usize>,
}

pub struct QuadNode {
    items: HashMap<usize, Shape>,
    bb: Rectangle,
    nw: Option<usize>,
    ne: Option<usize>,
    sw: Option<usize>,
    se: Option<usize>,
    subdivided: bool,
}

impl QuadNode {
    fn new(bb: Rectangle) -> QuadNode {
        QuadNode {
            items: HashMap::new(),
            bb,
            nw: None,
            ne: None,
            sw: None,
            se: None,
            subdivided: false,
        }
    }
}

impl QuadTree {
    pub fn new(bb: Rectangle) -> QuadTree {
        let mut index_pool = pool::IntegerPool::new(1);
        let root_index = index_pool.take();
        QuadTree {
            root: root_index,
            nodes: vec![QuadNode::new(bb)],
            node_capacity: 1,
            index_pool,
            id_to_node_index: HashMap::new(),
        }
    }

    pub fn insert(&mut self, id: usize, shape: Shape) {
        let node_index = self.root;
        self.insert_into(node_index, id, shape);
    }

    fn insert_into(&mut self, node_index: usize, id: usize, shape: Shape) {
        // TODO: Better way of doing this? Immutable & mutable problem.
        let node_bb = self.nodes[node_index].bb;
        let node_items_len = self.nodes[node_index].items.len();
        let node_subdivided = self.nodes[node_index].subdivided;

        let shape_bb = shape.bb();
        if !rectangle_rectangle_collision(node_bb, shape_bb) {
            return;
        }

        if node_items_len > self.node_capacity && !node_subdivided {
            self.subdivide_node(node_index);
        }

        let mut insert_into_this_node = true;
        if node_subdivided {
            let nw_index = self.nodes[node_index].nw.unwrap();
            let ne_index = self.nodes[node_index].ne.unwrap();
            let sw_index = self.nodes[node_index].sw.unwrap();
            let se_index = self.nodes[node_index].se.unwrap();
            let nw_bb = self.nodes[nw_index].bb;
            let ne_bb = self.nodes[ne_index].bb;
            let sw_bb = self.nodes[sw_index].bb;
            let se_bb = self.nodes[se_index].bb;

            insert_into_this_node = false;
            if rectangle_rectangle_collision(shape_bb, nw_bb) {
                if rectangle_rectangle_collision(shape_bb, ne_bb) {
                    insert_into_this_node = true;
                } else if rectangle_rectangle_collision(shape_bb, sw_bb) {
                    insert_into_this_node = true;
                } else if rectangle_rectangle_collision(shape_bb, se_bb) {
                    insert_into_this_node = true;
                } else {
                    self.insert_into(nw_index, id, shape);
                }
            } else if rectangle_rectangle_collision(shape_bb, ne_bb) {
                if rectangle_rectangle_collision(shape_bb, sw_bb) {
                    insert_into_this_node = true;
                } else if rectangle_rectangle_collision(shape_bb, se_bb) {
                    insert_into_this_node = true;
                } else {
                    self.insert_into(ne_index, id, shape);
                }
            } else if rectangle_rectangle_collision(shape_bb, sw_bb) {
                if rectangle_rectangle_collision(shape_bb, se_bb) {
                    insert_into_this_node = true;
                } else {
                    self.insert_into(sw_index, id, shape);
                }
            } else if rectangle_rectangle_collision(shape_bb, se_bb) {
                self.insert_into(se_index, id, shape);
            }
        }

        if insert_into_this_node {
            let node = &mut self.nodes[node_index];
            self.id_to_node_index.insert(id, node_index);
            node.items.insert(id, shape);
        }
    }

    fn subdivide_node(&mut self, node_index: usize) {
        let node_bb = self.nodes[node_index].bb;
        let half_width = node_bb.width / 2;
        let half_height = node_bb.height / 2;

        let nw = self.add_new_node(Rectangle {
            x: node_bb.x,
            y: node_bb.y,
            width: half_width,
            height: half_height,
        });
        let ne = self.add_new_node(Rectangle {
            x: node_bb.x + half_width,
            y: node_bb.y,
            width: half_width,
            height: half_height,
        });
        let sw = self.add_new_node(Rectangle {
            x: node_bb.x,
            y: node_bb.y + half_height,
            width: half_width,
            height: half_height,
        });
        let se = self.add_new_node(Rectangle {
            x: node_bb.x + half_width,
            y: node_bb.y + half_height,
            width: half_width,
            height: half_height,
        });

        let old_items;
        {
            let node = &mut self.nodes[node_index];
            node.nw = Some(nw);
            node.ne = Some(ne);
            node.sw = Some(sw);
            node.se = Some(se);
            node.subdivided = true;

            old_items = std::mem::replace(&mut node.items, HashMap::new());
        }

        for (id, shape) in old_items {
            self.insert_into(node_index, id, shape);
        }
    }

    fn add_new_node(&mut self, bb: Rectangle) -> usize {
        let index = self.index_pool.take();

        let quadnode = QuadNode::new(bb);
        if index == self.nodes.len() {
            self.nodes.push(quadnode);
        } else if index < self.nodes.len() {
            self.nodes[index] = quadnode;
        } else {
            panic!("Index from pool is out of bounds ({})", index);
        }

        index
    }

    pub fn collisions(&self, shape: Shape) -> Vec<usize> {
        self.collisions_from(self.root, shape)
    }

    fn collisions_from(&self, node_index: usize, query_shape: Shape) -> Vec<usize> {
        let node = &self.nodes[node_index];
        let node_items = &self.nodes[node_index].items;
        let mut collisions = Vec::new();
        for (id, shape) in node_items {
            match query_shape {
                Shape::Circle(query_circle) => {
                    match shape {
                        Shape::Circle(circle) => {
                            if circle_circle_collision(query_circle, *circle) {
                                collisions.push(*id);
                            }
                        }
                        Shape::Rectangle(rectangle) => {
                            if circle_rectangle_collision(query_circle, *rectangle) {
                                collisions.push(*id);
                            }
                        }
                    }
                }
                Shape::Rectangle(query_rectangle) => {
                    match shape {
                        Shape::Circle(circle) => {
                            if circle_rectangle_collision(*circle, query_rectangle) {
                                collisions.push(*id);
                            }
                        }
                        Shape::Rectangle(rectangle) => {
                            if rectangle_rectangle_collision(query_rectangle, *rectangle) {
                                collisions.push(*id);
                            }
                        }
                    }
                }
            }
        }

        if node.subdivided {
            let nw_index = node.nw.unwrap();
            let ne_index = node.ne.unwrap();
            let sw_index = node.sw.unwrap();
            let se_index = node.se.unwrap();
            let nw = &self.nodes[nw_index];
            let ne = &self.nodes[ne_index];
            let sw = &self.nodes[sw_index];
            let se = &self.nodes[se_index];

            let query_shape_bb = query_shape.bb();

            if rectangle_rectangle_collision(query_shape_bb, nw.bb) {
                collisions.append(&mut self.collisions_from(nw_index, query_shape));
            }

            if rectangle_rectangle_collision(query_shape_bb, ne.bb) {
                collisions.append(&mut self.collisions_from(ne_index, query_shape));
            }

            if rectangle_rectangle_collision(query_shape_bb, sw.bb) {
                collisions.append(&mut self.collisions_from(sw_index, query_shape));
            }

            if rectangle_rectangle_collision(query_shape_bb, se.bb) {
                collisions.append(&mut self.collisions_from(se_index, query_shape));
            }
        }

        collisions
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Shape {
    Circle(Circle),
    Rectangle(Rectangle),
}

#[derive(Copy, Clone, Debug)]
pub struct Circle {
    pub x: i32,
    pub y: i32,
    pub radius: i32,
}

#[derive(Copy, Clone, Debug)]
pub struct Rectangle {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Shape {
    fn bb(&self) -> Rectangle {
        match self {
            Shape::Circle(circle) => {
                Rectangle {
                    x: circle.x - circle.radius,
                    y: circle.y - circle.radius,
                    width: circle.radius * 2,
                    height: circle.radius * 2,
                }
            }
            Shape::Rectangle(rectangle) => {
                *rectangle
            }
        }
    }
}

impl Rectangle {
    fn right(&self) -> i32 {
        self.x + self.width
    }
    fn bottom(&self) -> i32 {
        self.y + self.height
    }
}

fn circle_circle_collision(circle_a: Circle, circle_b: Circle) -> bool {
    let collision_distance = circle_a.radius + circle_b.radius;
    let distance_x = circle_a.x - circle_b.x;
    let distance_y = circle_a.y - circle_b.y;
    distance_x.pow(2) + distance_y.pow(2) < collision_distance.pow(2)
}

fn rectangle_rectangle_collision(rectangle_a: Rectangle, rectangle_b: Rectangle) -> bool {
    rectangle_a.x < rectangle_b.right() &&
        rectangle_a.right() > rectangle_b.x &&
        rectangle_a.y < rectangle_b.bottom() &&
        rectangle_a.bottom() > rectangle_b.y
}

fn circle_rectangle_collision(circle: Circle, rectangle: Rectangle) -> bool {
    let rect_half_width = rectangle.width / 2;
    let rect_half_height = rectangle.height / 2;

    let distance_x = (circle.x - rectangle.x - rect_half_width).abs();
    let distance_y = (circle.y - rectangle.y - rect_half_height).abs();

    if distance_x > rect_half_width + circle.radius {
        return false;
    }
    if distance_y > rect_half_height + circle.radius {
        return false;
    }

    if distance_x < rect_half_width {
        return true;
    }
    if distance_y < rect_half_height {
        return true;
    }

    let center_rect_distance_x = distance_x - rect_half_width;
    let center_rect_distance_y = distance_y - rect_half_height;

    center_rect_distance_x.pow(2) + center_rect_distance_y.pow(2) < circle.radius.pow(2)
}
