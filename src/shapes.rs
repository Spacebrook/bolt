use std::fmt::Debug;

pub trait Shape: Debug {
    fn bounding_box(&self) -> Rectangle;
    fn as_any(&self) -> &dyn std::any::Any;
    fn box_clone(&self) -> Box<dyn Shape>;
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
            x: x - radius,
            y: y - radius,
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

    fn box_clone(&self) -> Box<dyn Shape> {
        Box::new(self.clone())
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
        Self { x, y, width, height }
    }

    pub fn x(&self) -> f32 {
        self.x
    }

    pub fn y(&self) -> f32 {
        self.y
    }

    pub fn width(&self) -> f32 {
        self.width
    }

    pub fn height(&self) -> f32 {
        self.height
    }

    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }

    pub fn center_x(&self) -> f32 {
        self.x + self.width / 2.0
    }

    pub fn center_y(&self) -> f32 {
        self.y + self.height / 2.0
    }
}

impl Default for Rectangle {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0, width: 0.0, height: 0.0 }
    }
}

impl Shape for Rectangle {
    fn bounding_box(&self) -> Rectangle {
        *self
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn box_clone(&self) -> Box<dyn Shape> {
        Box::new(self.clone())
    }
}
