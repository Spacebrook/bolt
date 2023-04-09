use pyo3::pyclass;

use ::collisions::ShapeWithPosition;
use ::quadtree::shapes::{Circle, Rectangle, ShapeEnum};
use ncollide2d::math::{Isometry, Vector};
use ncollide2d::shape::{Ball, Cuboid};
use pyo3::exceptions::PyTypeError;
use pyo3::pymethods;
use pyo3::types::PyList;
use pyo3::{PyObject, PyResult, Python};

pub mod collisions;
pub mod quadtree;

#[derive(Debug, Clone)]
#[pyclass(name = "Circle")]
struct PyCircle {
    x: f32,
    y: f32,
    radius: f32,
}

#[pymethods]
impl PyCircle {
    #[new]
    pub fn new(x: f32, y: f32, radius: f32) -> Self {
        PyCircle { x, y, radius }
    }
}

#[derive(Debug, Clone)]
#[pyclass(name = "Rectangle")]
struct PyRectangle {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

#[pymethods]
impl PyRectangle {
    #[new]
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        PyRectangle {
            x,
            y,
            width,
            height,
        }
    }
}

fn extract_shape(py: Python, shape: PyObject) -> PyResult<ShapeEnum> {
    if let Ok(py_rectangle) = shape.extract::<PyRectangle>(py) {
        Ok(ShapeEnum::Rectangle(Rectangle {
            x: py_rectangle.x,
            y: py_rectangle.y,
            width: py_rectangle.width,
            height: py_rectangle.height,
        }))
    } else if let Ok(py_circle) = shape.extract::<PyCircle>(py) {
        Ok(ShapeEnum::Circle(Circle::new(
            py_circle.x,
            py_circle.y,
            py_circle.radius,
        )))
    } else {
        Err(PyTypeError::new_err(
            "Expected a Rectangle or Circle object",
        ))
    }
}

fn extract_shape_ncollide(py: Python, shape: PyObject) -> PyResult<ShapeWithPosition> {
    let shape = extract_shape(py, shape)?;
    match shape {
        ShapeEnum::Circle(shape) => Ok(ShapeWithPosition {
            shape: Box::new(Ball::new(shape.radius)),
            position: Isometry::new(Vector::new(shape.x, shape.y), 0.0),
        }),
        ShapeEnum::Rectangle(shape) => Ok(ShapeWithPosition {
            shape: Box::new(Cuboid::new(Vector::new(
                shape.width / 2.0,
                shape.height / 2.0,
            ))),
            position: Isometry::new(Vector::new(shape.x, shape.y), 0.0),
        }),
    }
}

fn extract_entity_types(entity_types: Option<&PyList>) -> PyResult<Option<Vec<u32>>> {
    match entity_types {
        Some(entity_types_list) => {
            let et: Result<Vec<u32>, _> = entity_types_list
                .iter()
                .map(|item| item.extract::<u32>())
                .collect();
            Ok(Some(et?))
        }
        None => Ok(None),
    }
}
