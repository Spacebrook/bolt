use rand::Rng;
use std::fmt::Debug;

pub trait Shape: Debug {
    fn bounding_box(&self) -> Rectangle;
    fn as_any(&self) -> &dyn std::any::Any;
}

#[derive(Debug, Copy, Clone)]
pub struct Circle {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub bounding_box: Rectangle,
}

impl Circle {
    pub fn new(x: f32, y: f32, radius: f32) -> Self {
        let bounding_box = Rectangle {
            x: x,
            y: y,
            width: radius * 2.0,
            height: radius * 2.0,
        };
        Self {
            x,
            y,
            radius,
            bounding_box,
        }
    }

    pub fn x(&self) -> f32 {
        self.x
    }

    pub fn y(&self) -> f32 {
        self.y
    }

    pub fn radius(&self) -> f32 {
        self.radius
    }

    pub fn update(&mut self, x: f32, y: f32) {
        self.x = x;
        self.y = y;
        self.update_bounding_box();
    }

    pub fn update_with_radius(&mut self, x: f32, y: f32, radius: f32) {
        self.x = x;
        self.y = y;
        self.radius = radius;
        self.update_bounding_box();
    }

    // Helper method to update the bounding box
    fn update_bounding_box(&mut self) {
        self.bounding_box = Rectangle {
            x: self.x - self.radius,
            y: self.y - self.radius,
            width: self.radius * 2.0,
            height: self.radius * 2.0,
        };
    }
}

impl Default for Circle {
    fn default() -> Self {
        let default_rectangle = Rectangle::default();
        Self {
            x: 0.0,
            y: 0.0,
            radius: 0.0,
            bounding_box: default_rectangle,
        }
    }
}

impl Shape for Circle {
    fn bounding_box(&self) -> Rectangle {
        self.bounding_box
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Rectangle {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rectangle {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn width(&self) -> f32 {
        self.width
    }

    pub fn height(&self) -> f32 {
        self.height
    }

    pub fn left(&self) -> f32 {
        self.x - self.width / 2.0
    }

    pub fn right(&self) -> f32 {
        self.x + self.width / 2.0
    }

    pub fn top(&self) -> f32 {
        self.y - self.height / 2.0
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.height / 2.0
    }

    pub fn top_left(&self) -> (f32, f32) {
        (self.left(), self.top())
    }

    pub fn top_right(&self) -> (f32, f32) {
        (self.right(), self.top())
    }

    pub fn bottom_left(&self) -> (f32, f32) {
        (self.left(), self.bottom())
    }

    pub fn bottom_right(&self) -> (f32, f32) {
        (self.right(), self.bottom())
    }

    pub fn distance_to_point(&self, x: f32, y: f32) -> f32 {
        let dx = (x - self.x).abs() - self.width / 2.0;
        let dy = (y - self.y).abs() - self.height / 2.0;
        f32::max(dx, 0.0).powi(2) + f32::max(dy, 0.0).powi(2)
    }

    pub fn contains_circle(&self, x: f32, y: f32, radius: f32) -> bool {
        let dx = (x - self.x).abs();
        let dy = (y - self.y).abs();
        let half_width = self.width / 2.0;
        let half_height = self.height / 2.0;
        if dx > half_width + radius || dy > half_height + radius {
            return false;
        }
        if dx <= half_width || dy <= half_height {
            return true;
        }
        let corner_distance_sq = (dx - half_width).powi(2) + (dy - half_height).powi(2);
        corner_distance_sq <= radius.powi(2)
    }

    pub fn contains_point(&self, x: f32, y: f32) -> bool {
        x >= self.left() && x <= self.right() && y >= self.top() && y <= self.bottom()
    }

    pub fn expand_to_include(&mut self, other: &Rectangle) {
        let left = f32::min(self.left(), other.left());
        let right = f32::max(self.right(), other.right());
        let top = f32::min(self.top(), other.top());
        let bottom = f32::max(self.bottom(), other.bottom());
        self.x = (left + right) / 2.0;
        self.y = (top + bottom) / 2.0;
        self.width = right - left;
        self.height = bottom - top;
    }

    pub fn get_random_circle_coords_inside<R: Rng>(&self, radius: f32, rng: &mut R) -> (f32, f32) {
        // Increase radius by 1 in calculations to add a minimal margin.
        let radius = radius + 1.0;
        (
            self._safe_randf32(rng, self.left() + radius, self.right() - radius),
            self._safe_randf32(rng, self.top() + radius, self.bottom() - radius),
        )
    }

    fn _safe_randf32<R: Rng>(&self, rng: &mut R, min: f32, max: f32) -> f32 {
        if min > max {
            return min;
        }
        rng.gen_range(min..=max)
    }
}

impl Default for Rectangle {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        }
    }
}

impl Shape for Rectangle {
    fn bounding_box(&self) -> Rectangle {
        *self
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Clone, Debug)]
pub enum ShapeEnum {
    Circle(Circle),
    Rectangle(Rectangle),
}

impl Shape for ShapeEnum {
    fn bounding_box(&self) -> Rectangle {
        match self {
            ShapeEnum::Circle(circle) => circle.bounding_box(),
            ShapeEnum::Rectangle(rectangle) => rectangle.bounding_box(),
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        match self {
            ShapeEnum::Circle(circle) => circle.as_any(),
            ShapeEnum::Rectangle(rectangle) => rectangle.as_any(),
        }
    }
}
