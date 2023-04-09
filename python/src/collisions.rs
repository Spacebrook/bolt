use crate::extract_shape_ncollide;
use ::collisions as libcollisions;
use ::collisions::ShapeWithPosition;
use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3::wrap_pyfunction;

#[pyfunction]
fn get_mtv(py: Python, entity: PyObject, colliding_polys: &PyList) -> PyResult<Option<(f32, f32)>> {
    let entity_shape = extract_shape_ncollide(py, entity)?;

    let colliding_polys_rust: Vec<ShapeWithPosition> = colliding_polys
        .iter()
        .map(|item| Ok(extract_shape_ncollide(py, item.into())?))
        .collect::<PyResult<_>>()?;

    let result = libcollisions::get_mtv(&entity_shape, colliding_polys_rust);
    Ok(result)
}

#[pymodule]
pub fn collisions(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_wrapped(wrap_pyfunction!(get_mtv))?;
    Ok(())
}
