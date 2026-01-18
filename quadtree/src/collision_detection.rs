use crate::error::QuadtreeResult;
use crate::quadtree::{validate_circle_radius, validate_rect_dims};
use common::shapes::{Circle, Rectangle, ShapeEnum};

// Check that Rectangle inner is fully contained in Rectangle outer, including on the boundary
pub fn rectangle_contains_rectangle(outer: &Rectangle, inner: &Rectangle) -> QuadtreeResult<bool> {
    validate_rect_dims(outer.width, outer.height)?;
    validate_rect_dims(inner.width, inner.height)?;
    Ok(outer.left() <= inner.left()
        && outer.right() >= inner.right()
        && outer.top() <= inner.top()
        && outer.bottom() >= inner.bottom())
}

/// Note: touching edges are not treated as collisions.
pub fn rectangle_rectangle(a: &Rectangle, b: &Rectangle) -> QuadtreeResult<bool> {
    validate_rect_dims(a.width, a.height)?;
    validate_rect_dims(b.width, b.height)?;
    Ok(a.left() < b.right() && a.right() > b.left() && a.top() < b.bottom() && a.bottom() > b.top())
}

/// Note: touching edges are not treated as collisions.
pub fn circle_circle(a: &Circle, b: &Circle) -> QuadtreeResult<bool> {
    validate_circle_radius(a.radius)?;
    validate_circle_radius(b.radius)?;
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let distance_sq = dx * dx + dy * dy;
    let radius_sum = a.radius + b.radius;
    Ok(distance_sq < radius_sum * radius_sum)
}

/// Note: touching edges are not treated as collisions.
pub fn circle_rectangle(circle: &Circle, rectangle: &Rectangle) -> QuadtreeResult<bool> {
    validate_circle_radius(circle.radius)?;
    validate_rect_dims(rectangle.width, rectangle.height)?;
    let dx = (circle.x - rectangle.x).abs();
    let dy = (circle.y - rectangle.y).abs();

    let half_rect_width = rectangle.width / 2.0;
    let half_rect_height = rectangle.height / 2.0;

    // Check if the circle is outside the rectangle's bounds
    if dx >= half_rect_width + circle.radius || dy >= half_rect_height + circle.radius {
        return Ok(false);
    }

    // Check if the circle's center is inside the rectangle
    if dx < half_rect_width || dy < half_rect_height {
        return Ok(true);
    }

    // Check if the circle intersects the rectangle's corner
    let corner_dx = dx - half_rect_width;
    let corner_dy = dy - half_rect_height;
    let corner_distance_sq = corner_dx * corner_dx + corner_dy * corner_dy;

    Ok(corner_distance_sq < circle.radius * circle.radius)
}

/// Note: touching edges are not treated as collisions.
pub fn shape_shape(a: &ShapeEnum, b: &ShapeEnum) -> QuadtreeResult<bool> {
    match (a, b) {
        (ShapeEnum::Circle(circle_a), ShapeEnum::Circle(circle_b)) => circle_circle(circle_a, circle_b),
        (ShapeEnum::Circle(circle), ShapeEnum::Rectangle(rectangle))
        | (ShapeEnum::Rectangle(rectangle), ShapeEnum::Circle(circle)) => {
            circle_rectangle(circle, rectangle)
        }
        (ShapeEnum::Rectangle(rectangle_a), ShapeEnum::Rectangle(rectangle_b)) => {
            rectangle_rectangle(rectangle_a, rectangle_b)
        }
    }
}
