use common::shapes::{Circle, Rectangle, ShapeEnum};

// Check that Rectangle inner is fully contained in Rectangle outer, including on the boundary
pub fn rectangle_contains_rectangle(outer: &Rectangle, inner: &Rectangle) -> bool {
    outer.left() <= inner.left()
        && outer.right() >= inner.right()
        && outer.top() <= inner.top()
        && outer.bottom() >= inner.bottom()
}

pub fn rectangle_rectangle(a: &Rectangle, b: &Rectangle) -> bool {
    a.left() < b.right() && a.right() > b.left() && a.top() < b.bottom() && a.bottom() > b.top()
}

pub fn circle_circle(a: &Circle, b: &Circle) -> bool {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let distance_sq = dx * dx + dy * dy;
    let radius_sum = a.radius + b.radius;
    distance_sq < radius_sum * radius_sum
}

pub fn circle_rectangle(circle: &Circle, rectangle: &Rectangle) -> bool {
    let dx = (circle.x - rectangle.x).abs();
    let dy = (circle.y - rectangle.y).abs();

    let half_rect_width = rectangle.width / 2.0;
    let half_rect_height = rectangle.height / 2.0;

    // Check if the circle is outside the rectangle's bounds
    if dx > half_rect_width + circle.radius || dy > half_rect_height + circle.radius {
        return false;
    }

    // Check if the circle's center is inside the rectangle
    if dx <= half_rect_width || dy <= half_rect_height {
        return true;
    }

    // Check if the circle intersects the rectangle's corner
    let corner_dx = dx - half_rect_width;
    let corner_dy = dy - half_rect_height;
    let corner_distance_sq = corner_dx * corner_dx + corner_dy * corner_dy;

    corner_distance_sq <= circle.radius * circle.radius
}

pub fn shape_shape(a: &ShapeEnum, b: &ShapeEnum) -> bool {
    match (a, b) {
        (ShapeEnum::Circle(circle_a), ShapeEnum::Circle(circle_b)) => {
            circle_circle(circle_a, circle_b)
        }
        (ShapeEnum::Circle(circle), ShapeEnum::Rectangle(rectangle))
        | (ShapeEnum::Rectangle(rectangle), ShapeEnum::Circle(circle)) => {
            circle_rectangle(circle, rectangle)
        }
        (ShapeEnum::Rectangle(rectangle_a), ShapeEnum::Rectangle(rectangle_b)) => {
            rectangle_rectangle(rectangle_a, rectangle_b)
        }
    }
}
