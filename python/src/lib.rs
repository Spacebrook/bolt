use ::collisions::ShapeWithPosition;
use bolt_quadtree::shapes::{Circle, Rectangle, ShapeEnum};
use parry2d::math::{Isometry, Vector};
use parry2d::shape::{Ball, Cuboid, SharedShape};
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::pymethods;
use pyo3::types::{PyAnyMethods, PyModule, PyModuleMethods};
use pyo3::{Bound, PyResult, Python};

mod collisions;
mod netcode;
mod serialization;

use crate::collisions::get_mtv;
use crate::netcode::NetCodec;
use crate::serialization::DiffFieldSetWrapper;

#[pymodule]
fn pycollisions(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(get_mtv, m)?)?;
    Ok(())
}

#[pymodule]
fn pyserialization(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<DiffFieldSetWrapper>()?;
    Ok(())
}

#[pymodule]
fn bolt(py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    let submod_collisions = PyModule::new(py, "collisions")?;
    pycollisions(py, &submod_collisions)?;
    m.add_submodule(&submod_collisions)?;

    let quadtree_mod = PyModule::import(py, "bolt_quadtree")?;
    for name in ["QuadTree", "Config", "Rectangle", "Circle"] {
        let obj = quadtree_mod.getattr(name)?;
        m.add(name, obj)?;
    }

    m.add_class::<DiffFieldSetWrapper>()?;
    m.add_class::<NetCodec>()?;

    m.add_class::<PySquare>()?;

    Ok(())
}

#[derive(Clone, Debug)]
#[pyclass(name = "Square")]
pub struct PySquare {
    #[pyo3(get, set)]
    pub x: f32,
    #[pyo3(get, set)]
    pub y: f32,
    #[pyo3(get, set)]
    pub radius: f32,
    #[pyo3(get, set)]
    pub angle: f32,
}

#[pymethods]
impl PySquare {
    #[new]
    pub fn new(x: f32, y: f32, radius: f32, angle: f32) -> Self {
        PySquare {
            x,
            y,
            radius,
            angle,
        }
    }
}

fn extract_shape(py: Python, shape: Py<PyAny>) -> PyResult<ShapeEnum> {
    let shape = shape.bind(py);
    if let (Ok(width_obj), Ok(height_obj)) = (shape.getattr("width"), shape.getattr("height")) {
        let x = shape.getattr("x")?.extract::<f32>()?;
        let y = shape.getattr("y")?.extract::<f32>()?;
        let width = width_obj.extract::<f32>()?;
        let height = height_obj.extract::<f32>()?;
        let center_x = x + width * 0.5;
        let center_y = y + height * 0.5;
        return Ok(ShapeEnum::Rectangle(Rectangle::new(
            center_x, center_y, width, height,
        )));
    }
    if let Ok(radius_obj) = shape.getattr("radius") {
        let x = shape.getattr("x")?.extract::<f32>()?;
        let y = shape.getattr("y")?.extract::<f32>()?;
        let radius = radius_obj.extract::<f32>()?;
        return Ok(ShapeEnum::Circle(Circle::new(x, y, radius)));
    }
    Err(PyTypeError::new_err(
        "Expected a Rectangle or Circle-like object",
    ))
}

fn extract_shape_ncollide(py: Python, shape: Py<PyAny>) -> PyResult<ShapeWithPosition> {
    if let Ok(py_square) = shape.extract::<PySquare>(py) {
        return Ok(ShapeWithPosition {
            shape: SharedShape::new(Cuboid::new(Vector::new(py_square.radius, py_square.radius))),
            position: Isometry::new(Vector::new(py_square.x, py_square.y), py_square.angle),
        });
    }

    let shape = extract_shape(py, shape)?;
    match shape {
        ShapeEnum::Circle(shape) => Ok(ShapeWithPosition {
            shape: SharedShape::new(Ball::new(shape.radius)),
            position: Isometry::translation(shape.x, shape.y),
        }),
        ShapeEnum::Rectangle(shape) => Ok(ShapeWithPosition {
            shape: SharedShape::new(Cuboid::new(Vector::new(
                shape.width / 2.0,
                shape.height / 2.0,
            ))),
            position: Isometry::translation(shape.x, shape.y),
        }),
    }
}
