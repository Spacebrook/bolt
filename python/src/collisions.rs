use crate::extract_shape_ncollide;
use ::collisions as libcollisions;
use ::collisions::ShapeWithPosition;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyListMethods};
use pyo3::Bound;

#[pyfunction]
pub fn get_mtv(
    py: Python,
    entity: Py<PyAny>,
    colliding_polys: &Bound<'_, PyList>,
) -> PyResult<Option<(f32, f32)>> {
    let entity_shape = extract_shape_ncollide(py, entity)?;

    let colliding_polys_rust: Vec<ShapeWithPosition> = colliding_polys
        .iter()
        .map(|item| extract_shape_ncollide(py, item.unbind()))
        .collect::<PyResult<_>>()?;

    let result = libcollisions::get_mtv(&entity_shape, &colliding_polys_rust);
    Ok(result)
}
