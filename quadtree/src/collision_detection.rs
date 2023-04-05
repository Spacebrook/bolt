use crate::shapes::{Circle, Rectangle, ShapeEnum};

// Check that Rectangle inner is fully contained in Rectangle outer, including on the boundary
pub fn rectangle_contains_rectangle(outer: &Rectangle, inner: &Rectangle) -> bool {
    outer.x <= inner.x
        && outer.right() >= inner.right()
        && outer.y <= inner.y
        && outer.bottom() >= inner.bottom()
}

pub fn rectangle_rectangle(a: &Rectangle, b: &Rectangle) -> bool {
    a.x < b.right() && a.right() > b.x && a.y < b.bottom() && a.bottom() > b.y
}

pub fn circle_circle(a: &Circle, b: &Circle) -> bool {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let distance_sq = dx * dx + dy * dy;
    distance_sq < (a.radius + b.radius) * (a.radius + b.radius)
}

pub fn circle_rectangle(circle: &Circle, rectangle: &Rectangle) -> bool {
    let circle_distance_x = (circle.x - rectangle.center_x()).abs();
    let circle_distance_y = (circle.y - rectangle.center_y()).abs();

    let half_rect_width = rectangle.width / 2.0;
    let half_rect_height = rectangle.height / 2.0;

    if circle_distance_x > half_rect_width + circle.radius {
        return false;
    }
    if circle_distance_y > half_rect_height + circle.radius {
        return false;
    }

    if circle_distance_x <= half_rect_width || circle_distance_y <= half_rect_height {
        return true;
    }

    let corner_dx = circle_distance_x - half_rect_width;
    let corner_dy = circle_distance_y - half_rect_height;
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
