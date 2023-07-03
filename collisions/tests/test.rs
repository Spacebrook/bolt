use collisions::*;
use ncollide2d::math::{Isometry, Vector};
use ncollide2d::shape::{Ball, Cuboid};

#[test]
fn test_no_colliding_polys() {
    let entity = ShapeWithPosition {
        shape: Box::new(Ball::new(1.0)),
        position: Isometry::new(Vector::new(0.0, 0.0), 0.0),
    };
    let colliding_polys: Vec<ShapeWithPosition> = Vec::new();
    assert_eq!(get_mtv(&entity, colliding_polys), None);
}

#[test]
fn test_no_collision() {
    let entity = ShapeWithPosition {
        shape: Box::new(Ball::new(1.0)),
        position: Isometry::new(Vector::new(0.0, 0.0), 0.0),
    };
    let colliding_poly = ShapeWithPosition {
        shape: Box::new(Cuboid::new(Vector::new(1.0, 1.0))),
        position: Isometry::new(Vector::new(3.0, 3.0), 0.0),
    };
    assert_eq!(get_mtv(&entity, vec![colliding_poly]), None);
}

#[test]
fn test_single_collision() {
    let entity = ShapeWithPosition {
        shape: Box::new(Ball::new(1.0)),
        position: Isometry::new(Vector::new(0.0, 0.0), 0.0),
    };
    let colliding_poly = ShapeWithPosition {
        shape: Box::new(Cuboid::new(Vector::new(1.0, 1.0))),
        position: Isometry::new(Vector::new(1.5, 0.0), 0.0),
    };
    assert_eq!(get_mtv(&entity, vec![colliding_poly]), Some((0.5, 0.0)));
}

#[test]
fn test_multiple_collisions() {
    let entity = ShapeWithPosition {
        shape: Box::new(Ball::new(1.0)),
        position: Isometry::new(Vector::new(0.0, 0.0), 0.0),
    };
    let colliding_poly1 = ShapeWithPosition {
        shape: Box::new(Cuboid::new(Vector::new(1.0, 1.0))),
        position: Isometry::new(Vector::new(1.0, 0.0), 0.0),
    };
    let colliding_poly2 = ShapeWithPosition {
        shape: Box::new(Cuboid::new(Vector::new(1.0, 1.0))),
        position: Isometry::new(Vector::new(0.0, 1.05), 0.0),
    };
    assert_eq!(
        get_mtv(&entity, vec![colliding_poly1, colliding_poly2]),
        Some((1.0, 0.95000005))
    );
}

#[test]
fn test_circle_halfway_inside_rectangle() {
    let entity = ShapeWithPosition {
        shape: Box::new(Ball::new(15.0)),
        position: Isometry::new(Vector::new(0.0, 0.0), 0.0),
    };
    let colliding_poly = ShapeWithPosition {
        shape: Box::new(Cuboid::new(Vector::new(1000.0, 1000.0))),
        position: Isometry::new(Vector::new(0.0, -1000.0), 0.0),
    };
    assert_eq!(get_mtv(&entity, vec![colliding_poly]), Some((0.0, -15.0)));
}

#[test]
fn test_circle_halfway_inside_rectangle_and_a_bit_more() {
    let entity = ShapeWithPosition {
        shape: Box::new(Ball::new(15.0)),
        position: Isometry::new(Vector::new(0.0, -1.0), 0.0),
    };
    let colliding_poly = ShapeWithPosition {
        shape: Box::new(Cuboid::new(Vector::new(1000.0, 1000.0))),
        position: Isometry::new(Vector::new(0.0, -1000.0), 0.0),
    };
    assert_eq!(get_mtv(&entity, vec![colliding_poly]), Some((0.0, -16.0)));
}

#[test]
fn test_circle_halfway_inside_rectangle_and_a_bit_less() {
    let entity = ShapeWithPosition {
        shape: Box::new(Ball::new(15.0)),
        position: Isometry::new(Vector::new(0.0, 1.0), 0.0),
    };
    let colliding_poly = ShapeWithPosition {
        shape: Box::new(Cuboid::new(Vector::new(1000.0, 1000.0))),
        position: Isometry::new(Vector::new(0.0, -1000.0), 0.0),
    };
    assert_eq!(get_mtv(&entity, vec![colliding_poly]), Some((0.0, -14.0)));
}

#[test]
fn test_diagonal_penetration() {
    let entity = ShapeWithPosition {
        shape: Box::new(Ball::new(15.0)),
        position: Isometry::new(Vector::new(500.0, 1.0), 0.0),
    };
    let colliding_poly = ShapeWithPosition {
        shape: Box::new(Cuboid::new(Vector::new(1000.0, 1000.0))),
        position: Isometry::new(Vector::new(1000.0, -1000.0), 0.0),
    };
    assert_eq!(get_mtv(&entity, vec![colliding_poly]), Some((0.0, -14.0)));
}

#[test]
fn test_overlapping_rectangles() {
    let entity = ShapeWithPosition {
        shape: Box::new(Ball::new(15.0)),
        position: Isometry::new(Vector::new(249397.66076660156, 31855.16436767578), 0.0),
    };
    let colliding_poly1 = ShapeWithPosition {
        shape: Box::new(Cuboid::new(Vector::new(96.0 / 2.0, 480.0 / 2.0))),
        position: Isometry::new(Vector::new(249356.0 + 96.0 / 2.0, 31856.0 + 480.0 / 2.0), 0.0),
    };
    let colliding_poly2 = ShapeWithPosition {
        shape: Box::new(Cuboid::new(Vector::new(384.0 / 2.0, 96.0 / 2.0))),
        position: Isometry::new(Vector::new(249388.0 + 384.0 / 2.0, 31856.0 + 96.0 / 2.0), 0.0),
    };
    assert_eq!(get_mtv(&entity, vec![colliding_poly1, colliding_poly2]), Some((0.0, 14.1640625)));
}
