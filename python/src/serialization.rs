use netcode::{FieldKind, NET_SCHEMA};
use serialization::*;

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::pyclass;
use pyo3::pymethods;
use pyo3::types::{PyAny, PyDict, PyList};
use pyo3::IntoPyObjectExt;
use smallvec::SmallVec;
use std::collections::HashMap;

#[pyclass(name = "DiffFieldSet", unsendable)]
pub struct DiffFieldSetWrapper {
    diff_field_set: DiffFieldSet,
    field_names: Vec<String>,
}

#[pymethods]
impl DiffFieldSetWrapper {
    #[new]
    pub fn new(
        py: Python,
        field_types: Vec<i32>,
        field_defaults: Vec<Py<PyAny>>,
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
            .enumerate()
            .map(|(index, (field_type, value))| get_rust_value(py, field_type, value, index, None))
            .collect::<PyResult<SmallVec<[FieldValue; 16]>>>()?;

        Ok(Self {
            diff_field_set: DiffFieldSet::new(rust_field_types, rust_field_defaults),
            field_names: Vec::new(),
        })
    }

    #[staticmethod]
    pub fn from_schema(
        py: Python,
        message_name: &str,
        field_names: Vec<String>,
        field_defaults: Vec<Py<PyAny>>,
    ) -> PyResult<Self> {
        if field_names.len() != field_defaults.len() {
            return Err(PyTypeError::new_err(format!(
                "Field defaults length mismatch for {message_name}: expected {}, got {}",
                field_names.len(),
                field_defaults.len()
            )));
        }

        let schema = NET_SCHEMA
            .messages
            .get(message_name)
            .ok_or_else(|| PyTypeError::new_err(format!("Unknown message schema: {message_name}")))?;

        let rust_field_types = field_names
            .iter()
            .map(|name| {
                let field = schema.fields_by_name.get(name).ok_or_else(|| {
                    PyTypeError::new_err(format!(
                        "Unknown field name '{name}' for message {message_name}"
                    ))
                })?;
                field_kind_to_type(field.kind).ok_or_else(|| {
                    PyTypeError::new_err(format!(
                        "Unsupported field type for {message_name}.{name}"
                    ))
                })
            })
            .collect::<PyResult<SmallVec<[FieldType; 16]>>>()?;

        let rust_field_defaults = rust_field_types
            .iter()
            .zip(field_defaults)
            .enumerate()
            .map(|(index, (field_type, value))| {
                let name = field_names.get(index).map(String::as_str);
                get_rust_value(py, field_type, value, index, name)
            })
            .collect::<PyResult<SmallVec<[FieldValue; 16]>>>()?;

        Ok(Self {
            diff_field_set: DiffFieldSet::new(rust_field_types, rust_field_defaults),
            field_names,
        })
    }

    #[staticmethod]
    pub fn from_profile(
        py: Python,
        profile_name: &str,
        field_defaults: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let profile = NET_SCHEMA.profiles.get(profile_name).ok_or_else(|| {
            PyTypeError::new_err(format!("Unknown profile schema: {profile_name}"))
        })?;
        let schema = NET_SCHEMA.messages.get(&profile.message).ok_or_else(|| {
            PyTypeError::new_err(format!(
                "Unknown message schema '{}' for profile {}",
                profile.message, profile_name
            ))
        })?;

        let field_names = profile.fields.clone();
        let rust_field_types = field_names
            .iter()
            .map(|name| {
                let field = schema.fields_by_name.get(name).ok_or_else(|| {
                    PyTypeError::new_err(format!(
                        "Unknown field name '{name}' for message {}",
                        profile.message
                    ))
                })?;
                field_kind_to_type(field.kind).ok_or_else(|| {
                    PyTypeError::new_err(format!(
                        "Unsupported field type for {}.{name}",
                        profile.message
                    ))
                })
            })
            .collect::<PyResult<SmallVec<[FieldType; 16]>>>()?;

        let field_name_to_index: HashMap<String, usize> = field_names
            .iter()
            .enumerate()
            .map(|(index, name): (usize, &String)| (name.clone(), index))
            .collect();

        let mut defaults: Vec<Py<PyAny>> = (0..field_names.len()).map(|_| py.None()).collect();
        if let Some(values) = field_defaults {
            if let Ok(list) = values.cast::<PyList>() {
                if list.len() != field_names.len() {
                    return Err(PyTypeError::new_err(format!(
                        "Field defaults length mismatch for {profile_name}: expected {}, got {}",
                        field_names.len(),
                        list.len()
                    )));
                }
                defaults = list
                    .iter()
                    .map(|value| value.unbind())
                    .collect();
            } else if let Ok(dict) = values.cast::<PyDict>() {
                for (key, value) in dict.iter() {
                    let name = key.extract::<String>()?;
                    let index = field_name_to_index.get(&name).ok_or_else(|| {
                        PyTypeError::new_err(format!(
                            "Unknown default field '{name}' for profile {profile_name}"
                        ))
                    })?;
                    defaults[*index] = value.unbind();
                }
            } else {
                return Err(PyTypeError::new_err(
                    "field_defaults must be a list, dict, or None",
                ));
            }
        }

        let rust_field_defaults = rust_field_types
            .iter()
            .zip(defaults)
            .enumerate()
            .map(|(index, (field_type, value))| {
                let name = field_names.get(index).map(String::as_str);
                get_rust_value(py, field_type, value, index, name)
            })
            .collect::<PyResult<SmallVec<[FieldValue; 16]>>>()?;

        Ok(Self {
            diff_field_set: DiffFieldSet::new(rust_field_types, rust_field_defaults),
            field_names,
        })
    }

    #[staticmethod]
    pub fn has_profile(profile_name: &str) -> bool {
        NET_SCHEMA.profiles.contains_key(profile_name)
    }

    pub fn update(&mut self, py: Python, updates: &Bound<'_, PyList>) -> PyResult<()> {
        let mut rust_updates = SmallVec::<[FieldValue; 16]>::new();
        for (index, item) in updates.iter().enumerate() {
            let field_type = &self.diff_field_set.field_types[index];
            let field_name = self.field_names.get(index).map(String::as_str);
            let value = get_rust_value(py, field_type, item.unbind(), index, field_name)?;
            rust_updates.push(value);
        }
        self.diff_field_set.update(rust_updates);
        Ok(())
    }

    pub fn has_changed(&self) -> bool {
        self.diff_field_set.has_changed()
    }

    pub fn get_diff(&self, py: Python) -> PyResult<Py<PyAny>> {
        let diff = self.diff_field_set.get_diff();
        convert_to_py_list(py, diff)
    }

    pub fn get_all(&self, py: Python) -> PyResult<Py<PyAny>> {
        let all_fields = self.diff_field_set.get_all();
        convert_to_py_list(py, all_fields)
    }

    pub fn get_diff_named(&self, py: Python) -> PyResult<Py<PyAny>> {
        convert_to_py_dict(py, &self.field_names, self.diff_field_set.get_diff())
    }

    pub fn get_all_named(&self, py: Python) -> PyResult<Py<PyAny>> {
        convert_to_py_dict(py, &self.field_names, self.diff_field_set.get_all())
    }
}

fn convert_to_py_list(
    py: Python,
    field_values: SmallVec<[(usize, FieldValue); 16]>,
) -> PyResult<Py<PyAny>> {
    let py_list = PyList::empty(py);
    for (index, value) in field_values {
        let py_value = match value {
            FieldValue::Int(val) => val.into_py_any(py)?,
            FieldValue::Float(val) => val.into_py_any(py)?,
            FieldValue::Bool(val) => val.into_py_any(py)?,
            FieldValue::String(val) => val.into_py_any(py)?,
            FieldValue::None => py.None(),
        };
        py_list.append((index, py_value))?;
    }
    Ok(py_list.unbind().into_any())
}

fn convert_to_py_dict(
    py: Python,
    names: &[String],
    field_values: SmallVec<[(usize, FieldValue); 16]>,
) -> PyResult<Py<PyAny>> {
    if names.is_empty() {
        return Err(PyTypeError::new_err(
            "Field names not configured for DiffFieldSet",
        ));
    }
    let dict = PyDict::new(py);
    for (index, value) in field_values {
        let name = names.get(index).ok_or_else(|| {
            PyTypeError::new_err(format!("Field index out of range: {index}"))
        })?;
        let py_value = match value {
            FieldValue::Int(val) => val.into_py_any(py)?,
            FieldValue::Float(val) => val.into_py_any(py)?,
            FieldValue::Bool(val) => val.into_py_any(py)?,
            FieldValue::String(val) => val.into_py_any(py)?,
            FieldValue::None => py.None(),
        };
        dict.set_item(name.as_str(), py_value)?;
    }
    Ok(dict.unbind().into_any())
}

fn field_kind_to_type(kind: FieldKind) -> Option<FieldType> {
    match kind {
        FieldKind::Int32 | FieldKind::UInt32 | FieldKind::Enum => Some(FieldType::Int),
        FieldKind::Float => Some(FieldType::Float),
        FieldKind::Bool => Some(FieldType::Bool),
        FieldKind::String => Some(FieldType::String),
        FieldKind::Bytes | FieldKind::Message => None,
    }
}

fn get_rust_value(
    py: Python,
    field_type: &FieldType,
    value: Py<PyAny>,
    index: usize,
    field_name: Option<&str>,
) -> PyResult<FieldValue> {
    if value.is_none(py) {
        return Ok(FieldValue::None);
    }

    let label = field_name
        .map(|name| format!("{name} (index {index})"))
        .unwrap_or_else(|| format!("index {index}"));
    let value_ref = value.bind(py);
    let value_type = value_ref.get_type().name()?.to_string();
    let value_repr = value_ref.repr()?.extract::<String>()?;

    match field_type {
        FieldType::Int => value.extract::<i32>(py).map(FieldValue::Int).map_err(|_| {
            PyTypeError::new_err(format!(
                "Expected an integer value for {label}, got {value_type} value {value_repr}"
            ))
        }),
        FieldType::Float => value
            .extract::<f32>(py)
            .map(FieldValue::Float)
            .map_err(|_| {
                PyTypeError::new_err(format!(
                    "Expected a float value for {label}, got {value_type} value {value_repr}"
                ))
            }),
        FieldType::Bool => value
            .extract::<bool>(py)
            .map(FieldValue::Bool)
            .map_err(|_| {
                PyTypeError::new_err(format!(
                    "Expected a boolean value for {label}, got {value_type} value {value_repr}"
                ))
            }),
        FieldType::String => value
            .extract::<String>(py)
            .map(FieldValue::String)
            .map_err(|_| {
                PyTypeError::new_err(format!(
                    "Expected a string value for {label}, got {value_type} value {value_repr}"
                ))
            }),
    }
}
