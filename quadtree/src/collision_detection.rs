use crate::error::{QuadtreeError, QuadtreeResult};
use common::shapes::{Circle, Rectangle, ShapeEnum};

// Check that Rectangle inner is fully contained in Rectangle outer, including on the boundary
pub fn rectangle_contains_rectangle(outer: &Rectangle, inner: &Rectangle) -> QuadtreeResult<bool> {
    if !(outer.width.is_finite() && outer.height.is_finite()) || outer.width < 0.0 || outer.height < 0.0 {
        return Err(QuadtreeError::InvalidRectangleDims {
            width: outer.width,
            height: outer.height,
        });
    }
    if !(inner.width.is_finite() && inner.height.is_finite()) || inner.width < 0.0 || inner.height < 0.0 {
        return Err(QuadtreeError::InvalidRectangleDims {
            width: inner.width,
            height: inner.height,
        });
    }
    Ok(outer.left() <= inner.left()
        && outer.right() >= inner.right()
        && outer.top() <= inner.top()
        && outer.bottom() >= inner.bottom())
}

/// Note: touching edges are not treated as collisions.
pub fn rectangle_rectangle(a: &Rectangle, b: &Rectangle) -> QuadtreeResult<bool> {
    if !(a.width.is_finite() && a.height.is_finite()) || a.width < 0.0 || a.height < 0.0 {
        return Err(QuadtreeError::InvalidRectangleDims {
            width: a.width,
            height: a.height,
        });
    }
    if !(b.width.is_finite() && b.height.is_finite()) || b.width < 0.0 || b.height < 0.0 {
        return Err(QuadtreeError::InvalidRectangleDims {
            width: b.width,
            height: b.height,
        });
    }
    Ok(a.left() < b.right() && a.right() > b.left() && a.top() < b.bottom() && a.bottom() > b.top())
}

/// Note: touching edges are not treated as collisions.
pub fn circle_circle(a: &Circle, b: &Circle) -> QuadtreeResult<bool> {
    if !(a.radius.is_finite() && a.radius >= 0.0) {
        return Err(QuadtreeError::InvalidCircleRadius { radius: a.radius });
    }
    if !(b.radius.is_finite() && b.radius >= 0.0) {
        return Err(QuadtreeError::InvalidCircleRadius { radius: b.radius });
    }
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let distance_sq = dx * dx + dy * dy;
    let radius_sum = a.radius + b.radius;
    Ok(distance_sq < radius_sum * radius_sum)
}

/// Note: touching edges are not treated as collisions.
pub fn circle_rectangle(circle: &Circle, rectangle: &Rectangle) -> QuadtreeResult<bool> {
    if !(circle.radius.is_finite() && circle.radius >= 0.0) {
        return Err(QuadtreeError::InvalidCircleRadius {
            radius: circle.radius,
        });
    }
    if !(rectangle.width.is_finite() && rectangle.height.is_finite())
        || rectangle.width < 0.0
        || rectangle.height < 0.0
    {
        return Err(QuadtreeError::InvalidRectangleDims {
            width: rectangle.width,
            height: rectangle.height,
        });
    }
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
