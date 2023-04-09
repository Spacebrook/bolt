use ::quadtree::quadtree::{Config, QuadTree, RelocationRequest};
use ::quadtree::shapes::{Circle, Rectangle, Shape, ShapeEnum};

use crate::{extract_entity_types, extract_shape, PyCircle, PyRectangle};
use pyo3::exceptions::PyTypeError;
#[cfg(feature = "pyo3")]
use pyo3::prelude::*;
use pyo3::pyclass;
use pyo3::pymethods;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use pyo3::IntoPy;
use pyo3::Py;
use pyo3::PyObject;
use pyo3::PyResult;
use pyo3::Python;

#[derive(Clone)]
#[pyclass(name = "Config")]
pub struct PyConfig {
    pool_size: usize,
    node_capacity: usize,
    max_depth: usize,
}

#[pymethods]
impl PyConfig {
    #[new]
    pub fn new(pool_size: usize, node_capacity: usize, max_depth: usize) -> Self {
        PyConfig {
            pool_size,
            node_capacity,
            max_depth,
        }
    }
}

#[pyclass(name = "QuadTree", unsendable)]
pub struct QuadTreeWrapper {
    quadtree: QuadTree,
}

#[pymethods]
impl QuadTreeWrapper {
    #[new]
    pub fn new(bounding_box: PyRectangle) -> Self {
        let bounding_rect = Rectangle {
            x: bounding_box.x,
            y: bounding_box.y,
            width: bounding_box.width,
            height: bounding_box.height,
        };
        QuadTreeWrapper {
            quadtree: QuadTree::new(bounding_rect),
        }
    }

    #[staticmethod]
    pub fn new_with_config(bounding_box: PyRectangle, config: PyConfig) -> Self {
        let bounding_rect = Rectangle {
            x: bounding_box.x,
            y: bounding_box.y,
            width: bounding_box.width,
            height: bounding_box.height,
        };
        let rust_config = Config {
            pool_size: config.pool_size,
            node_capacity: config.node_capacity,
            max_depth: config.max_depth,
        };
        QuadTreeWrapper {
            quadtree: QuadTree::new_with_config(bounding_rect, rust_config),
        }
    }

    pub fn insert(
        &mut self,
        py: Python,
        value: u32,
        shape: PyObject,
        entity_type: Option<u32>,
    ) -> PyResult<()> {
        let shape = extract_shape(py, shape)?;
        self.quadtree.insert(value, shape, entity_type);
        Ok(())
    }

    pub fn delete(&mut self, value: u32) {
        self.quadtree.delete(value);
    }

    pub fn collisions(&self, py: Python, shape: PyObject) -> PyResult<Vec<u32>> {
        return self.collisions_filter(py, shape, None);
    }

    pub fn collisions_filter(
        &self,
        py: Python,
        shape: PyObject,
        entity_types: Option<&PyList>,
    ) -> PyResult<Vec<u32>> {
        let shape = extract_shape(py, shape)?;

        let entity_types = extract_entity_types(entity_types)?;

        let mut collisions = Vec::new();
        self.quadtree
            .collisions_filter(shape, entity_types, &mut collisions);
        Ok(collisions)
    }

    pub fn collisions_batch(&self, py: Python, shapes: &PyList) -> PyResult<Vec<Vec<u32>>> {
        self.collisions_batch_filter(py, shapes, None)
    }

    pub fn collisions_batch_filter(
        &self,
        py: Python,
        shapes: &PyList,
        entity_types: Option<&PyList>,
    ) -> PyResult<Vec<Vec<u32>>> {
        let shapes: Vec<ShapeEnum> = shapes
            .iter()
            .map(|shape| extract_shape(py, shape.into()))
            .collect::<Result<_, _>>()?;

        let entity_types = extract_entity_types(entity_types)?;

        Ok(self.quadtree.collisions_batch_filter(shapes, entity_types))
    }

    pub fn relocate(
        &mut self,
        py: Python,
        value: u32,
        shape: PyObject,
        entity_type: Option<u32>,
    ) -> PyResult<()> {
        let shape = extract_shape(py, shape)?;
        self.quadtree.relocate(value, shape, entity_type);
        Ok(())
    }

    pub fn relocate_batch(
        &mut self,
        py: Python,
        relocation_requests: Vec<&PyTuple>,
    ) -> PyResult<()> {
        // Convert the Python tuples into Rust RelocationRequest objects
        let requests: Vec<RelocationRequest> = relocation_requests
            .into_iter()
            .map(|tuple| {
                let value = tuple.get_item(0).unwrap().extract::<u32>().unwrap();
                let shape = extract_shape(py, tuple.get_item(1).unwrap().into()).unwrap();
                let entity_type: Option<u32> = match tuple.get_item(2).unwrap() {
                    obj if obj.is_none() => None, // Check if it's a Python None
                    obj => Some(obj.extract::<u32>().unwrap()),
                };
                RelocationRequest {
                    value,
                    shape,
                    entity_type,
                }
            })
            .collect();

        self.quadtree.relocate_batch(requests);

        Ok(())
    }

    pub fn all_node_bounding_boxes(&self) -> Vec<(f32, f32, f32, f32)> {
        let mut bounding_boxes = Vec::new();
        self.quadtree.all_node_bounding_boxes(&mut bounding_boxes);
        bounding_boxes
            .into_iter()
            .map(|rect| (rect.x, rect.y, rect.width, rect.height))
            .collect()
    }

    pub fn all_shapes(&self, py: Python) -> PyResult<Vec<PyObject>> {
        let mut shapes = Vec::new();
        self.quadtree.all_shapes(&mut shapes);
        let mut py_shapes = Vec::new();
        for shape in shapes {
            let py_shape = if let Some(circle) = shape.as_any().downcast_ref::<Circle>() {
                Py::new(
                    py,
                    PyCircle {
                        x: circle.x,
                        y: circle.y,
                        radius: circle.radius,
                    },
                )?
                .into_py(py)
            } else if let Some(rect) = shape.as_any().downcast_ref::<Rectangle>() {
                Py::new(
                    py,
                    PyRectangle {
                        x: rect.x,
                        y: rect.y,
                        width: rect.width,
                        height: rect.height,
                    },
                )?
                .into_py(py)
            } else {
                return Err(PyTypeError::new_err("Unknown shape"));
            };
            py_shapes.push(py_shape);
        }
        Ok(py_shapes)
    }
}
