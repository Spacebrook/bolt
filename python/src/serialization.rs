use serialization::*;

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::pyclass;
use pyo3::pymethods;
use pyo3::types::{PyList, PyTuple};
use smallvec::SmallVec;

#[pyclass(name = "DiffFieldSet", unsendable)]
pub struct DiffFieldSetWrapper {
    diff_field_set: DiffFieldSet,
}

#[pymethods]
impl DiffFieldSetWrapper {
    #[new]
    pub fn new(
        py: Python,
        field_types: Vec<i32>,
        field_defaults: Vec<PyObject>,
    ) -> PyResult<Self> {
        // Convert Py field types to Rust field types
        let rust_field_types = field_types
            .into_iter()
            .map(FieldType::from_int)
            .collect::<Result<SmallVec<[FieldType; 16]>, String>>()
            .map_err(|err| PyTypeError::new_err(err))?;

        // Convert Py field defaults to Rust field values
        let rust_field_defaults = rust_field_types
            .iter()
            .zip(field_defaults)
            .map(|(field_type, value)| get_rust_value(py, field_type, value))
            .collect::<PyResult<SmallVec<[FieldValue; 16]>>>()?;

        Ok(Self {
            diff_field_set: DiffFieldSet::new(rust_field_types, rust_field_defaults),
        })
    }

    pub fn update(&mut self, py: Python, updates: &PyList) -> PyResult<()> {
        let mut rust_updates = SmallVec::<[(usize, FieldValue); 16]>::new();
        for item in updates {
            if let Ok(py_tuple) = item.extract::<&PyTuple>() {
                if py_tuple.len() == 2 {
                    let index = py_tuple.get_item(0)?.extract::<usize>()?;
                    let field_type = &self.diff_field_set.field_types[index];
                    let value = get_rust_value(py, field_type, py_tuple.get_item(1)?.to_object(py))?;
                    rust_updates.push((index, value));
                } else {
                    return Err(PyTypeError::new_err("Each tuple must contain exactly 2 items"));
                }
            } else {
                return Err(PyTypeError::new_err("List must contain tuples of (index, value) pairs"));
            }
        }
        self.diff_field_set.update(rust_updates);
        Ok(())
    }

    pub fn has_changed(&self) -> bool {
        self.diff_field_set.has_changed()
    }

    pub fn get_diff(&self, py: Python) -> PyResult<PyObject> {
        let diff = self.diff_field_set.get_diff();
        convert_to_py_list(py, diff)
    }

    pub fn get_all(&self, py: Python) -> PyResult<PyObject> {
        let all_fields = self.diff_field_set.get_all();
        convert_to_py_list(py, all_fields)
    }
}

fn convert_to_py_list(
    py: Python,
    field_values: SmallVec<[(usize, FieldValue); 16]>,
) -> PyResult<PyObject> {
    let py_list =  PyList::empty(py);
    for (index, value) in field_values {
        let py_value = match value {
            FieldValue::Int(val) => val.into_py(py),
            FieldValue::Float(val) => val.into_py(py),
            FieldValue::Bool(val) => val.into_py(py),
            FieldValue::String(val) => val.into_py(py),
            FieldValue::None => py.None(),
        };
        py_list.append((index, py_value))?;
    }
    Ok(py_list.to_object(py))
}

fn get_rust_value(py: Python, field_type: &FieldType, value: PyObject) -> PyResult<FieldValue> {
    if value.is_none(py) {
        return Ok(FieldValue::None);
    }

    match field_type {
        FieldType::Int => value
            .extract::<i32>(py)
            .map(FieldValue::Int)
            .map_err(|_| PyTypeError::new_err("Expected an integer value")),
        FieldType::Float => value
            .extract::<f32>(py)
            .map(FieldValue::Float)
            .map_err(|_| PyTypeError::new_err("Expected a float value")),
        FieldType::Bool => value
            .extract::<bool>(py)
            .map(FieldValue::Bool)
            .map_err(|_| PyTypeError::new_err("Expected a boolean value")),
        FieldType::String => value
            .extract::<String>(py)
            .map(FieldValue::String)
            .map_err(|_| PyTypeError::new_err("Expected a string value")),
    }
}
