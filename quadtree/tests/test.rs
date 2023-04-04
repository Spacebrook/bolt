use quadtree::quadtree::{Config, QuadTree};
use quadtree::shapes::{Circle, Rectangle, ShapeEnum};

use rand::Rng;
use std::collections::HashSet;

#[test]
fn test_single_collision() {
    let mut qt = QuadTree::new(Rectangle::new(0.0, 0.0, 100.0, 100.0));
    qt.insert(
        0,
        ShapeEnum::Rectangle(Rectangle::new(0.0, 15.0, 100.0, 50.0)),
    );
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(
        ShapeEnum::Rectangle(Rectangle::new(0.0, 0.0, 20.0, 20.0)),
        &mut collisions,
    );
    assert_eq!(collisions, vec![0]);
}

#[test]
fn test_full_tree() {
    let mut qt = QuadTree::new(Rectangle::new(0.0, 0.0, 1000.0, 1000.0));
    qt.insert(
        0,
        ShapeEnum::Rectangle(Rectangle::new(500.0, 500.0, 50.0, 50.0)),
    );
    qt.insert(1, ShapeEnum::Circle(Circle::new(500.0, 500.0, 25.0)));

    let mut rng = rand::thread_rng();
    for i in 2..5 {
        qt.insert(
            i,
            ShapeEnum::Rectangle(Rectangle::new(
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
            ShapeEnum::Circle(Circle::new(
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
    qt.collisions(
        ShapeEnum::Rectangle(Rectangle::new(500.0, 500.0, 1.0, 1.0)),
        &mut collisions,
    );
    assert!(collisions.contains(&0));
    assert!(collisions.contains(&1));

    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(
        ShapeEnum::Circle(Circle::new(500.0, 500.0, 1.0)),
        &mut collisions,
    );
    assert!(collisions.contains(&0));
    assert!(collisions.contains(&1));
}

#[test]
fn test_huge_bounds() {
    let bounds = Rectangle::new(-1000000.0, -1000000.0, 2000000.0, 2000000.0);
    let mut qt = QuadTree::new(bounds);
    qt.insert(
        0,
        ShapeEnum::Rectangle(Rectangle::new(16000.0, -355.0, 60.0, 60.0)),
    );
    qt.insert(
        1,
        ShapeEnum::Rectangle(Rectangle::new(15980.0, -350.0, 60.0, 60.0)),
    );
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(
        ShapeEnum::Rectangle(Rectangle::new(15980.0, -350.0, 60.0, 60.0)),
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
    qt.insert(
        0,
        ShapeEnum::Rectangle(Rectangle::new(10.0, 10.0, 20.0, 20.0)),
    );
    qt.insert(
        1,
        ShapeEnum::Rectangle(Rectangle::new(50.0, 50.0, 20.0, 20.0)),
    );
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(
        ShapeEnum::Rectangle(Rectangle::new(80.0, 80.0, 10.0, 10.0)),
        &mut collisions,
    );
    assert!(collisions.is_empty());
}

#[test]
fn test_multiple_collisions() {
    // Test case where a query shape collides with multiple objects in the quadtree
    let mut qt = QuadTree::new(Rectangle::new(0.0, 0.0, 100.0, 100.0));
    qt.insert(
        0,
        ShapeEnum::Rectangle(Rectangle::new(10.0, 10.0, 20.0, 20.0)),
    );
    qt.insert(
        1,
        ShapeEnum::Rectangle(Rectangle::new(20.0, 20.0, 30.0, 30.0)),
    );
    qt.insert(
        2,
        ShapeEnum::Rectangle(Rectangle::new(15.0, 15.0, 15.0, 15.0)),
    );
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(
        ShapeEnum::Rectangle(Rectangle::new(15.0, 15.0, 20.0, 20.0)),
        &mut collisions,
    );
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
    qt.insert(
        0,
        ShapeEnum::Rectangle(Rectangle::new(10.0, 10.0, 10.0, 10.0)),
    );
    qt.relocate(
        0,
        ShapeEnum::Rectangle(Rectangle::new(60.0, 60.0, 10.0, 10.0)),
    );
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(
        ShapeEnum::Rectangle(Rectangle::new(60.0, 60.0, 10.0, 10.0)),
        &mut collisions,
    );
    assert_eq!(collisions, vec![0]);
}

#[test]
fn test_object_relocation_initial() {
    // Test case where an object is relocated without ever being inserted
    let mut qt = QuadTree::new(Rectangle::new(0.0, 0.0, 100.0, 100.0));
    qt.relocate(
        0,
        ShapeEnum::Rectangle(Rectangle::new(10.0, 10.0, 10.0, 10.0)),
    );
    qt.relocate(
        0,
        ShapeEnum::Rectangle(Rectangle::new(60.0, 60.0, 10.0, 10.0)),
    );
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(
        ShapeEnum::Rectangle(Rectangle::new(60.0, 60.0, 10.0, 10.0)),
        &mut collisions,
    );
    assert_eq!(collisions, vec![0]);
}

#[test]
fn test_object_relocation_multiple_times() {
    use rand::Rng;

    // Test case where 1,000 objects are created and each relocated 10 times at random locations
    let mut qt = QuadTree::new(Rectangle::new(0.0, 0.0, 100.0, 100.0));
    let num_objects = 1_000;
    let relocation_count = 10;
    let mut rng = rand::thread_rng();

    // Helper function to generate random rectangles within quadtree bounds
    fn random_rectangle(rng: &mut rand::rngs::ThreadRng) -> Rectangle {
        let x = rng.gen_range(0.0..90.0);
        let y = rng.gen_range(0.0..90.0);
        let width = rng.gen_range(1.0..10.0);
        let height = rng.gen_range(1.0..10.0);
        Rectangle::new(x, y, width, height)
    }

    // Insert 1,000 objects at random locations
    for i in 0..num_objects {
        let rect = random_rectangle(&mut rng);
        qt.insert(i as u32, ShapeEnum::Rectangle(rect));
    }

    // Relocate each object 10 times
    for i in 0..num_objects {
        for _ in 0..relocation_count {
            let rect = random_rectangle(&mut rng);
            qt.relocate(i as u32, ShapeEnum::Rectangle(rect));
        }
    }
}

#[test]
fn test_object_deletion() {
    // Test case where an object is deleted from the quadtree
    let mut qt = QuadTree::new(Rectangle::new(0.0, 0.0, 100.0, 100.0));
    qt.insert(
        0,
        ShapeEnum::Rectangle(Rectangle::new(10.0, 10.0, 10.0, 10.0)),
    );
    qt.insert(
        1,
        ShapeEnum::Rectangle(Rectangle::new(50.0, 50.0, 10.0, 10.0)),
    );
    qt.delete(0);
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(
        ShapeEnum::Rectangle(Rectangle::new(10.0, 10.0, 10.0, 10.0)),
        &mut collisions,
    );
    assert!(!collisions.contains(&0));
}

#[test]
fn test_object_out_of_bounds() {
    // Test case where an object is outside the bounds of the quadtree
    let mut qt = QuadTree::new(Rectangle::new(0.0, 0.0, 100.0, 100.0));
    qt.insert(
        0,
        ShapeEnum::Rectangle(Rectangle::new(150.0, 150.0, 10.0, 10.0)),
    );
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(
        ShapeEnum::Rectangle(Rectangle::new(150.0, 150.0, 10.0, 10.0)),
        &mut collisions,
    );
    assert_eq!(collisions, vec![0]);
}

#[test]
fn test_empty_quad_tree() {
    // Test case where the quadtree is empty
    let qt = QuadTree::new(Rectangle::new(0.0, 0.0, 100.0, 100.0));
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(
        ShapeEnum::Rectangle(Rectangle::new(10.0, 10.0, 10.0, 10.0)),
        &mut collisions,
    );
    assert!(collisions.is_empty());
}

#[test]
fn test_query_with_large_shape() {
    // Test case where a query shape is large and intersects multiple nodes
    let mut qt = QuadTree::new(Rectangle::new(0.0, 0.0, 100.0, 100.0));
    qt.insert(
        0,
        ShapeEnum::Rectangle(Rectangle::new(10.0, 10.0, 10.0, 10.0)),
    );
    qt.insert(
        1,
        ShapeEnum::Rectangle(Rectangle::new(50.0, 50.0, 10.0, 10.0)),
    );
    qt.insert(
        2,
        ShapeEnum::Rectangle(Rectangle::new(70.0, 70.0, 10.0, 10.0)),
    );
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(
        ShapeEnum::Rectangle(Rectangle::new(0.0, 0.0, 100.0, 100.0)),
        &mut collisions,
    );
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
    qt.insert(
        0,
        ShapeEnum::Rectangle(Rectangle::new(10.0, 10.0, 20.0, 20.0)),
    );
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(
        ShapeEnum::Rectangle(Rectangle::new(30.0, 10.0, 10.0, 10.0)),
        &mut collisions,
    );
    assert!(collisions.is_empty());
}

#[test]
fn test_shape_spanning_multiple_quadrants() {
    // Test case where a shape spans multiple quadrants
    let mut qt = QuadTree::new(Rectangle::new(0.0, 0.0, 100.0, 100.0));
    qt.insert(
        0,
        ShapeEnum::Rectangle(Rectangle::new(45.0, 45.0, 10.0, 10.0)),
    );
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(
        ShapeEnum::Rectangle(Rectangle::new(48.0, 48.0, 2.0, 2.0)),
        &mut collisions,
    );
    assert_eq!(collisions, vec![0]);
}

#[test]
fn test_object_insertion_with_same_key() {
    // Test case where an object is inserted with the same key as an existing object
    let mut qt = QuadTree::new(Rectangle::new(0.0, 0.0, 100.0, 100.0));
    qt.insert(
        0,
        ShapeEnum::Rectangle(Rectangle::new(10.0, 10.0, 10.0, 10.0)),
    );
    qt.insert(
        0,
        ShapeEnum::Rectangle(Rectangle::new(60.0, 60.0, 10.0, 10.0)),
    ); // Same key as the first object
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(
        ShapeEnum::Rectangle(Rectangle::new(10.0, 10.0, 10.0, 10.0)),
        &mut collisions,
    );
    assert!(!collisions.contains(&0)); // The first object should be replaced by the second one
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(
        ShapeEnum::Rectangle(Rectangle::new(60.0, 60.0, 10.0, 10.0)),
        &mut collisions,
    );
    assert_eq!(collisions, vec![0]);
}

#[test]
fn test_relocation_outside_quadtree_bounds() {
    // Create a QuadTree with a bounding box of (0.0, 0.0, 100.0, 100.0)
    let mut qt = QuadTree::new(Rectangle::new(0.0, 0.0, 100.0, 100.0));

    // Insert an object with ID 0 and a bounding box (10.0, 10.0, 10.0, 10.0)
    qt.insert(
        0,
        ShapeEnum::Rectangle(Rectangle::new(10.0, 10.0, 10.0, 10.0)),
    );
    // Attempt to relocate the object to a position outside the bounds of the quadtree
    qt.relocate(
        0,
        ShapeEnum::Rectangle(Rectangle::new(200.0, 200.0, 10.0, 10.0)),
    );

    // Verify that the object is still in the quadtree
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(
        ShapeEnum::Rectangle(Rectangle::new(200.0, 200.0, 10.0, 10.0)),
        &mut collisions,
    );
    assert_eq!(collisions, vec![0]);
}

#[test]
fn test_no_multiple_subdivision() {
    let bounding_box = Rectangle {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 100.0,
    };

    let config = Config {
        pool_size: 4000,
        node_capacity: 4,
        max_depth: 2,
    };

    // Create a QuadTree with the custom config
    let mut qt = QuadTree::new_with_config(bounding_box, config);

    // Insert shapes into the QuadTree in such a way that they will be redistributed
    // during the subdivision process and cause multiple subdivision attempts
    qt.insert(
        1,
        ShapeEnum::Rectangle(Rectangle {
            x: 10.0,
            y: 10.0,
            width: 60.0,
            height: 60.0,
        }),
    );
    qt.insert(
        2,
        ShapeEnum::Rectangle(Rectangle {
            x: 40.0,
            y: 10.0,
            width: 10.0,
            height: 10.0,
        }),
    );
    qt.insert(
        3,
        ShapeEnum::Rectangle(Rectangle {
            x: 10.0,
            y: 40.0,
            width: 10.0,
            height: 10.0,
        }),
    );
    qt.insert(
        4,
        ShapeEnum::Rectangle(Rectangle {
            x: 40.0,
            y: 40.0,
            width: 10.0,
            height: 10.0,
        }),
    );

    // The next insertion will trigger subdivision of the root node
    qt.insert(
        5,
        ShapeEnum::Rectangle(Rectangle {
            x: 30.0,
            y: 30.0,
            width: 40.0,
            height: 40.0,
        }),
    );

    // Insert more items into the QuadTree to trigger the second subdivision
    qt.insert(
        6,
        ShapeEnum::Rectangle(Rectangle {
            x: 10.0,
            y: 10.0,
            width: 10.0,
            height: 10.0,
        }),
    );
    qt.insert(
        7,
        ShapeEnum::Rectangle(Rectangle {
            x: 40.0,
            y: 10.0,
            width: 10.0,
            height: 10.0,
        }),
    );
    qt.insert(
        8,
        ShapeEnum::Rectangle(Rectangle {
            x: 10.0,
            y: 40.0,
            width: 10.0,
            height: 10.0,
        }),
    );
    qt.insert(
        9,
        ShapeEnum::Rectangle(Rectangle {
            x: 40.0,
            y: 40.0,
            width: 10.0,
            height: 10.0,
        }),
    );

    // Without the fix, the next insertion would recursively trigger subdivision and overwrite child nodes
    qt.insert(
        10,
        ShapeEnum::Rectangle(Rectangle {
            x: 30.0,
            y: 30.0,
            width: 40.0,
            height: 40.0,
        }),
    );

    // Check that all items were successfully redistributed and the QuadTree is in a consistent state
    let mut all_shapes = Vec::new();
    qt.all_shapes(&mut all_shapes);
    assert_eq!(all_shapes.len(), 10);

    let mut all_bounding_boxes = Vec::new();
    qt.all_node_bounding_boxes(&mut all_bounding_boxes);
    assert!(all_bounding_boxes.len() > 1);
}

#[test]
fn test_collisions_batch() {
    // Define the bounding box of the quadtree.
    let bounding_box = Rectangle {
        x: 0.0,
        y: 0.0,
        width: 10.0,
        height: 10.0,
    };

    // Initialize the quadtree.
    let mut quadtree = QuadTree::new(bounding_box);

    // Insert shapes into the quadtree.
    let shape1 = ShapeEnum::Circle(Circle::new(2.0, 2.0, 1.0));
    let shape2 = ShapeEnum::Circle(Circle::new(4.0, 4.0, 1.0));
    let shape3 = ShapeEnum::Circle(Circle::new(6.0, 6.0, 1.0));

    quadtree.insert(1, shape1);
    quadtree.insert(2, shape2);
    quadtree.insert(3, shape3);

    // Define batch collision queries.
    let query1 = ShapeEnum::Circle(Circle::new(2.0, 2.0, 1.5));
    let query2 = ShapeEnum::Circle(Circle::new(6.0, 6.0, 1.5));
    let query3 = ShapeEnum::Circle(Circle::new(8.0, 8.0, 1.0));
    let queries = vec![query1, query2, query3];

    // Perform batch collision queries.
    let collision_results = quadtree.collisions_batch(queries);

    // Check results.
    assert_eq!(collision_results.len(), 3);

    // Query 1 should collide with shape 1 only.
    assert!(collision_results[0].contains(&1));
    assert!(!collision_results[0].contains(&2));
    assert!(!collision_results[0].contains(&3));

    // Query 2 should collide with shape 3.
    assert!(!collision_results[1].contains(&1));
    assert!(!collision_results[1].contains(&2));
    assert!(collision_results[1].contains(&3));

    // Query 3 should not collide with any shape.
    assert!(!collision_results[2].contains(&1));
    assert!(!collision_results[2].contains(&2));
    assert!(!collision_results[2].contains(&3));
}
