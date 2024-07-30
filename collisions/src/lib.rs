use nalgebra::{Isometry2, Vector2};
use parry2d::shape::SharedShape;

pub struct ShapeWithPosition {
    pub shape: SharedShape,
    pub position: Isometry2<f32>,
}

pub fn get_mtv(entity: &ShapeWithPosition, others: &[ShapeWithPosition]) -> Option<(f32, f32)> {
    if let (Some(circle), true) = (
        entity.shape.as_ball(),
        others.iter().all(|s| s.shape.as_cuboid().is_some()),
    ) {
        // Existing circle-rectangle collision logic
        let circle_radius = circle.radius;
        let circle_center = entity.position.translation.vector;

        let mut max_mtv: Vector2<f32> = Vector2::new(0.0, 0.0);

        for rect in others {
            let cuboid = rect.shape.as_cuboid().unwrap();
            let rect_center = rect.position.translation.vector;
            let rect_half_extents = cuboid.half_extents;
            let rect_rotation = rect.position.rotation;

            // Calculate the vector from rectangle center to circle center
            let to_circle = circle_center - rect_center;

            // Rotate the vector to align with the rectangle's local space
            let local_circle_pos = rect_rotation.inverse() * to_circle;

            // Find the closest point on the rectangle to the circle center
            let closest = Vector2::new(
                local_circle_pos
                    .x
                    .clamp(-rect_half_extents.x, rect_half_extents.x),
                local_circle_pos
                    .y
                    .clamp(-rect_half_extents.y, rect_half_extents.y),
            );

            // Calculate the vector from the closest point to the circle center
            let to_circle = local_circle_pos - closest;
            let distance = to_circle.magnitude();

            if distance <= circle_radius {
                // Collision detected
                let penetration = if distance == 0.0 {
                    // Circle center is inside the rectangle
                    let dx = rect_half_extents.x - local_circle_pos.x.abs();
                    let dy = rect_half_extents.y - local_circle_pos.y.abs();
                    if dx < dy {
                        rect_half_extents.x + circle_radius - local_circle_pos.x.abs()
                    } else {
                        rect_half_extents.y + circle_radius - local_circle_pos.y.abs()
                    }
                } else {
                    circle_radius - distance
                };

                let normal = if distance == 0.0 {
                    // Use the axis of least penetration
                    let dx = rect_half_extents.x - local_circle_pos.x.abs();
                    let dy = rect_half_extents.y - local_circle_pos.y.abs();
                    if dx < dy {
                        Vector2::new(local_circle_pos.x.signum(), 0.0)
                    } else {
                        Vector2::new(0.0, local_circle_pos.y.signum())
                    }
                } else {
                    to_circle.normalize()
                };

                // Rotate the normal back to world space
                let world_normal = rect_rotation * normal;
                let mtv = world_normal * penetration;

                if mtv.x.abs() > max_mtv.x.abs() {
                    max_mtv.x = mtv.x;
                }
                if mtv.y.abs() > max_mtv.y.abs() {
                    max_mtv.y = mtv.y;
                }
            }
        }

        let magnitude = max_mtv.magnitude();

        if magnitude < 1e-6 {
            None
        } else {
            Some((-max_mtv.x, -max_mtv.y))
        }
    } else {
        // General case for any shape combination
        let max_mtv: Vector2<f32> = others
            .iter()
            .filter_map(|other| {
                parry2d::query::contact(
                    &entity.position,
                    entity.shape.as_ref(),
                    &other.position,
                    other.shape.as_ref(),
                    0.001,
                )
                .ok()
                .flatten()
            })
            .fold(Vector2::new(0.0, 0.0), |mut max_mtv, contact| {
                let mtv = contact.normal1.into_inner() * contact.dist.abs();
                if mtv.x.abs() > max_mtv.x.abs() {
                    max_mtv.x = mtv.x;
                }
                if mtv.y.abs() > max_mtv.y.abs() {
                    max_mtv.y = mtv.y;
                }
                max_mtv
            });

        let magnitude = max_mtv.magnitude();

        if magnitude < 1e-6 {
            None
        } else {
            Some((-max_mtv.x, -max_mtv.y))
        }
    }
}