use collisions::{get_mtv, ShapeWithPosition};
use parry2d::math::{Isometry, Vector};
use parry2d::shape::{Ball, Cuboid, SharedShape};

fn assert_vec_approx_eq(actual: Option<(f32, f32)>, expected: (f32, f32)) {
    let actual = actual.expect("expected a collision result");
    assert!(
        (actual.0 - expected.0).abs() < 1e-3,
        "{actual:?} != {expected:?}"
    );
    assert!(
        (actual.1 - expected.1).abs() < 1e-3,
        "{actual:?} != {expected:?}"
    );
}

#[test]
fn test_no_colliding_polys() {
    let entity = ShapeWithPosition {
        shape: SharedShape::new(Ball::new(1.0)),
        position: Isometry::new(Vector::new(0.0, 0.0), 0.0),
    };
    let colliding_polys: Vec<ShapeWithPosition> = Vec::new();
    assert_eq!(get_mtv(&entity, &colliding_polys), None);
}

#[test]
fn test_no_collision() {
    let entity = ShapeWithPosition {
        shape: SharedShape::new(Ball::new(1.0)),
        position: Isometry::new(Vector::new(0.0, 0.0), 0.0),
    };
    let colliding_poly = ShapeWithPosition {
        shape: SharedShape::new(Cuboid::new(Vector::new(1.0, 1.0))),
        position: Isometry::new(Vector::new(3.0, 3.0), 0.0),
    };
    assert_eq!(get_mtv(&entity, &[colliding_poly]), None);
}

#[test]
fn test_single_collision() {
    let entity = ShapeWithPosition {
        shape: SharedShape::new(Ball::new(1.0)),
        position: Isometry::new(Vector::new(0.0, 0.0), 0.0),
    };
    let colliding_poly = ShapeWithPosition {
        shape: SharedShape::new(Cuboid::new(Vector::new(1.0, 1.0))),
        position: Isometry::new(Vector::new(1.5, 0.0), 0.0),
    };
    assert_vec_approx_eq(get_mtv(&entity, &[colliding_poly]), (0.5, 0.0));
}

#[test]
fn test_circle_halfway_inside_rectangle_horizontally() {
    let entity = ShapeWithPosition {
        shape: SharedShape::new(Ball::new(10.0)),
        position: Isometry::translation(10.0, 0.0),
    };
    let colliding_poly = ShapeWithPosition {
        shape: SharedShape::new(Cuboid::new(Vector::new(10.0, 10.0))),
        position: Isometry::translation(0.0, 0.0),
    };
    let result = get_mtv(&entity, &[colliding_poly]);
    assert_vec_approx_eq(result, (-10.0, 0.0));
}

#[test]
fn test_circle_halfway_inside_rectangle_vertically() {
    let entity = ShapeWithPosition {
        shape: SharedShape::new(Ball::new(10.0)),
        position: Isometry::translation(0.0, 10.0),
    };
    let colliding_poly = ShapeWithPosition {
        shape: SharedShape::new(Cuboid::new(Vector::new(10.0, 10.0))),
        position: Isometry::translation(0.0, 0.0),
    };
    let result = get_mtv(&entity, &[colliding_poly]);
    assert_vec_approx_eq(result, (0.0, -10.0));
}

#[test]
fn test_circle_touching_flat_surface() {
    let entity = ShapeWithPosition {
        shape: SharedShape::new(Ball::new(15.0)),
        position: Isometry::new(Vector::new(0.0, 15.0), 0.0),
    };
    let surface = ShapeWithPosition {
        shape: SharedShape::new(Cuboid::new(Vector::new(1000.0, 1000.0))),
        position: Isometry::new(Vector::new(0.0, -1000.0), 0.0),
    };
    let result = get_mtv(&entity, &[surface]);
    assert_eq!(result, None);
}

#[test]
fn test_circle_halfway_inside_rectangle_and_a_bit_more() {
    let entity = ShapeWithPosition {
        shape: SharedShape::new(Ball::new(15.0)),
        position: Isometry::new(Vector::new(0.0, -1.0), 0.0),
    };
    let colliding_poly = ShapeWithPosition {
        shape: SharedShape::new(Cuboid::new(Vector::new(1000.0, 1000.0))),
        position: Isometry::new(Vector::new(0.0, -1000.0), 0.0),
    };
    assert_vec_approx_eq(get_mtv(&entity, &[colliding_poly]), (0.0, -16.0));
}

#[test]
fn test_circle_halfway_inside_rectangle_and_a_bit_less() {
    let entity = ShapeWithPosition {
        shape: SharedShape::new(Ball::new(15.0)),
        position: Isometry::new(Vector::new(0.0, 1.0), 0.0),
    };
    let colliding_poly = ShapeWithPosition {
        shape: SharedShape::new(Cuboid::new(Vector::new(1000.0, 1000.0))),
        position: Isometry::new(Vector::new(0.0, -1000.0), 0.0),
    };
    assert_vec_approx_eq(get_mtv(&entity, &[colliding_poly]), (0.0, -14.0));
}

#[test]
fn test_axis_aligned_wall_mtv_has_no_tangential_component() {
    let horizontal_wall_entity = ShapeWithPosition {
        shape: SharedShape::new(Ball::new(18.0)),
        position: Isometry::translation(1000.3, 470.0),
    };
    let horizontal_wall = ShapeWithPosition {
        shape: SharedShape::new(Cuboid::new(Vector::new(1600.0, 1000.0))),
        position: Isometry::translation(1600.0, 1480.0),
    };
    let mtv = get_mtv(&horizontal_wall_entity, &[horizontal_wall]).unwrap();
    assert_eq!(mtv.0, 0.0, "horizontal wall introduced x drift: {mtv:?}");
    assert!((mtv.1 - 8.0).abs() < 1e-3, "{mtv:?}");

    let vertical_wall_entity = ShapeWithPosition {
        shape: SharedShape::new(Ball::new(18.0)),
        position: Isometry::translation(310.0, 1000.3),
    };
    let vertical_wall = ShapeWithPosition {
        shape: SharedShape::new(Cuboid::new(Vector::new(1000.0, 1600.0))),
        position: Isometry::translation(1320.0, 1600.0),
    };
    let mtv = get_mtv(&vertical_wall_entity, &[vertical_wall]).unwrap();
    assert!((mtv.0 - 8.0).abs() < 1e-3, "{mtv:?}");
    assert_eq!(mtv.1, 0.0, "vertical wall introduced y drift: {mtv:?}");
}

#[test]
fn test_diagonal_penetration() {
    let entity = ShapeWithPosition {
        shape: SharedShape::new(Ball::new(15.0)),
        position: Isometry::new(Vector::new(500.0, 1.0), 0.0),
    };
    let colliding_poly = ShapeWithPosition {
        shape: SharedShape::new(Cuboid::new(Vector::new(1000.0, 1000.0))),
        position: Isometry::new(Vector::new(1000.0, -1000.0), 0.0),
    };
    assert_vec_approx_eq(get_mtv(&entity, &[colliding_poly]), (0.0, -14.0));
}

#[test]
fn test_overlapping_rectangles() {
    let entity = ShapeWithPosition {
        shape: SharedShape::new(Ball::new(15.0)),
        position: Isometry::new(Vector::new(249397.66076660156, 31855.16436767578), 0.0),
    };
    let colliding_poly1 = ShapeWithPosition {
        shape: SharedShape::new(Cuboid::new(Vector::new(96.0 / 2.0, 480.0 / 2.0))),
        position: Isometry::new(
            Vector::new(249356.0 + 96.0 / 2.0, 31856.0 + 480.0 / 2.0),
            0.0,
        ),
    };
    let colliding_poly2 = ShapeWithPosition {
        shape: SharedShape::new(Cuboid::new(Vector::new(384.0 / 2.0, 96.0 / 2.0))),
        position: Isometry::new(
            Vector::new(249388.0 + 384.0 / 2.0, 31856.0 + 96.0 / 2.0),
            0.0,
        ),
    };
    assert_vec_approx_eq(
        get_mtv(&entity, &[colliding_poly1, colliding_poly2]),
        (0.0, 14.1640625),
    );
}

#[test]
fn test_collision_with_two_rectangles_one_touching() {
    let entity = ShapeWithPosition {
        shape: SharedShape::new(Ball::new(10.0)),
        position: Isometry::new(Vector::new(0.0, 0.0), 0.0),
    };
    let touching_rect = ShapeWithPosition {
        shape: SharedShape::new(Cuboid::new(Vector::new(5.0, 5.0))),
        position: Isometry::new(Vector::new(15.0, 0.0), 0.0),
    };
    let significantly_overlapping_rect = ShapeWithPosition {
        shape: SharedShape::new(Cuboid::new(Vector::new(5.0, 5.0))),
        position: Isometry::new(Vector::new(0.0, 10.0), 0.0),
    };

    let result = get_mtv(&entity, &[touching_rect, significantly_overlapping_rect]);

    assert_vec_approx_eq(result, (0.0, 5.0));
}

#[test]
fn test_two_axis_aligned_constraints_resolve_diagonally() {
    let entity = ShapeWithPosition {
        shape: SharedShape::new(Ball::new(10.0)),
        position: Isometry::translation(0.0, 0.0),
    };
    let right_wall = ShapeWithPosition {
        shape: SharedShape::new(Cuboid::new(Vector::new(10.0, 100.0))),
        position: Isometry::translation(15.0, 0.0),
    };
    let top_wall = ShapeWithPosition {
        shape: SharedShape::new(Cuboid::new(Vector::new(100.0, 10.0))),
        position: Isometry::translation(0.0, 15.0),
    };

    let mtv = get_mtv(&entity, &[right_wall, top_wall]).unwrap();
    assert!((mtv.0 - 5.0).abs() < 1e-3);
    assert!((mtv.1 - 5.0).abs() < 1e-3);
}

#[test]
fn test_rotated_box_collision_resolves() {
    let entity = ShapeWithPosition {
        shape: SharedShape::new(Ball::new(8.0)),
        position: Isometry::translation(0.0, 0.0),
    };
    let rotated_box = ShapeWithPosition {
        shape: SharedShape::new(Cuboid::new(Vector::new(20.0, 6.0))),
        position: Isometry::new(Vector::new(6.0, 0.0), std::f32::consts::FRAC_PI_4),
    };

    let mtv = get_mtv(&entity, &[rotated_box]).unwrap();
    let resolved_position = Isometry::translation(-mtv.0, -mtv.1);
    let resolved_contact = parry2d::query::contact(
        &resolved_position,
        entity.shape.as_ref(),
        &Isometry::new(Vector::new(6.0, 0.0), std::f32::consts::FRAC_PI_4),
        SharedShape::new(Cuboid::new(Vector::new(20.0, 6.0))).as_ref(),
        0.0,
    )
    .unwrap();
    if let Some(contact) = resolved_contact {
        assert!(
            contact.dist >= -1e-2,
            "unexpected residual penetration: {}",
            contact.dist
        );
    }
}

#[test]
fn test_ball_union_chooses_true_minimum_free_point() {
    let entity = ShapeWithPosition {
        shape: SharedShape::new(Ball::new(15.0)),
        position: Isometry::translation(0.0, 0.0),
    };
    let colliding_ball_a = ShapeWithPosition {
        shape: SharedShape::new(Ball::new(33.3)),
        position: Isometry::translation(36.1, 0.4),
    };
    let colliding_ball_b = ShapeWithPosition {
        shape: SharedShape::new(Ball::new(32.6)),
        position: Isometry::translation(-41.8, -14.1),
    };

    let mtv = get_mtv(&entity, &[colliding_ball_a, colliding_ball_b]).unwrap();
    let resolved_position = Isometry::translation(-mtv.0, -mtv.1);

    for obstacle in [
        ShapeWithPosition {
            shape: SharedShape::new(Ball::new(33.3)),
            position: Isometry::translation(36.1, 0.4),
        },
        ShapeWithPosition {
            shape: SharedShape::new(Ball::new(32.6)),
            position: Isometry::translation(-41.8, -14.1),
        },
    ] {
        let resolved_contact = parry2d::query::contact(
            &resolved_position,
            entity.shape.as_ref(),
            &obstacle.position,
            obstacle.shape.as_ref(),
            0.0,
        )
        .unwrap();
        if let Some(contact) = resolved_contact {
            assert!(
                contact.dist >= -1e-2,
                "unexpected residual penetration: {}",
                contact.dist
            );
        }
    }

    assert!((mtv.0 - 8.208_884).abs() < 1e-3, "{mtv:?}");
    assert!((mtv.1 + 19.625_316).abs() < 1e-3, "{mtv:?}");
}

#[test]
fn test_rotated_cuboid_chooses_true_minimum_free_distance() {
    let entity = ShapeWithPosition {
        shape: SharedShape::new(Ball::new(8.0)),
        position: Isometry::translation(0.0, 0.0),
    };
    let rotated_box = ShapeWithPosition {
        shape: SharedShape::new(Cuboid::new(Vector::new(20.0, 6.0))),
        position: Isometry::new(Vector::new(6.0, 0.0), std::f32::consts::FRAC_PI_4),
    };

    let mtv = get_mtv(&entity, &[rotated_box]).unwrap();
    let resolved_position = Isometry::translation(-mtv.0, -mtv.1);
    let resolved_contact = parry2d::query::contact(
        &resolved_position,
        entity.shape.as_ref(),
        &Isometry::new(Vector::new(6.0, 0.0), std::f32::consts::FRAC_PI_4),
        SharedShape::new(Cuboid::new(Vector::new(20.0, 6.0))).as_ref(),
        0.0,
    )
    .unwrap();
    if let Some(contact) = resolved_contact {
        assert!(
            contact.dist >= -1e-2,
            "unexpected residual penetration: {}",
            contact.dist
        );
    }

    let mtv_length = (mtv.0 * mtv.0 + mtv.1 * mtv.1).sqrt();
    assert!((mtv_length - 9.757_359).abs() < 2e-2, "{mtv:?}");
}
