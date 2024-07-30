use nalgebra::{Vector2, Isometry2};
use parry2d::shape::{SharedShape};

pub struct ShapeWithPosition {
    pub shape: SharedShape,
    pub position: Isometry2<f32>,
}

pub fn get_mtv(
    circle: &ShapeWithPosition,
    rectangles: &[ShapeWithPosition],
) -> Option<(f32, f32)> {
    // Ensure the circle is actually a circle
    let circle_radius = circle.shape.as_ball()?.radius;

    let circle_center = circle.position.translation.vector;

    rectangles.iter()
        .filter_map(|rect| {
            // Ensure the rectangle is actually a rectangle
            let cuboid = rect.shape.as_cuboid()?;

            let rect_center = rect.position.translation.vector;
            let rect_half_extents = cuboid.half_extents;
            let rect_rotation = rect.position.rotation;

            // Calculate the vector from rectangle center to circle center
            let to_circle = circle_center - rect_center;

            // Rotate the vector to align with the rectangle's local space
            let local_circle_pos = rect_rotation.inverse() * to_circle;

            // Find the closest point on the rectangle to the circle center
            let closest = Vector2::new(
                local_circle_pos.x.clamp(-rect_half_extents.x, rect_half_extents.x),
                local_circle_pos.y.clamp(-rect_half_extents.y, rect_half_extents.y),
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
                Some((world_normal.x * -penetration, world_normal.y * -penetration))
            } else {
                None // No collision
            }
        })
        .min_by(|a, b| {
            let a_mag = (a.0 * a.0 + a.1 * a.1).sqrt();
            let b_mag = (b.0 * b.0 + b.1 * b.1).sqrt();
            a_mag.partial_cmp(&b_mag).unwrap_or(std::cmp::Ordering::Equal)
        })
        .and_then(|(x, y)| {
            let magnitude = (x * x + y * y).sqrt();
            if magnitude < 1e-6 {
                None
            } else {
                Some((x, y))
            }
        })
}
