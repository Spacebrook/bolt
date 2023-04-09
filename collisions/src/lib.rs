use ncollide2d::math::{Isometry, Vector};
use ncollide2d::query::{self, Contact};
use ncollide2d::shape::Shape;

pub struct ShapeWithPosition {
    pub shape: Box<dyn Shape<f32>>,
    pub position: Isometry<f32>,
}

pub fn get_mtv(
    entity: &ShapeWithPosition,
    colliding_polys: Vec<ShapeWithPosition>,
) -> Option<(f32, f32)> {
    // Early return if there are no colliding polygons
    if colliding_polys.is_empty() {
        return None;
    }

    let mut mtv = Vector::zeros();

    // Accumulate overlap vectors for each collision
    for colliding_poly in colliding_polys {
        let contact = query::contact(
            &entity.position,
            entity.shape.as_ref(),
            &colliding_poly.position,
            colliding_poly.shape.as_ref(),
            f32::EPSILON,
        );
        if let Some(Contact { normal, depth, .. }) = contact {
            mtv += normal.as_ref() * depth;
        }
    }

    let epsilon = f32::EPSILON;
    if mtv.norm() < epsilon {
        // No collision if the length of mtv is close to zero
        return None;
    }

    // Convert the result to tuple
    let result_mtv = Some((mtv.x, mtv.y));
    result_mtv
}
