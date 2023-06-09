use serialization::*;

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::pyclass;
use pyo3::pymethods;
use pyo3::types::PyList;
use smallvec::SmallVec;

#[pyclass(name = "DiffFieldSet", unsendable)]
pub struct DiffFieldSetWrapper {
    diff_field_set: DiffFieldSet,
}

#[pymethods]
impl DiffFieldSetWrapper {
    #[new]
    pub fn new(py: Python, field_types: Vec<i32>, field_defaults: Vec<PyObject>) -> PyResult<Self> {
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
            .enumerate()
            .map(|(index, (field_type, value))| get_rust_value(py, field_type, value, index))
            .collect::<PyResult<SmallVec<[FieldValue; 16]>>>()?;

        Ok(Self {
            diff_field_set: DiffFieldSet::new(rust_field_types, rust_field_defaults),
        })
    }

    pub fn update(&mut self, py: Python, updates: &PyList) -> PyResult<()> {
        let mut rust_updates = SmallVec::<[FieldValue; 16]>::new();
        for (index, item) in updates.iter().enumerate() {
            let field_type = &self.diff_field_set.field_types[index];
            let value = get_rust_value(py, field_type, item.to_object(py), index)?;
            rust_updates.push(value);
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
    let py_list = PyList::empty(py);
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

fn get_rust_value(
    py: Python,
    field_type: &FieldType,
    value: PyObject,
    index: usize,
) -> PyResult<FieldValue> {
    if value.is_none(py) {
        return Ok(FieldValue::None);
    }

    match field_type {
        FieldType::Int => value.extract::<i32>(py).map(FieldValue::Int).map_err(|_| {
            PyTypeError::new_err(format!("Expected an integer value at index {index}"))
        }),
        FieldType::Float => value
            .extract::<f32>(py)
            .map(FieldValue::Float)
            .map_err(|_| PyTypeError::new_err(format!("Expected a float value at index {index}"))),
        FieldType::Bool => value
            .extract::<bool>(py)
            .map(FieldValue::Bool)
            .map_err(|_| {
                PyTypeError::new_err(format!("Expected a boolean value at index {index}"))
            }),
        FieldType::String => value
            .extract::<String>(py)
            .map(FieldValue::String)
            .map_err(|_| PyTypeError::new_err(format!("Expected a string value at index {index}"))),
    }
}
