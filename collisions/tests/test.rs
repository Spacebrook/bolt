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
