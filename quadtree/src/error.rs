use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QuadtreeError {
    InvalidRectangleDims { width: f32, height: f32 },
    InvalidCircleRadius { radius: f32 },
    InvalidRectExtent {
        min_x: f32,
        min_y: f32,
        max_x: f32,
        max_y: f32,
    },
    RectExtentOutOfBounds {
        min_x: f32,
        min_y: f32,
        max_x: f32,
        max_y: f32,
        bounds_min_x: f32,
        bounds_min_y: f32,
        bounds_max_x: f32,
        bounds_max_y: f32,
    },
}

pub type QuadtreeResult<T> = Result<T, QuadtreeError>;

impl fmt::Display for QuadtreeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QuadtreeError::InvalidRectangleDims { width, height } => {
                write!(
                    f,
                    "rectangle width/height must be finite and non-negative (width: {}, height: {})",
                    width, height
                )
            }
            QuadtreeError::InvalidCircleRadius { radius } => {
                write!(
                    f,
                    "circle radius must be finite and non-negative (radius: {})",
                    radius
                )
            }
            QuadtreeError::InvalidRectExtent {
                min_x,
                min_y,
                max_x,
                max_y,
            } => {
                write!(
                    f,
                    "rectangle extents must be finite with min <= max (min_x: {}, min_y: {}, max_x: {}, max_y: {})",
                    min_x, min_y, max_x, max_y
                )
            }
            QuadtreeError::RectExtentOutOfBounds {
                min_x,
                min_y,
                max_x,
                max_y,
                bounds_min_x,
                bounds_min_y,
                bounds_max_x,
                bounds_max_y,
            } => {
                write!(
                    f,
                    "rectangle extents must be within quadtree bounds (min_x: {}, min_y: {}, max_x: {}, max_y: {}, bounds_min_x: {}, bounds_min_y: {}, bounds_max_x: {}, bounds_max_y: {})",
                    min_x,
                    min_y,
                    max_x,
                    max_y,
                    bounds_min_x,
                    bounds_min_y,
                    bounds_max_x,
                    bounds_max_y
                )
            }
        }
    }
}

impl std::error::Error for QuadtreeError {}
