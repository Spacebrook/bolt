use ncollide2d::math::{Isometry, Vector};
use ncollide2d::query::{self, ClosestPoints};
use ncollide2d::shape::{Ball, Cuboid, Shape};

pub struct ShapeWithPosition {
    pub shape: Box<dyn Shape<f32>>,
    pub position: Isometry<f32>,
}

pub fn get_mtv(
    entity: &ShapeWithPosition,
    colliding_polys: Vec<ShapeWithPosition>,
) -> Option<(f32, f32)> {
    if colliding_polys.is_empty() {
        return None;
    }

    let mut largest_mtv: Vector<f32> = Vector::zeros();

    for colliding_poly in &colliding_polys {
        let closest_points = query::closest_points(
            &entity.position,
            entity.shape.as_ref(),
            &colliding_poly.position,
            colliding_poly.shape.as_ref(),
            f32::EPSILON,
        );

        match closest_points {
            ClosestPoints::Intersecting => {
                if let (Some(entity_half_extents), Some(colliding_poly_half_extents)) = (
                    get_half_extents(entity.shape.as_ref()),
                    get_half_extents(colliding_poly.shape.as_ref()),
                ) {
                    let distance = entity.position.translation.vector
                        - colliding_poly.position.translation.vector;
                    let total_half_extents = entity_half_extents + colliding_poly_half_extents;
                    let penetration = total_half_extents - distance.abs();

                    let mut mtv = Vector::zeros();
                    if penetration.x.abs() < penetration.y.abs() {
                        mtv.x = -penetration.x * distance.x.signum();
                    } else {
                        mtv.y = -penetration.y * distance.y.signum();
                    }

                    if mtv.x.abs() > largest_mtv.x.abs() {
                        largest_mtv.x = mtv.x;
                    }
                    if mtv.y.abs() > largest_mtv.y.abs() {
                        largest_mtv.y = mtv.y;
                    }
                }
            }
            _ => (),
        }
    }

    if largest_mtv.norm() < f32::EPSILON {
        None
    } else {
        Some((largest_mtv.x, largest_mtv.y))
    }
}

fn get_half_extents(shape: &dyn Shape<f32>) -> Option<Vector<f32>> {
    if let Some(ball) = shape.as_shape::<Ball<f32>>() {
        Some(Vector::repeat(ball.radius))
    } else if let Some(cuboid) = shape.as_shape::<Cuboid<f32>>() {
        Some(cuboid.half_extents)
    } else {
        None
    }
}
