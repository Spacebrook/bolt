use ::quadtree::quadtree::{Config, EntityTypeUpdate, QuadTree, RelocationRequest};
use ::quadtree::QuadtreeError;
use common::shapes::{Circle, Rectangle, Shape, ShapeEnum};

use crate::{extract_entity_types, extract_shape, PyCircle, PyRectangle};
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::pyclass;
use pyo3::pymethods;
use pyo3::types::{PyAny, PyAnyMethods, PyList, PyListMethods, PyTuple, PyTupleMethods};
use pyo3::{Bound, Py, PyErr, PyResult, Python};

#[derive(Clone)]
#[pyclass(name = "Config")]
pub struct PyConfig {
    pool_size: usize,
    node_capacity: usize,
    max_depth: usize,
    min_size: f32,
    looseness: f32,
    large_entity_threshold_factor: f32,
}

#[pymethods]
impl PyConfig {
    #[new]
    #[pyo3(signature = (pool_size, node_capacity, max_depth, min_size=None, looseness=None, large_entity_threshold_factor=None))]
    pub fn new(
        pool_size: usize,
        node_capacity: usize,
        max_depth: usize,
        min_size: Option<f32>,
        looseness: Option<f32>,
        large_entity_threshold_factor: Option<f32>,
    ) -> Self {
        let min_size = min_size.unwrap_or(1.0);
        let looseness = looseness.unwrap_or(1.0).max(1.0);
        let large_entity_threshold_factor = large_entity_threshold_factor.unwrap_or(0.0);
        PyConfig {
            pool_size,
            node_capacity,
            max_depth,
            min_size,
            looseness,
            large_entity_threshold_factor,
        }
    }
}

#[pyclass(name = "QuadTree", unsendable)]
pub struct QuadTreeWrapper {
    quadtree: QuadTree,
}

fn map_quadtree_error(err: QuadtreeError) -> PyErr {
    PyValueError::new_err(err.to_string())
}

#[pymethods]
impl QuadTreeWrapper {
    #[new]
    pub fn new(bounding_box: PyRectangle) -> PyResult<Self> {
        let bounding_rect = Rectangle {
            x: bounding_box.x() + bounding_box.width() / 2.0,
            y: bounding_box.y() + bounding_box.height() / 2.0,
            width: bounding_box.width(),
            height: bounding_box.height(),
        };
        Ok(QuadTreeWrapper {
            quadtree: QuadTree::new(bounding_rect).map_err(map_quadtree_error)?,
        })
    }

    #[staticmethod]
    pub fn new_with_config(bounding_box: PyRectangle, config: PyConfig) -> PyResult<Self> {
        let bounding_rect = Rectangle {
            x: bounding_box.x() + bounding_box.width() / 2.0,
            y: bounding_box.y() + bounding_box.height() / 2.0,
            width: bounding_box.width(),
            height: bounding_box.height(),
        };
        let rust_config = Config {
            pool_size: config.pool_size,
            node_capacity: config.node_capacity,
            max_depth: config.max_depth,
            min_size: config.min_size,
            looseness: config.looseness,
            large_entity_threshold_factor: config.large_entity_threshold_factor,
            profile_summary: false,
            profile_detail: false,
            profile_limit: 5,
        };
        Ok(QuadTreeWrapper {
            quadtree: QuadTree::new_with_config(bounding_rect, rust_config)
                .map_err(map_quadtree_error)?,
        })
    }

    #[pyo3(signature = (value, shape, entity_type=None))]
    pub fn insert(
        &mut self,
        py: Python,
        value: u32,
        shape: Py<PyAny>,
        entity_type: Option<u32>,
    ) -> PyResult<()> {
        let shape = extract_shape(py, shape)?;
        self.quadtree
            .insert(value, shape, entity_type)
            .map_err(map_quadtree_error)?;
        Ok(())
    }

    pub fn delete(&mut self, value: u32) {
        self.quadtree.delete(value);
    }

    pub fn collisions(&mut self, py: Python, shape: Py<PyAny>) -> PyResult<Vec<u32>> {
        return self.collisions_filter(py, shape, None);
    }

    #[pyo3(signature = (shape, entity_types=None))]
    pub fn collisions_filter(
        &mut self,
        py: Python,
        shape: Py<PyAny>,
        entity_types: Option<&Bound<'_, PyList>>,
    ) -> PyResult<Vec<u32>> {
        let shape = extract_shape(py, shape)?;

        let entity_types = extract_entity_types(entity_types)?;

        let mut collisions = Vec::new();
        self.quadtree
            .collisions_filter(shape, entity_types, &mut collisions)
            .map_err(map_quadtree_error)?;
        Ok(collisions)
    }

    pub fn collisions_batch(
        &mut self,
        py: Python,
        shapes: &Bound<'_, PyList>,
    ) -> PyResult<Vec<Vec<u32>>> {
        self.collisions_batch_filter(py, shapes, None)
    }

    #[pyo3(signature = (shapes, entity_types=None))]
    pub fn collisions_batch_filter(
        &mut self,
        py: Python,
        shapes: &Bound<'_, PyList>,
        entity_types: Option<&Bound<'_, PyList>>,
    ) -> PyResult<Vec<Vec<u32>>> {
        let shapes: Vec<ShapeEnum> = shapes
            .iter()
            .map(|shape| extract_shape(py, shape.unbind()))
            .collect::<Result<_, _>>()?;

        let entity_types = extract_entity_types(entity_types)?;

        self.quadtree
            .collisions_batch_filter(shapes, entity_types)
            .map_err(map_quadtree_error)
    }

    #[pyo3(signature = (value, shape, entity_type=None))]
    pub fn relocate(
        &mut self,
        py: Python,
        value: u32,
        shape: Py<PyAny>,
        entity_type: Option<Py<PyAny>>,
    ) -> PyResult<()> {
        let shape = extract_shape(py, shape)?;
        let update = match entity_type {
            None => EntityTypeUpdate::Preserve,
            Some(obj) => {
                let obj = obj.bind(py);
                if obj.is_none() {
                    EntityTypeUpdate::Preserve
                } else if let Ok(value) = obj.extract::<u32>() {
                    EntityTypeUpdate::Set(value)
                } else if let Ok(text) = obj.extract::<String>() {
                    match text.as_str() {
                        "clear" => EntityTypeUpdate::Clear,
                        "preserve" => EntityTypeUpdate::Preserve,
                        _ => {
                            return Err(PyTypeError::new_err(
                                "entity_type must be an int, 'clear', 'preserve', or None",
                            ));
                        }
                    }
                } else {
                    return Err(PyTypeError::new_err(
                        "entity_type must be an int, 'clear', 'preserve', or None",
                    ));
                }
            }
        };
        self.quadtree
            .relocate(value, shape, update)
            .map_err(map_quadtree_error)?;
        Ok(())
    }

    pub fn relocate_batch(
        &mut self,
        py: Python,
        relocation_requests: Vec<Bound<'_, PyTuple>>,
    ) -> PyResult<()> {
        // Convert the Python tuples into Rust RelocationRequest objects
        let requests: Vec<RelocationRequest> = relocation_requests
            .into_iter()
            .map(|tuple| {
                let value = tuple.get_item(0)?.extract::<u32>()?;
                let shape = extract_shape(py, tuple.get_item(1)?.unbind())?;
                let entity_type = match tuple.get_item(2)? {
                    obj if obj.is_none() => EntityTypeUpdate::Preserve,
                    obj => {
                        if let Ok(value) = obj.extract::<u32>() {
                            EntityTypeUpdate::Set(value)
                        } else if let Ok(text) = obj.extract::<String>() {
                            match text.as_str() {
                                "clear" => EntityTypeUpdate::Clear,
                                "preserve" => EntityTypeUpdate::Preserve,
                                _ => {
                                    return Err(PyTypeError::new_err(
                                        "entity_type must be an int, 'clear', 'preserve', or None",
                                    ));
                                }
                            }
                        } else {
                            return Err(PyTypeError::new_err(
                                "entity_type must be an int, 'clear', 'preserve', or None",
                            ));
                        }
                    }
                };
                Ok(RelocationRequest {
                    value,
                    shape,
                    entity_type,
                })
            })
            .collect::<PyResult<_>>()?;

        self.quadtree
            .relocate_batch(requests)
            .map_err(map_quadtree_error)?;

        Ok(())
    }

    pub fn all_node_bounding_boxes(&mut self) -> Vec<(f32, f32, f32, f32)> {
        let mut bounding_boxes = Vec::new();
        self.quadtree.all_node_bounding_boxes(&mut bounding_boxes);
        bounding_boxes
            .into_iter()
            .map(|rect| {
                (
                    rect.x - rect.width / 2.0,
                    rect.y - rect.height / 2.0,
                    rect.width,
                    rect.height,
                )
            })
            .collect()
    }

    pub fn all_shapes(&self, py: Python) -> PyResult<Vec<Py<PyAny>>> {
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
                .into()
            } else if let Some(rect) = shape.as_any().downcast_ref::<Rectangle>() {
                Py::new(
                    py,
                    PyRectangle::new(
                        rect.x - rect.width / 2.0,
                        rect.y - rect.width / 2.0,
                        rect.width,
                        rect.height,
                    ),
                )?
                .into()
            } else {
                return Err(PyTypeError::new_err("Unknown shape"));
            };
            py_shapes.push(py_shape);
        }
        Ok(py_shapes)
    }
}

#[cfg(test)]
mod tests {
    use super::{PyRectangle, QuadTreeWrapper};

    #[test]
    fn quadtree_wrapper_preserves_top_left_bounds() {
        let qt = QuadTreeWrapper::new(PyRectangle::new(10.0, 20.0, 100.0, 200.0));
        let bounding_boxes = qt.all_node_bounding_boxes();
        assert_eq!(bounding_boxes.len(), 1);
        let (x, y, width, height) = bounding_boxes[0];
        assert!((x - 10.0).abs() < 1e-6);
        assert!((y - 20.0).abs() < 1e-6);
        assert!((width - 100.0).abs() < 1e-6);
        assert!((height - 200.0).abs() < 1e-6);
    }
}
