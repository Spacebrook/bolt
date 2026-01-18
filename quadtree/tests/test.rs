use common::shapes::{Circle, Rectangle, ShapeEnum};
use quadtree::collision_detection::shape_shape;
use quadtree::quadtree::{Config, EntityTypeUpdate, QuadTree, RelocationRequest};

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::HashSet;

fn assert_collisions_with_expected<F>(
    label: &str,
    tree: &mut QuadTree,
    query: &ShapeEnum,
    hits: &mut Vec<u32>,
    expected: &mut HashSet<u32>,
    build_expected: F,
) where
    F: FnOnce(&ShapeEnum, &mut HashSet<u32>),
{
    hits.clear();
    tree.collisions(query.clone(), hits).unwrap();
    let hit_set: HashSet<u32> = hits.iter().copied().collect();
    assert_eq!(
        hit_set.len(),
        hits.len(),
        "{} returned duplicate ids",
        label
    );
    expected.clear();
    build_expected(query, expected);
    assert_eq!(hit_set, *expected, "{}", label);
}

fn assert_tree_contents(
    label: &str,
    tree: &mut QuadTree,
    query: &ShapeEnum,
    expected: &HashSet<u32>,
) {
    let mut hits = Vec::new();
    tree.collisions(query.clone(), &mut hits).unwrap();
    let hit_set: HashSet<u32> = hits.iter().copied().collect();
    assert_eq!(
        hit_set.len(),
        hits.len(),
        "{} returned duplicate ids",
        label
    );
    assert_eq!(hit_set, *expected, "{}", label);
}

#[test]
fn test_single_collision() {
    let mut qt = QuadTree::new(Rectangle::new(50.0, 50.0, 100.0, 100.0)).unwrap();
    qt.insert(
        0,
        ShapeEnum::Rectangle(Rectangle::new(50.0, 40.0, 100.0, 50.0)),
        None,
    ).unwrap();
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(
        ShapeEnum::Rectangle(Rectangle::new(10.0, 10.0, 20.0, 20.0)),
        &mut collisions,
    ).unwrap();
    assert_eq!(collisions, vec![0]);
}

#[test]
fn test_full_tree() {
    let mut qt = QuadTree::new(Rectangle::new(500.0, 500.0, 1000.0, 1000.0)).unwrap();
    qt.insert(
        0,
        ShapeEnum::Rectangle(Rectangle::new(525.0, 525.0, 50.0, 50.0)),
        None,
    ).unwrap();
    qt.insert(1, ShapeEnum::Circle(Circle::new(500.0, 500.0, 25.0)), None).unwrap();

    let mut rng = StdRng::seed_from_u64(0);
    for i in 2..5 {
        let width = rng.gen_range(0.0..100.0);
        let height = rng.gen_range(0.0..100.0);
        let x = rng.gen_range(0.0..(900.0 - width / 2.0)) + width / 2.0;
        let y = rng.gen_range(0.0..(900.0 - height / 2.0)) + height / 2.0;
        qt.insert(
            i,
            ShapeEnum::Rectangle(Rectangle::new(x, y, width, height)),
            None,
        ).unwrap();
    }

    for i in 5..8 {
        let radius = rng.gen_range(0.0..50.0);
        let x = rng.gen_range(0.0..(950.0 - radius)) + radius;
        let y = rng.gen_range(0.0..(950.0 - radius)) + radius;
        qt.insert(i, ShapeEnum::Circle(Circle::new(x, y, radius)), None).unwrap();
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
        ShapeEnum::Rectangle(Rectangle::new(500.5, 500.5, 1.0, 1.0)),
        &mut collisions,
    ).unwrap();
    assert!(collisions.contains(&0));
    assert!(collisions.contains(&1));

    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(
        ShapeEnum::Circle(Circle::new(500.0, 500.0, 1.0)),
        &mut collisions,
    ).unwrap();
    assert!(collisions.contains(&0));
    assert!(collisions.contains(&1));
}

#[test]
fn delete_after_relocate_before_update_clears_nodes() {
    let bounds = Rectangle::new(0.0, 0.0, 100.0, 100.0);
    let config = Config {
        pool_size: 64,
        node_capacity: 1,
        max_depth: 4,
        min_size: 1.0,
        looseness: 1.0,
        large_entity_threshold_factor: 0.0,
        profile_summary: false,
        profile_detail: false,
        profile_limit: 5,
    };
    let mut qt = QuadTree::new_with_config(bounds, config).unwrap();

    qt.insert(1, ShapeEnum::Circle(Circle::new(-30.0, -30.0, 2.0)), None).unwrap();
    qt.insert(2, ShapeEnum::Circle(Circle::new(-30.0, 30.0, 2.0)), None).unwrap();
    qt.insert(3, ShapeEnum::Circle(Circle::new(30.0, -30.0, 2.0)), None).unwrap();
    qt.insert(4, ShapeEnum::Circle(Circle::new(30.0, 30.0, 2.0)), None).unwrap();
    qt.insert(10, ShapeEnum::Circle(Circle::new(-35.0, 0.0, 2.0)), None).unwrap();
    qt.update();

    qt.relocate(10, ShapeEnum::Circle(Circle::new(35.0, 0.0, 2.0)), EntityTypeUpdate::Preserve).unwrap();
    qt.delete(10);

    let mut hits = Vec::new();
    qt.collisions(
        ShapeEnum::Rectangle(Rectangle::new(0.0, 0.0, 200.0, 200.0)),
        &mut hits,
    ).unwrap();
    assert!(
        !hits.contains(&10),
        "deleted entity should not appear in world query"
    );

    hits.clear();
    qt.collisions(ShapeEnum::Circle(Circle::new(-35.0, 0.0, 3.0)), &mut hits).unwrap();
    assert!(
        !hits.contains(&10),
        "deleted entity should not appear at prior location"
    );
}

#[test]
fn test_huge_bounds() {
    let mut qt = QuadTree::new(Rectangle::new(0.0, 0.0, 2000000.0, 2000000.0)).unwrap();
    qt.insert(
        0,
        ShapeEnum::Rectangle(Rectangle::new(16030.0, -325.0, 60.0, 60.0)),
        None,
    ).unwrap();
    qt.insert(
        1,
        ShapeEnum::Rectangle(Rectangle::new(16010.0, -320.0, 60.0, 60.0)),
        None,
    ).unwrap();
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(
        ShapeEnum::Rectangle(Rectangle::new(16010.0, -320.0, 60.0, 60.0)),
        &mut collisions,
    ).unwrap();
    let collision_set: HashSet<_> = collisions.into_iter().collect();
    assert_eq!(collision_set.len(), 2);
    assert!(collision_set.contains(&0));
    assert!(collision_set.contains(&1));
}

#[test]
fn test_no_collision() {
    // Test case where there are no collisions
    let mut qt = QuadTree::new(Rectangle::new(0.0, 0.0, 100.0, 100.0)).unwrap();
    qt.insert(
        0,
        ShapeEnum::Rectangle(Rectangle::new(10.0, 10.0, 20.0, 20.0)),
        None,
    ).unwrap();
    qt.insert(
        1,
        ShapeEnum::Rectangle(Rectangle::new(50.0, 50.0, 20.0, 20.0)),
        None,
    ).unwrap();
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(
        ShapeEnum::Rectangle(Rectangle::new(80.0, 80.0, 10.0, 10.0)),
        &mut collisions,
    ).unwrap();
    assert!(collisions.is_empty());
}

#[test]
fn test_edge_touching_exclusive_collisions() {
    let mut qt = QuadTree::new(Rectangle::new(0.0, 0.0, 100.0, 100.0)).unwrap();
    qt.insert(
        1,
        ShapeEnum::Rectangle(Rectangle::new(0.0, 0.0, 10.0, 10.0)),
        None,
    ).unwrap();
    qt.insert(2, ShapeEnum::Circle(Circle::new(20.0, 0.0, 5.0)), None).unwrap();
    qt.insert(3, ShapeEnum::Circle(Circle::new(45.0, 0.0, 5.0)), None).unwrap();

    let mut hits = Vec::new();
    qt.collisions(
        ShapeEnum::Rectangle(Rectangle::new(10.0, 0.0, 10.0, 10.0)),
        &mut hits,
    ).unwrap();
    assert!(
        !hits.contains(&1),
        "rectangle edge touch should not collide"
    );

    hits.clear();
    qt.collisions(ShapeEnum::Circle(Circle::new(30.0, 0.0, 5.0)), &mut hits).unwrap();
    assert!(!hits.contains(&2), "circle edge touch should not collide");

    hits.clear();
    qt.collisions(
        ShapeEnum::Rectangle(Rectangle::new(30.0, 0.0, 20.0, 10.0)),
        &mut hits,
    ).unwrap();
    assert!(
        !hits.contains(&3),
        "circle-rectangle edge touch should not collide"
    );
}

#[test]
fn test_multiple_collisions() {
    // Test case where a query shape collides with multiple objects in the quadtree
    let mut qt = QuadTree::new(Rectangle::new(0.0, 0.0, 100.0, 100.0)).unwrap();
    qt.insert(
        0,
        ShapeEnum::Rectangle(Rectangle::new(10.0, 10.0, 20.0, 20.0)),
        None,
    ).unwrap();
    qt.insert(
        1,
        ShapeEnum::Rectangle(Rectangle::new(20.0, 20.0, 30.0, 30.0)),
        None,
    ).unwrap();
    qt.insert(
        2,
        ShapeEnum::Rectangle(Rectangle::new(15.0, 15.0, 15.0, 15.0)),
        None,
    ).unwrap();
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(
        ShapeEnum::Rectangle(Rectangle::new(15.0, 15.0, 20.0, 20.0)),
        &mut collisions,
    ).unwrap();
    let collision_set: HashSet<_> = collisions.into_iter().collect();
    assert_eq!(collision_set.len(), 3);
    assert!(collision_set.contains(&0));
    assert!(collision_set.contains(&1));
    assert!(collision_set.contains(&2));
}

#[test]
fn test_object_relocation() {
    // Test case where an object is relocated within the quadtree
    let mut qt = QuadTree::new(Rectangle::new(0.0, 0.0, 100.0, 100.0)).unwrap();
    qt.insert(
        0,
        ShapeEnum::Rectangle(Rectangle::new(10.0, 10.0, 10.0, 10.0)),
        None,
    ).unwrap();
    qt.relocate(
        0,
        ShapeEnum::Rectangle(Rectangle::new(60.0, 60.0, 10.0, 10.0)),
        EntityTypeUpdate::Preserve,
    ).unwrap();
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(
        ShapeEnum::Rectangle(Rectangle::new(60.0, 60.0, 10.0, 10.0)),
        &mut collisions,
    ).unwrap();
    assert_eq!(collisions, vec![0]);
}

#[test]
fn test_object_relocation_initial() {
    // Test case where an object is relocated without ever being inserted
    let mut qt = QuadTree::new(Rectangle::new(0.0, 0.0, 100.0, 100.0)).unwrap();
    qt.relocate(
        0,
        ShapeEnum::Rectangle(Rectangle::new(10.0, 10.0, 10.0, 10.0)),
        EntityTypeUpdate::Preserve,
    ).unwrap();
    qt.relocate(
        0,
        ShapeEnum::Rectangle(Rectangle::new(60.0, 60.0, 10.0, 10.0)),
        EntityTypeUpdate::Preserve,
    ).unwrap();
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(
        ShapeEnum::Rectangle(Rectangle::new(60.0, 60.0, 10.0, 10.0)),
        &mut collisions,
    ).unwrap();
    assert_eq!(collisions, vec![0]);
}

#[test]
fn test_object_relocation_multiple_times() {
    use rand::Rng;

    // Test case where 1,000 objects are created and each relocated 10 times at random locations
    let mut qt = QuadTree::new(Rectangle::new(0.0, 0.0, 100.0, 100.0)).unwrap();
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
        qt.insert(i as u32, ShapeEnum::Rectangle(rect), None).unwrap();
    }

    // Relocate each object 10 times
    for i in 0..num_objects {
        for _ in 0..relocation_count {
            let rect = random_rectangle(&mut rng);
            qt.relocate(i as u32, ShapeEnum::Rectangle(rect), EntityTypeUpdate::Preserve).unwrap();
        }
    }
}

#[test]
fn test_object_deletion() {
    // Test case where an object is deleted from the quadtree
    let mut qt = QuadTree::new(Rectangle::new(0.0, 0.0, 100.0, 100.0)).unwrap();
    qt.insert(
        0,
        ShapeEnum::Rectangle(Rectangle::new(10.0, 10.0, 10.0, 10.0)),
        None,
    ).unwrap();
    qt.insert(
        1,
        ShapeEnum::Rectangle(Rectangle::new(50.0, 50.0, 10.0, 10.0)),
        None,
    ).unwrap();
    qt.delete(0);
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(
        ShapeEnum::Rectangle(Rectangle::new(10.0, 10.0, 10.0, 10.0)),
        &mut collisions,
    ).unwrap();
    assert!(!collisions.contains(&0));
}

#[test]
fn test_object_out_of_bounds() {
    // Test case where an object is outside the bounds of the quadtree
    let mut qt = QuadTree::new(Rectangle::new(0.0, 0.0, 100.0, 100.0)).unwrap();
    qt.insert(
        0,
        ShapeEnum::Rectangle(Rectangle::new(150.0, 150.0, 10.0, 10.0)),
        None,
    ).unwrap();
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(
        ShapeEnum::Rectangle(Rectangle::new(150.0, 150.0, 10.0, 10.0)),
        &mut collisions,
    ).unwrap();
    assert_eq!(collisions, vec![0]);
}

#[test]
fn test_empty_quad_tree() {
    // Test case where the quadtree is empty
    let mut qt = QuadTree::new(Rectangle::new(0.0, 0.0, 100.0, 100.0)).unwrap();
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(
        ShapeEnum::Rectangle(Rectangle::new(10.0, 10.0, 10.0, 10.0)),
        &mut collisions,
    ).unwrap();
    assert!(collisions.is_empty());
}

#[test]
fn test_query_with_large_shape() {
    // Test case where a query shape is large and intersects multiple nodes
    let mut qt = QuadTree::new(Rectangle::new(50.0, 50.0, 100.0, 100.0)).unwrap();
    qt.insert(
        0,
        ShapeEnum::Rectangle(Rectangle::new(15.0, 15.0, 10.0, 10.0)),
        None,
    ).unwrap();
    qt.insert(
        1,
        ShapeEnum::Rectangle(Rectangle::new(55.0, 55.0, 10.0, 10.0)),
        None,
    ).unwrap();
    qt.insert(
        2,
        ShapeEnum::Rectangle(Rectangle::new(75.0, 75.0, 10.0, 10.0)),
        None,
    ).unwrap();
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(
        ShapeEnum::Rectangle(Rectangle::new(50.0, 50.0, 100.0, 100.0)),
        &mut collisions,
    ).unwrap();
    let collision_set: HashSet<_> = collisions.into_iter().collect();
    assert_eq!(collision_set.len(), 3);
    assert!(collision_set.contains(&0));
    assert!(collision_set.contains(&1));
    assert!(collision_set.contains(&2));
}

#[test]
fn test_boundary_collision() {
    // Test case where a query shape is positioned on the boundary of another shape
    let mut qt = QuadTree::new(Rectangle::new(0.0, 0.0, 100.0, 100.0)).unwrap();
    qt.insert(
        0,
        ShapeEnum::Rectangle(Rectangle::new(10.0, 10.0, 20.0, 20.0)),
        None,
    ).unwrap();
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(
        ShapeEnum::Rectangle(Rectangle::new(30.0, 10.0, 10.0, 10.0)),
        &mut collisions,
    ).unwrap();
    assert!(collisions.is_empty());
}

#[test]
fn test_shape_spanning_multiple_quadrants() {
    // Test case where a shape spans multiple quadrants
    let mut qt = QuadTree::new(Rectangle::new(0.0, 0.0, 100.0, 100.0)).unwrap();
    qt.insert(
        0,
        ShapeEnum::Rectangle(Rectangle::new(45.0, 45.0, 10.0, 10.0)),
        None,
    ).unwrap();
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(
        ShapeEnum::Rectangle(Rectangle::new(48.0, 48.0, 2.0, 2.0)),
        &mut collisions,
    ).unwrap();
    assert_eq!(collisions, vec![0]);
}

#[test]
fn test_object_insertion_with_same_key() {
    // Test case where an object is inserted with the same key as an existing object
    let mut qt = QuadTree::new(Rectangle::new(0.0, 0.0, 100.0, 100.0)).unwrap();
    qt.insert(
        0,
        ShapeEnum::Rectangle(Rectangle::new(10.0, 10.0, 10.0, 10.0)),
        None,
    ).unwrap();
    qt.insert(
        0,
        ShapeEnum::Rectangle(Rectangle::new(60.0, 60.0, 10.0, 10.0)),
        None,
    ).unwrap(); // Same key as the first object
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(
        ShapeEnum::Rectangle(Rectangle::new(10.0, 10.0, 10.0, 10.0)),
        &mut collisions,
    ).unwrap();
    assert!(!collisions.contains(&0)); // The first object should be replaced by the second one
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(
        ShapeEnum::Rectangle(Rectangle::new(60.0, 60.0, 10.0, 10.0)),
        &mut collisions,
    ).unwrap();
    assert_eq!(collisions, vec![0]);
}

#[test]
fn test_relocation_outside_quadtree_bounds() {
    // Create a QuadTree with a bounding box of (0.0, 0.0, 100.0, 100.0)
    let mut qt = QuadTree::new(Rectangle::new(0.0, 0.0, 100.0, 100.0)).unwrap();

    // Insert an object with ID 0 and a bounding box (10.0, 10.0, 10.0, 10.0)
    qt.insert(
        0,
        ShapeEnum::Rectangle(Rectangle::new(10.0, 10.0, 10.0, 10.0)),
        None,
    ).unwrap();
    // Attempt to relocate the object to a position outside the bounds of the quadtree
    qt.relocate(
        0,
        ShapeEnum::Rectangle(Rectangle::new(200.0, 200.0, 10.0, 10.0)),
        EntityTypeUpdate::Preserve,
    ).unwrap();

    // Verify that the object is still in the quadtree
    let mut collisions: Vec<u32> = Vec::new();
    qt.collisions(
        ShapeEnum::Rectangle(Rectangle::new(200.0, 200.0, 10.0, 10.0)),
        &mut collisions,
    ).unwrap();
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
        min_size: 1.0,
        looseness: 1.0,
        large_entity_threshold_factor: 0.0,
        profile_summary: false,
        profile_detail: false,
        profile_limit: 5,
    };

    // Create a QuadTree with the custom config
    let mut qt = QuadTree::new_with_config(bounding_box, config).unwrap();

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
        None,
    ).unwrap();
    qt.insert(
        2,
        ShapeEnum::Rectangle(Rectangle {
            x: 40.0,
            y: 10.0,
            width: 10.0,
            height: 10.0,
        }),
        None,
    ).unwrap();
    qt.insert(
        3,
        ShapeEnum::Rectangle(Rectangle {
            x: 10.0,
            y: 40.0,
            width: 10.0,
            height: 10.0,
        }),
        None,
    ).unwrap();
    qt.insert(
        4,
        ShapeEnum::Rectangle(Rectangle {
            x: 40.0,
            y: 40.0,
            width: 10.0,
            height: 10.0,
        }),
        None,
    ).unwrap();

    // The next insertion will trigger subdivision of the root node
    qt.insert(
        5,
        ShapeEnum::Rectangle(Rectangle {
            x: 30.0,
            y: 30.0,
            width: 40.0,
            height: 40.0,
        }),
        None,
    ).unwrap();

    // Insert more items into the QuadTree to trigger the second subdivision
    qt.insert(
        6,
        ShapeEnum::Rectangle(Rectangle {
            x: 10.0,
            y: 10.0,
            width: 10.0,
            height: 10.0,
        }),
        None,
    ).unwrap();
    qt.insert(
        7,
        ShapeEnum::Rectangle(Rectangle {
            x: 40.0,
            y: 10.0,
            width: 10.0,
            height: 10.0,
        }),
        None,
    ).unwrap();
    qt.insert(
        8,
        ShapeEnum::Rectangle(Rectangle {
            x: 10.0,
            y: 40.0,
            width: 10.0,
            height: 10.0,
        }),
        None,
    ).unwrap();
    qt.insert(
        9,
        ShapeEnum::Rectangle(Rectangle {
            x: 40.0,
            y: 40.0,
            width: 10.0,
            height: 10.0,
        }),
        None,
    ).unwrap();

    // Without the fix, the next insertion would recursively trigger subdivision and overwrite child nodes
    qt.insert(
        10,
        ShapeEnum::Rectangle(Rectangle {
            x: 30.0,
            y: 30.0,
            width: 40.0,
            height: 40.0,
        }),
        None,
    ).unwrap();

    // Check that all items were successfully redistributed and the QuadTree is in a consistent state
    let mut all_shapes = Vec::new();
    qt.all_shapes(&mut all_shapes);
    assert_eq!(all_shapes.len(), 10);

    let mut all_bounding_boxes = Vec::new();
    qt.all_node_bounding_boxes(&mut all_bounding_boxes);
    assert!(all_bounding_boxes.len() > 1);
}

#[test]
fn stress_multi_tree_collision_queries() {
    const GROUP_A_COUNT: usize = 200;
    const GROUP_B_COUNT: usize = 400;
    const GROUP_C_COUNT: usize = 300;
    const TICKS: usize = 80;
    const ARENA_W: f32 = 1200.0;
    const ARENA_H: f32 = 1200.0;

    let bounds = Rectangle {
        x: 0.0,
        y: 0.0,
        width: ARENA_W,
        height: ARENA_H,
    };
    let min_x = -ARENA_W * 0.5;
    let max_x = ARENA_W * 0.5;
    let min_y = -ARENA_H * 0.5;
    let max_y = ARENA_H * 0.5;

    let config = Config {
        pool_size: 5000,
        node_capacity: 4,
        max_depth: 6,
        min_size: 1.0,
        looseness: 1.0,
        large_entity_threshold_factor: 0.0,
        profile_summary: false,
        profile_detail: false,
        profile_limit: 5,
    };
    let mut group_a_quadtree = QuadTree::new_with_config(bounds, config.clone()).unwrap();
    let mut group_a_active_quadtree = QuadTree::new_with_config(bounds, config.clone()).unwrap();
    let mut group_a_inactive_quadtree = QuadTree::new_with_config(bounds, config.clone()).unwrap();
    let mut group_b_quadtree = QuadTree::new_with_config(bounds, config.clone()).unwrap();
    let mut group_c_quadtree = QuadTree::new_with_config(bounds, config).unwrap();

    let mut seed: u64 = 0x1234_5678_9abc_def0;
    let mut next_f32 = || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let bits = (seed >> 32) as u32;
        (bits as f32) / (u32::MAX as f32)
    };

    let mut group_a = Vec::with_capacity(GROUP_A_COUNT);
    let mut group_a_active = vec![false; GROUP_A_COUNT];
    let mut group_a_inactive = vec![false; GROUP_A_COUNT];
    for id in 0..GROUP_A_COUNT {
        let radius = 8.0 + next_f32() * 16.0;
        let x = min_x + radius + next_f32() * (ARENA_W - radius * 2.0);
        let y = min_y + radius + next_f32() * (ARENA_H - radius * 2.0);
        let vx = (next_f32() * 2.0 - 1.0) * 90.0;
        let vy = (next_f32() * 2.0 - 1.0) * 90.0;
        group_a.push((id as u32, x, y, vx, vy, radius));
        group_a_quadtree.insert_circle_raw(id as u32, x, y, radius, None).unwrap();
        group_a_active[id] = next_f32() > 0.4;
        group_a_inactive[id] = next_f32() > 0.9;
    }

    let mut group_b = Vec::with_capacity(GROUP_B_COUNT);
    for i in 0..GROUP_B_COUNT {
        let id = (GROUP_A_COUNT + i) as u32;
        let is_rect = next_f32() < 0.2;
        let w = 10.0 + next_f32() * 50.0;
        let h = 10.0 + next_f32() * 50.0;
        let radius = 8.0 + next_f32() * 40.0;
        let bound_w = if is_rect { w } else { radius * 1.2 };
        let bound_h = if is_rect { h } else { radius * 1.2 };
        let x = min_x + bound_w + next_f32() * (ARENA_W - bound_w * 2.0);
        let y = min_y + bound_h + next_f32() * (ARENA_H - bound_h * 2.0);
        let vx = (next_f32() * 2.0 - 1.0) * 120.0;
        let vy = (next_f32() * 2.0 - 1.0) * 120.0;
        group_b.push((id, x, y, vx, vy, radius, w, h, is_rect));
    }

    let mut group_c = Vec::with_capacity(GROUP_C_COUNT);
    for i in 0..GROUP_C_COUNT {
        let id = (GROUP_A_COUNT + GROUP_B_COUNT + i) as u32;
        let radius = 3.0 + next_f32() * 6.0;
        let x = min_x + radius + next_f32() * (ARENA_W - radius * 2.0);
        let y = min_y + radius + next_f32() * (ARENA_H - radius * 2.0);
        let vx = (next_f32() * 2.0 - 1.0) * 70.0;
        let vy = (next_f32() * 2.0 - 1.0) * 70.0;
        group_c.push((id, x, y, vx, vy, radius));
    }

    let ticks = std::env::var("BOLT_QT_STRESS_TICKS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(TICKS);
    let log_progress = std::env::var("BOLT_QT_STRESS_LOG").ok().as_deref() == Some("1");
    for tick in 0..ticks {
        if log_progress {
            eprintln!("stress tick {} start", tick);
        }
        if log_progress && tick % 5 == 0 {
            eprintln!("stress tick {} checkpoint", tick);
        }
        if log_progress {
            eprintln!("tick {} move group A", tick);
        }
        for (i, (_, x, y, vx, vy, radius)) in group_a.iter_mut().enumerate() {
            *x += *vx;
            *y += *vy;
            if *x - *radius < min_x {
                *x = min_x + *radius;
                *vx = -*vx;
            } else if *x + *radius > max_x {
                *x = max_x - *radius;
                *vx = -*vx;
            }
            if *y - *radius < min_y {
                *y = min_y + *radius;
                *vy = -*vy;
            } else if *y + *radius > max_y {
                *y = max_y - *radius;
                *vy = -*vy;
            }
            if next_f32() > 0.985 {
                group_a_active[i] = !group_a_active[i];
            }
            if next_f32() > 0.995 {
                group_a_inactive[i] = !group_a_inactive[i];
            }
        }

        if log_progress {
            eprintln!("tick {} move group B", tick);
        }
        for (_id, x, y, vx, vy, radius, w, h, is_rect) in group_b.iter_mut() {
            *x += *vx;
            *y += *vy;
            let bound_w = if *is_rect { *w } else { *radius * 1.2 };
            let bound_h = if *is_rect { *h } else { *radius * 1.2 };
            if *x - bound_w < min_x {
                *x = min_x + bound_w;
                *vx = -*vx;
            } else if *x + bound_w > max_x {
                *x = max_x - bound_w;
                *vx = -*vx;
            }
            if *y - bound_h < min_y {
                *y = min_y + bound_h;
                *vy = -*vy;
            } else if *y + bound_h > max_y {
                *y = max_y - bound_h;
                *vy = -*vy;
            }
        }

        if log_progress {
            eprintln!("tick {} move group C", tick);
        }
        for (_, x, y, vx, vy, radius) in group_c.iter_mut() {
            *x += *vx;
            *y += *vy;
            if *x - *radius < min_x {
                *x = min_x + *radius;
                *vx = -*vx;
            } else if *x + *radius > max_x {
                *x = max_x - *radius;
                *vx = -*vx;
            }
            if *y - *radius < min_y {
                *y = min_y + *radius;
                *vy = -*vy;
            } else if *y + *radius > max_y {
                *y = max_y - *radius;
                *vy = -*vy;
            }
        }

        if log_progress {
            eprintln!("tick {} relocate group A", tick);
        }
        let mut group_a_requests = Vec::with_capacity(group_a.len());
        for (id, x, y, _, _, radius) in group_a.iter() {
            group_a_requests.push(RelocationRequest {
                value: *id,
                shape: ShapeEnum::Circle(Circle::new(*x, *y, *radius)),
                entity_type: EntityTypeUpdate::Preserve,
            });
        }
        group_a_quadtree.relocate_batch(group_a_requests).unwrap();

        if log_progress {
            eprintln!("tick {} relocate active subset", tick);
        }
        for (idx, (id, ..)) in group_a.iter().enumerate() {
            if !group_a_active[idx] {
                group_a_active_quadtree.delete(*id);
            }
        }

        let mut active_requests = Vec::new();
        for (idx, (id, x, y, _, _, radius)) in group_a.iter().enumerate() {
            if group_a_active[idx] {
                active_requests.push(RelocationRequest {
                    value: *id,
                    shape: ShapeEnum::Circle(Circle::new(*x, *y, *radius)),
                    entity_type: EntityTypeUpdate::Preserve,
                });
            }
        }
        group_a_active_quadtree.relocate_batch(active_requests).unwrap();

        if log_progress {
            eprintln!("tick {} relocate inactive subset", tick);
        }
        for (idx, (id, ..)) in group_a.iter().enumerate() {
            if !group_a_inactive[idx] {
                group_a_inactive_quadtree.delete(*id);
            }
        }

        let mut dead_requests = Vec::new();
        for (idx, (id, x, y, _, _, radius)) in group_a.iter().enumerate() {
            if group_a_inactive[idx] {
                dead_requests.push(RelocationRequest {
                    value: *id,
                    shape: ShapeEnum::Circle(Circle::new(*x, *y, *radius)),
                    entity_type: EntityTypeUpdate::Preserve,
                });
            }
        }
        group_a_inactive_quadtree.relocate_batch(dead_requests).unwrap();

        if log_progress {
            eprintln!("tick {} relocate group B", tick);
        }
        let mut group_b_requests = Vec::with_capacity(group_b.len());
        for (id, x, y, _, _, radius, w, h, is_rect) in group_b.iter() {
            let shape = if *is_rect {
                ShapeEnum::Rectangle(Rectangle {
                    x: *x,
                    y: *y,
                    width: *w * 2.0,
                    height: *h * 2.0,
                })
            } else {
                ShapeEnum::Circle(Circle::new(*x, *y, *radius * 1.2))
            };
            group_b_requests.push(RelocationRequest {
                value: *id,
                shape,
                entity_type: EntityTypeUpdate::Preserve,
            });
        }
        group_b_quadtree.relocate_batch(group_b_requests).unwrap();

        if log_progress {
            eprintln!("tick {} relocate group C", tick);
        }
        let mut group_c_requests = Vec::with_capacity(group_c.len());
        for (id, x, y, _, _, radius) in group_c.iter() {
            group_c_requests.push(RelocationRequest {
                value: *id,
                shape: ShapeEnum::Circle(Circle::new(*x, *y, *radius)),
                entity_type: EntityTypeUpdate::Preserve,
            });
        }
        group_c_quadtree.relocate_batch(group_c_requests).unwrap();

        let world_query = ShapeEnum::Rectangle(Rectangle {
            x: 0.0,
            y: 0.0,
            width: ARENA_W * 2.0,
            height: ARENA_H * 2.0,
        });
        let mut expected_all = HashSet::new();
        let mut expected_active = HashSet::new();
        let mut expected_inactive = HashSet::new();
        let mut expected_b = HashSet::new();
        let mut expected_c = HashSet::new();
        for (idx, (id, ..)) in group_a.iter().enumerate() {
            expected_all.insert(*id);
            if group_a_active[idx] {
                expected_active.insert(*id);
            }
            if group_a_inactive[idx] {
                expected_inactive.insert(*id);
            }
        }
        for (id, ..) in group_b.iter() {
            expected_b.insert(*id);
        }
        for (id, ..) in group_c.iter() {
            expected_c.insert(*id);
        }
        assert_tree_contents(
            &format!("tick {} group A contents", tick),
            &mut group_a_quadtree,
            &world_query,
            &expected_all,
        );
        assert_tree_contents(
            &format!("tick {} active subset contents", tick),
            &mut group_a_active_quadtree,
            &world_query,
            &expected_active,
        );
        assert_tree_contents(
            &format!("tick {} inactive subset contents", tick),
            &mut group_a_inactive_quadtree,
            &world_query,
            &expected_inactive,
        );
        assert_tree_contents(
            &format!("tick {} group B contents", tick),
            &mut group_b_quadtree,
            &world_query,
            &expected_b,
        );
        assert_tree_contents(
            &format!("tick {} group C contents", tick),
            &mut group_c_quadtree,
            &world_query,
            &expected_c,
        );

        if log_progress {
            eprintln!("tick {} query group A", tick);
        }
        if log_progress && tick == 17 {
            let (nodes, node_entities, entities) = group_a_active_quadtree.storage_counts();
            eprintln!(
                "tick {} active subset counts: nodes={}, node_entities={}, entities={}",
                tick, nodes, node_entities, entities
            );
        }
        let mut pair_count = 0usize;
        let mut hits = Vec::new();
        let mut expected = HashSet::new();

        if log_progress {
            eprintln!("tick {} query group A", tick);
        }
        for (idx, (_id, x, y, _, _, radius)) in group_a.iter().enumerate() {
            let query = ShapeEnum::Circle(Circle::new(*x, *y, *radius));
            assert_collisions_with_expected(
                &format!("tick {} group A {} vs group A", tick, idx),
                &mut group_a_quadtree,
                &query,
                &mut hits,
                &mut expected,
                |query, expected| {
                    for (a_id, a_x, a_y, _, _, a_radius) in group_a.iter() {
                        let candidate = ShapeEnum::Circle(Circle::new(*a_x, *a_y, *a_radius));
                        if shape_shape(query, &candidate).unwrap() {
                            expected.insert(*a_id);
                        }
                    }
                },
            );
            pair_count = pair_count.wrapping_add(hits.len());

            assert_collisions_with_expected(
                &format!("tick {} group A {} vs group B", tick, idx),
                &mut group_b_quadtree,
                &query,
                &mut hits,
                &mut expected,
                |query, expected| {
                    for (b_id, b_x, b_y, _, _, b_radius, b_w, b_h, b_is_rect) in group_b.iter() {
                        let candidate = if *b_is_rect {
                            ShapeEnum::Rectangle(Rectangle {
                                x: *b_x,
                                y: *b_y,
                                width: *b_w * 2.0,
                                height: *b_h * 2.0,
                            })
                        } else {
                            ShapeEnum::Circle(Circle::new(*b_x, *b_y, *b_radius * 1.2))
                        };
                        if shape_shape(query, &candidate).unwrap() {
                            expected.insert(*b_id);
                        }
                    }
                },
            );
            pair_count = pair_count.wrapping_add(hits.len());

            let query_c = ShapeEnum::Circle(Circle::new(*x, *y, *radius * 1.5));
            assert_collisions_with_expected(
                &format!("tick {} group A {} vs group C", tick, idx),
                &mut group_c_quadtree,
                &query_c,
                &mut hits,
                &mut expected,
                |query, expected| {
                    for (c_id, c_x, c_y, _, _, c_radius) in group_c.iter() {
                        let candidate = ShapeEnum::Circle(Circle::new(*c_x, *c_y, *c_radius));
                        if shape_shape(query, &candidate).unwrap() {
                            expected.insert(*c_id);
                        }
                    }
                },
            );
            pair_count = pair_count.wrapping_add(hits.len());

            assert_collisions_with_expected(
                &format!("tick {} group A {} vs active subset", tick, idx),
                &mut group_a_active_quadtree,
                &query,
                &mut hits,
                &mut expected,
                |query, expected| {
                    for (a_idx, (a_id, a_x, a_y, _, _, a_radius)) in group_a.iter().enumerate() {
                        if !group_a_active[a_idx] {
                            continue;
                        }
                        let candidate = ShapeEnum::Circle(Circle::new(*a_x, *a_y, *a_radius));
                        if shape_shape(query, &candidate).unwrap() {
                            expected.insert(*a_id);
                        }
                    }
                },
            );
            pair_count = pair_count.wrapping_add(hits.len());

            assert_collisions_with_expected(
                &format!("tick {} group A {} vs inactive subset", tick, idx),
                &mut group_a_inactive_quadtree,
                &query,
                &mut hits,
                &mut expected,
                |query, expected| {
                    for (a_idx, (a_id, a_x, a_y, _, _, a_radius)) in group_a.iter().enumerate() {
                        if !group_a_inactive[a_idx] {
                            continue;
                        }
                        let candidate = ShapeEnum::Circle(Circle::new(*a_x, *a_y, *a_radius));
                        if shape_shape(query, &candidate).unwrap() {
                            expected.insert(*a_id);
                        }
                    }
                },
            );
            pair_count = pair_count.wrapping_add(hits.len());
        }

        if log_progress {
            eprintln!("tick {} query group B", tick);
        }
        for (idx, (_id, x, y, _, _, radius, w, h, is_rect)) in group_b.iter().enumerate() {
            let query = if *is_rect {
                ShapeEnum::Rectangle(Rectangle {
                    x: *x,
                    y: *y,
                    width: *w * 2.0,
                    height: *h * 2.0,
                })
            } else {
                ShapeEnum::Circle(Circle::new(*x, *y, *radius))
            };

            assert_collisions_with_expected(
                &format!("tick {} group B {} vs group A", tick, idx),
                &mut group_a_quadtree,
                &query,
                &mut hits,
                &mut expected,
                |query, expected| {
                    for (a_id, a_x, a_y, _, _, a_radius) in group_a.iter() {
                        let candidate = ShapeEnum::Circle(Circle::new(*a_x, *a_y, *a_radius));
                        if shape_shape(query, &candidate).unwrap() {
                            expected.insert(*a_id);
                        }
                    }
                },
            );
            pair_count = pair_count.wrapping_add(hits.len());

            assert_collisions_with_expected(
                &format!("tick {} group B {} vs active subset", tick, idx),
                &mut group_a_active_quadtree,
                &query,
                &mut hits,
                &mut expected,
                |query, expected| {
                    for (a_idx, (a_id, a_x, a_y, _, _, a_radius)) in group_a.iter().enumerate() {
                        if !group_a_active[a_idx] {
                            continue;
                        }
                        let candidate = ShapeEnum::Circle(Circle::new(*a_x, *a_y, *a_radius));
                        if shape_shape(query, &candidate).unwrap() {
                            expected.insert(*a_id);
                        }
                    }
                },
            );
            pair_count = pair_count.wrapping_add(hits.len());

            assert_collisions_with_expected(
                &format!("tick {} group B {} vs inactive subset", tick, idx),
                &mut group_a_inactive_quadtree,
                &query,
                &mut hits,
                &mut expected,
                |query, expected| {
                    for (a_idx, (a_id, a_x, a_y, _, _, a_radius)) in group_a.iter().enumerate() {
                        if !group_a_inactive[a_idx] {
                            continue;
                        }
                        let candidate = ShapeEnum::Circle(Circle::new(*a_x, *a_y, *a_radius));
                        if shape_shape(query, &candidate).unwrap() {
                            expected.insert(*a_id);
                        }
                    }
                },
            );
            pair_count = pair_count.wrapping_add(hits.len());

            assert_collisions_with_expected(
                &format!("tick {} group B {} vs group B", tick, idx),
                &mut group_b_quadtree,
                &query,
                &mut hits,
                &mut expected,
                |query, expected| {
                    for (b_id, b_x, b_y, _, _, b_radius, b_w, b_h, b_is_rect) in group_b.iter() {
                        let candidate = if *b_is_rect {
                            ShapeEnum::Rectangle(Rectangle {
                                x: *b_x,
                                y: *b_y,
                                width: *b_w * 2.0,
                                height: *b_h * 2.0,
                            })
                        } else {
                            ShapeEnum::Circle(Circle::new(*b_x, *b_y, *b_radius * 1.2))
                        };
                        if shape_shape(query, &candidate).unwrap() {
                            expected.insert(*b_id);
                        }
                    }
                },
            );
            pair_count = pair_count.wrapping_add(hits.len());

            assert_collisions_with_expected(
                &format!("tick {} group B {} vs group C", tick, idx),
                &mut group_c_quadtree,
                &query,
                &mut hits,
                &mut expected,
                |query, expected| {
                    for (c_id, c_x, c_y, _, _, c_radius) in group_c.iter() {
                        let candidate = ShapeEnum::Circle(Circle::new(*c_x, *c_y, *c_radius));
                        if shape_shape(query, &candidate).unwrap() {
                            expected.insert(*c_id);
                        }
                    }
                },
            );
            pair_count = pair_count.wrapping_add(hits.len());
        }

        if log_progress {
            eprintln!("tick {} query group C", tick);
        }
        for (idx, (_id, x, y, _, _, radius)) in group_c.iter().enumerate() {
            let query = ShapeEnum::Circle(Circle::new(*x, *y, *radius));

            assert_collisions_with_expected(
                &format!("tick {} group C {} vs group A", tick, idx),
                &mut group_a_quadtree,
                &query,
                &mut hits,
                &mut expected,
                |query, expected| {
                    for (a_id, a_x, a_y, _, _, a_radius) in group_a.iter() {
                        let candidate = ShapeEnum::Circle(Circle::new(*a_x, *a_y, *a_radius));
                        if shape_shape(query, &candidate).unwrap() {
                            expected.insert(*a_id);
                        }
                    }
                },
            );
            pair_count = pair_count.wrapping_add(hits.len());

            assert_collisions_with_expected(
                &format!("tick {} group C {} vs active subset", tick, idx),
                &mut group_a_active_quadtree,
                &query,
                &mut hits,
                &mut expected,
                |query, expected| {
                    for (a_idx, (a_id, a_x, a_y, _, _, a_radius)) in group_a.iter().enumerate() {
                        if !group_a_active[a_idx] {
                            continue;
                        }
                        let candidate = ShapeEnum::Circle(Circle::new(*a_x, *a_y, *a_radius));
                        if shape_shape(query, &candidate).unwrap() {
                            expected.insert(*a_id);
                        }
                    }
                },
            );
            pair_count = pair_count.wrapping_add(hits.len());

            assert_collisions_with_expected(
                &format!("tick {} group C {} vs inactive subset", tick, idx),
                &mut group_a_inactive_quadtree,
                &query,
                &mut hits,
                &mut expected,
                |query, expected| {
                    for (a_idx, (a_id, a_x, a_y, _, _, a_radius)) in group_a.iter().enumerate() {
                        if !group_a_inactive[a_idx] {
                            continue;
                        }
                        let candidate = ShapeEnum::Circle(Circle::new(*a_x, *a_y, *a_radius));
                        if shape_shape(query, &candidate).unwrap() {
                            expected.insert(*a_id);
                        }
                    }
                },
            );
            pair_count = pair_count.wrapping_add(hits.len());

            assert_collisions_with_expected(
                &format!("tick {} group C {} vs group B", tick, idx),
                &mut group_b_quadtree,
                &query,
                &mut hits,
                &mut expected,
                |query, expected| {
                    for (b_id, b_x, b_y, _, _, b_radius, b_w, b_h, b_is_rect) in group_b.iter() {
                        let candidate = if *b_is_rect {
                            ShapeEnum::Rectangle(Rectangle {
                                x: *b_x,
                                y: *b_y,
                                width: *b_w * 2.0,
                                height: *b_h * 2.0,
                            })
                        } else {
                            ShapeEnum::Circle(Circle::new(*b_x, *b_y, *b_radius * 1.2))
                        };
                        if shape_shape(query, &candidate).unwrap() {
                            expected.insert(*b_id);
                        }
                    }
                },
            );
            pair_count = pair_count.wrapping_add(hits.len());

            assert_collisions_with_expected(
                &format!("tick {} group C {} vs group C", tick, idx),
                &mut group_c_quadtree,
                &query,
                &mut hits,
                &mut expected,
                |query, expected| {
                    for (c_id, c_x, c_y, _, _, c_radius) in group_c.iter() {
                        let candidate = ShapeEnum::Circle(Circle::new(*c_x, *c_y, *c_radius));
                        if shape_shape(query, &candidate).unwrap() {
                            expected.insert(*c_id);
                        }
                    }
                },
            );
            pair_count = pair_count.wrapping_add(hits.len());
        }

        if log_progress {
            eprintln!("stress tick {} end", tick);
        }
        let _ = pair_count;
    }
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
    let mut quadtree = QuadTree::new(bounding_box).unwrap();

    // Insert shapes into the quadtree.
    let shape1 = ShapeEnum::Circle(Circle::new(2.0, 2.0, 1.0));
    let shape2 = ShapeEnum::Circle(Circle::new(4.0, 4.0, 1.0));
    let shape3 = ShapeEnum::Circle(Circle::new(6.0, 6.0, 1.0));

    quadtree.insert(1, shape1, None).unwrap();
    quadtree.insert(2, shape2, None).unwrap();
    quadtree.insert(3, shape3, None).unwrap();

    // Define batch collision queries.
    let query1 = ShapeEnum::Circle(Circle::new(2.0, 2.0, 1.5));
    let query2 = ShapeEnum::Circle(Circle::new(6.0, 6.0, 1.5));
    let query3 = ShapeEnum::Circle(Circle::new(8.0, 8.0, 1.0));
    let queries = vec![query1, query2, query3];

    // Perform batch collision queries.
    let collision_results = quadtree.collisions_batch(queries).unwrap();

    // Check results.
    assert_eq!(collision_results.len(), 3);
    for (idx, hits) in collision_results.iter().enumerate() {
        let hit_set: HashSet<u32> = hits.iter().copied().collect();
        assert_eq!(
            hit_set.len(),
            hits.len(),
            "collisions_batch query {} returned duplicate ids",
            idx
        );
    }

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

#[test]
fn test_delete_and_reuse_ids() {
    let bounding_box = Rectangle {
        x: 0.0,
        y: 0.0,
        width: 20.0,
        height: 20.0,
    };
    let mut quadtree = QuadTree::new(bounding_box).unwrap();

    quadtree.insert(1, ShapeEnum::Circle(Circle::new(-4.0, -4.0, 1.0)), None).unwrap();
    quadtree.insert(2, ShapeEnum::Circle(Circle::new(-2.0, -2.0, 1.0)), None).unwrap();
    quadtree.insert(
        3,
        ShapeEnum::Rectangle(Rectangle {
            x: 2.0,
            y: -2.0,
            width: 2.0,
            height: 2.0,
        }),
        None,
    ).unwrap();
    quadtree.insert(4, ShapeEnum::Circle(Circle::new(4.0, 4.0, 1.0)), None).unwrap();

    let mut hits = Vec::new();
    quadtree.collisions(ShapeEnum::Circle(Circle::new(-2.0, -2.0, 1.0)), &mut hits).unwrap();
    assert!(hits.contains(&2));

    quadtree.delete(2);
    quadtree.delete(4);

    hits.clear();
    quadtree.collisions(ShapeEnum::Circle(Circle::new(-2.0, -2.0, 1.0)), &mut hits).unwrap();
    assert!(!hits.contains(&2));
    hits.clear();
    quadtree.collisions(ShapeEnum::Circle(Circle::new(4.0, 4.0, 1.0)), &mut hits).unwrap();
    assert!(!hits.contains(&4));

    quadtree.insert(2, ShapeEnum::Circle(Circle::new(6.0, -6.0, 1.0)), None).unwrap();
    quadtree.insert(
        4,
        ShapeEnum::Rectangle(Rectangle {
            x: -6.0,
            y: 6.0,
            width: 2.0,
            height: 2.0,
        }),
        None,
    ).unwrap();

    hits.clear();
    quadtree.collisions(ShapeEnum::Circle(Circle::new(6.0, -6.0, 1.0)), &mut hits).unwrap();
    assert!(hits.contains(&2));
    hits.clear();
    quadtree.collisions(ShapeEnum::Circle(Circle::new(-6.0, 6.0, 1.0)), &mut hits).unwrap();
    assert!(hits.contains(&4));
}

#[test]
fn test_collisions_with_entity_type_filter() {
    // Define the bounds of the QuadTree
    let mut qt = QuadTree::new(Rectangle {
        x: 0.0,
        y: 0.0,
        width: 10.0,
        height: 10.0,
    }).unwrap();

    // Insert entities into the QuadTree
    let entity_type_1: Option<u32> = Some(1);
    let entity_type_2: Option<u32> = Some(2);

    // Entity 1
    let shape_1 = ShapeEnum::Circle(Circle::new(2.0, 2.0, 1.0));
    qt.insert(1, shape_1.clone(), entity_type_1).unwrap();

    // Entity 2
    let shape_2 = ShapeEnum::Circle(Circle::new(4.0, 2.0, 1.0));
    qt.insert(2, shape_2.clone(), entity_type_2).unwrap();

    // Entity 3
    let shape_3 = ShapeEnum::Circle(Circle::new(7.0, 5.0, 1.0));
    qt.insert(3, shape_3.clone(), entity_type_1).unwrap();

    // Query shape for collisions
    let query_shape = ShapeEnum::Circle(Circle::new(3.0, 3.0, 2.0));

    // Test collisions with normal collisions method
    let mut collisions = Vec::new();
    qt.collisions(query_shape.clone(), &mut collisions).unwrap();
    collisions.sort();
    assert_eq!(collisions, vec![1, 2]);

    // Test collisions without entity type filter
    let mut collisions = Vec::new();
    qt.collisions_filter(query_shape.clone(), None, &mut collisions).unwrap();
    collisions.sort();
    assert_eq!(collisions, vec![1, 2]);

    // Test collisions with entity type filter (only entity type 1)
    let mut collisions = Vec::new();
    qt.collisions_filter(query_shape.clone(), Some(vec![1]), &mut collisions).unwrap();
    assert_eq!(collisions, vec![1]);

    // Test collisions with entity type filter (only entity type 2)
    let mut collisions = Vec::new();
    qt.collisions_filter(query_shape.clone(), Some(vec![2]), &mut collisions).unwrap();
    assert_eq!(collisions, vec![2]);
}
