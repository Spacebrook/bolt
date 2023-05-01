use serialization::*;

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::pyclass;
use pyo3::pymethods;
use pyo3::types::{PyDict, PyList, PyTuple};
use smallvec::SmallVec;
use std::borrow::Cow;
use std::collections::HashMap;

#[pyclass(name = "DiffFieldSet", unsendable)]
pub struct DiffFieldSetWrapper {
    diff_field_set: DiffFieldSet,
}

#[pymethods]
impl DiffFieldSetWrapper {
    #[new]
    pub fn new(py: Python, defaults: Option<HashMap<String, PyObject>>) -> PyResult<Self> {
        let mut rust_defaults = HashMap::new();
        if let Some(defaults) = defaults {
            for (key, value) in defaults {
                let rust_value = get_rust_value(py, value)?;
                rust_defaults.insert(key, rust_value);
            }
        }
        Ok(Self {
            diff_field_set: DiffFieldSet::new(Some(rust_defaults)),
        })
    }

    pub fn update(&mut self, py: Python, updates: &PyList) -> PyResult<()> {
        let mut rust_updates = SmallVec::<[(Cow<str>, FieldValue); 16]>::new();
        for item in updates {
            if let Ok(py_tuple) = item.extract::<&PyTuple>() {
                if py_tuple.len() == 2 {
                    let key = py_tuple.get_item(0)?.extract::<String>()?;
                    let value = get_rust_value(py, py_tuple.get_item(1)?.to_object(py))?;
                    rust_updates.push((Cow::Owned(key), value));
                } else {
                    return Err(PyTypeError::new_err("Each tuple must contain exactly 2 items"));
                }
            } else {
                return Err(PyTypeError::new_err("List must contain tuples of (key, value) pairs"));
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
        convert_to_py_dict(py, diff)
    }

    pub fn get_all(&self, py: Python) -> PyResult<PyObject> {
        let all_fields = self.diff_field_set.get_all();
        convert_to_py_dict(py, all_fields)
    }
}

fn convert_to_py_dict(
    py: Python,
    field_values: &HashMap<String, FieldValue>,
) -> PyResult<PyObject> {
    let py_dict = PyDict::new(py);
    for (key, value) in field_values {
        let py_value = match value {
            FieldValue::Int(val) => val.into_py(py),
            FieldValue::Float(val) => val.into_py(py),
            FieldValue::Bool(val) => val.into_py(py),
            FieldValue::String(val) => val.into_py(py),
            FieldValue::None => py.None(),
        };
        py_dict.set_item(key, py_value)?;
    }
    Ok(py_dict.to_object(py))
}

fn get_rust_value(py: Python, value: PyObject) -> PyResult<FieldValue> {
    if value.is_none(py) {
        return Ok(FieldValue::None);
    }

    value
        .extract::<i32>(py)
        .map(FieldValue::Int)
        .or_else(|_| value.extract::<f32>(py).map(FieldValue::Float))
        .or_else(|_| value.extract::<bool>(py).map(FieldValue::Bool))
        .or_else(|_| value.extract::<String>(py).map(FieldValue::String))
        .map_err(|_| {
            let type_name = value
                .getattr(py, "__class__")
                .unwrap()
                .getattr(py, "__name__")
                .unwrap()
                .extract::<String>(py)
                .unwrap_or_else(|_| "<unknown>".to_string());
            PyTypeError::new_err(format!("Unsupported field value type: {}", type_name))
        })
}
