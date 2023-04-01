use quadtree::quadtree::QuadTree;
use quadtree::shapes::{Circle, Rectangle};

use rand::Rng;
use std::collections::HashSet;

#[test]
fn test_single_collision() {
    let mut qt = QuadTree::new(Rectangle::new(0.0, 0.0, 100.0, 100.0));
    qt.insert(0, Box::new(Rectangle::new(0.0, 15.0, 100.0, 50.0)));
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(&Rectangle::new(0.0, 0.0, 20.0, 20.0), &mut collisions);
    assert_eq!(collisions, vec![0]);
}

#[test]
fn test_full_tree() {
    let mut qt = QuadTree::new(Rectangle::new(0.0, 0.0, 1000.0, 1000.0));
    qt.insert(0, Box::new(Rectangle::new(500.0, 500.0, 50.0, 50.0)));
    qt.insert(1, Box::new(Circle::new(500.0, 500.0, 25.0)));

    let mut rng = rand::thread_rng();
    for i in 2..5 {
        qt.insert(
            i,
            Box::new(Rectangle::new(
                rng.gen_range(0.0..900.0),
                rng.gen_range(0.0..900.0),
                rng.gen_range(0.0..100.0),
                rng.gen_range(0.0..100.0),
            )),
        );
    }

    for i in 5..8 {
        qt.insert(
            i,
            Box::new(Circle::new(
                rng.gen_range(0.0..950.0),
                rng.gen_range(0.0..950.0),
                rng.gen_range(0.0..50.0),
            )),
        );
    }

    // Print out information about the quadtree structure and its contents
    let mut bounding_boxes = Vec::new();
    qt.all_node_bounding_boxes(&mut bounding_boxes);
    println!("All Node Bounding Boxes: {:?}", bounding_boxes);

    let mut shapes = Vec::new();
    qt.all_shapes(&mut shapes);
    println!("All Shapes: {:?}", shapes);

    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(&Rectangle::new(500.0, 500.0, 1.0, 1.0), &mut collisions);
    assert!(collisions.contains(&0));
    assert!(collisions.contains(&1));

    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(&Circle::new(500.0, 500.0, 1.0), &mut collisions);
    assert!(collisions.contains(&0));
    assert!(collisions.contains(&1));
}

#[test]
fn test_huge_bounds() {
    let bounds = Rectangle::new(-1000000.0, -1000000.0, 2000000.0, 2000000.0);
    let mut qt = QuadTree::new(bounds);
    qt.insert(0, Box::new(Rectangle::new(16000.0, -355.0, 60.0, 60.0)));
    qt.insert(1, Box::new(Rectangle::new(15980.0, -350.0, 60.0, 60.0)));
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(
        &Rectangle::new(15980.0, -350.0, 60.0, 60.0),
        &mut collisions,
    );
    let collision_set: HashSet<_> = collisions.into_iter().collect();
    assert_eq!(collision_set.len(), 2);
    assert!(collision_set.contains(&0));
    assert!(collision_set.contains(&1));
}

#[test]
fn test_no_collision() {
    // Test case where there are no collisions
    let mut qt = QuadTree::new(Rectangle::new(0.0, 0.0, 100.0, 100.0));
    qt.insert(0, Box::new(Rectangle::new(10.0, 10.0, 20.0, 20.0)));
    qt.insert(1, Box::new(Rectangle::new(50.0, 50.0, 20.0, 20.0)));
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(&Rectangle::new(80.0, 80.0, 10.0, 10.0), &mut collisions);
    assert!(collisions.is_empty());
}

#[test]
fn test_multiple_collisions() {
    // Test case where a query shape collides with multiple objects in the quadtree
    let mut qt = QuadTree::new(Rectangle::new(0.0, 0.0, 100.0, 100.0));
    qt.insert(0, Box::new(Rectangle::new(10.0, 10.0, 20.0, 20.0)));
    qt.insert(1, Box::new(Rectangle::new(20.0, 20.0, 30.0, 30.0)));
    qt.insert(2, Box::new(Rectangle::new(15.0, 15.0, 15.0, 15.0)));
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(&Rectangle::new(15.0, 15.0, 20.0, 20.0), &mut collisions);
    let collision_set: HashSet<_> = collisions.into_iter().collect();
    assert_eq!(collision_set.len(), 3);
    assert!(collision_set.contains(&0));
    assert!(collision_set.contains(&1));
    assert!(collision_set.contains(&2));
}

#[test]
fn test_object_relocation() {
    // Test case where an object is relocated within the quadtree
    let mut qt = QuadTree::new(Rectangle::new(0.0, 0.0, 100.0, 100.0));
    qt.insert(0, Box::new(Rectangle::new(10.0, 10.0, 10.0, 10.0)));
    qt.relocate(0, Box::new(Rectangle::new(60.0, 60.0, 10.0, 10.0)));
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(&Rectangle::new(60.0, 60.0, 10.0, 10.0), &mut collisions);
    assert_eq!(collisions, vec![0]);
}

#[test]
fn test_object_deletion() {
    // Test case where an object is deleted from the quadtree
    let mut qt = QuadTree::new(Rectangle::new(0.0, 0.0, 100.0, 100.0));
    qt.insert(0, Box::new(Rectangle::new(10.0, 10.0, 10.0, 10.0)));
    qt.insert(1, Box::new(Rectangle::new(50.0, 50.0, 10.0, 10.0)));
    qt.delete(0);
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(&Rectangle::new(10.0, 10.0, 10.0, 10.0), &mut collisions);
    assert!(!collisions.contains(&0));
}

#[test]
fn test_object_out_of_bounds() {
    // Test case where an object is outside the bounds of the quadtree
    let mut qt = QuadTree::new(Rectangle::new(0.0, 0.0, 100.0, 100.0));
    qt.insert(0, Box::new(Rectangle::new(150.0, 150.0, 10.0, 10.0)));
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(&Rectangle::new(150.0, 150.0, 10.0, 10.0), &mut collisions);
    assert_eq!(collisions, vec![0]);
}

#[test]
fn test_empty_quad_tree() {
    // Test case where the quadtree is empty
    let qt = QuadTree::new(Rectangle::new(0.0, 0.0, 100.0, 100.0));
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(&Rectangle::new(10.0, 10.0, 10.0, 10.0), &mut collisions);
    assert!(collisions.is_empty());
}

#[test]
fn test_query_with_large_shape() {
    // Test case where a query shape is large and intersects multiple nodes
    let mut qt = QuadTree::new(Rectangle::new(0.0, 0.0, 100.0, 100.0));
    qt.insert(0, Box::new(Rectangle::new(10.0, 10.0, 10.0, 10.0)));
    qt.insert(1, Box::new(Rectangle::new(50.0, 50.0, 10.0, 10.0)));
    qt.insert(2, Box::new(Rectangle::new(70.0, 70.0, 10.0, 10.0)));
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(&Rectangle::new(0.0, 0.0, 100.0, 100.0), &mut collisions);
    let collision_set: HashSet<_> = collisions.into_iter().collect();
    assert_eq!(collision_set.len(), 3);
    assert!(collision_set.contains(&0));
    assert!(collision_set.contains(&1));
    assert!(collision_set.contains(&2));
}

#[test]
fn test_boundary_collision() {
    // Test case where a query shape is positioned on the boundary of another shape
    let mut qt = QuadTree::new(Rectangle::new(0.0, 0.0, 100.0, 100.0));
    qt.insert(0, Box::new(Rectangle::new(10.0, 10.0, 20.0, 20.0)));
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(&Rectangle::new(30.0, 10.0, 10.0, 10.0), &mut collisions);
    assert!(collisions.is_empty());
}

#[test]
fn test_shape_spanning_multiple_quadrants() {
    // Test case where a shape spans multiple quadrants
    let mut qt = QuadTree::new(Rectangle::new(0.0, 0.0, 100.0, 100.0));
    qt.insert(0, Box::new(Rectangle::new(45.0, 45.0, 10.0, 10.0)));
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(&Rectangle::new(48.0, 48.0, 2.0, 2.0), &mut collisions);
    assert_eq!(collisions, vec![0]);
}

#[test]
fn test_object_insertion_with_same_key() {
    // Test case where an object is inserted with the same key as an existing object
    let mut qt = QuadTree::new(Rectangle::new(0.0, 0.0, 100.0, 100.0));
    qt.insert(0, Box::new(Rectangle::new(10.0, 10.0, 10.0, 10.0)));
    qt.insert(0, Box::new(Rectangle::new(60.0, 60.0, 10.0, 10.0))); // Same key as the first object
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(&Rectangle::new(10.0, 10.0, 10.0, 10.0), &mut collisions);
    assert!(!collisions.contains(&0)); // The first object should be replaced by the second one
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(&Rectangle::new(60.0, 60.0, 10.0, 10.0), &mut collisions);
    assert_eq!(collisions, vec![0]);
}
