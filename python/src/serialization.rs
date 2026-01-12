use netcode::{FieldKind, NET_SCHEMA};
use serialization::*;

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::pyclass;
use pyo3::pymethods;
use pyo3::types::{PyAny, PyBool, PyDict, PyFloat, PyInt, PyList, PyString, PyStringMethods};
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
            .map(|(index, (field_type, value))| {
                get_rust_value(field_type, value.bind(py), index, None)
            })
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

        let schema = NET_SCHEMA.messages.get(message_name).ok_or_else(|| {
            PyTypeError::new_err(format!("Unknown message schema: {message_name}"))
        })?;

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
                get_rust_value(field_type, value.bind(py), index, name)
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
                defaults = list.iter().map(|value| value.unbind()).collect();
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
                get_rust_value(field_type, value.bind(py), index, name)
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

    pub fn update(&mut self, updates: &Bound<'_, PyList>) -> PyResult<()> {
        let mut rust_updates = SmallVec::<[FieldValue; 16]>::new();
        for (index, item) in updates.iter().enumerate() {
            let field_type = &self.diff_field_set.field_types[index];
            let field_name = self.field_names.get(index).map(String::as_str);
            let value = get_rust_value(field_type, &item, index, field_name)?;
            rust_updates.push(value);
        }
        self.diff_field_set.update(rust_updates);
        Ok(())
    }

    pub fn update_from_getters(
        &mut self,
        obj: &Bound<'_, PyAny>,
        getters: &Bound<'_, PyList>,
    ) -> PyResult<()> {
        update_from_getters_internal(self, obj, getters)?;
        Ok(())
    }

    pub fn update_from_getters_with_children(
        &mut self,
        obj: &Bound<'_, PyAny>,
        getters: &Bound<'_, PyList>,
        children: &Bound<'_, PyDict>,
    ) -> PyResult<()> {
        update_from_getters_internal(self, obj, getters)?;
        update_children_internal(obj, children)?;
        Ok(())
    }

    #[staticmethod]
    pub fn build_child_payload(
        obj: &Bound<'_, PyAny>,
        include_all: bool,
        children: &Bound<'_, PyDict>,
        extras: &Bound<'_, PyList>,
    ) -> PyResult<Py<PyAny>> {
        build_payload_internal(None, obj, include_all, children, extras)
    }

    #[staticmethod]
    pub fn build_child_payload_if_changed(
        obj: &Bound<'_, PyAny>,
        children: &Bound<'_, PyDict>,
        extras: &Bound<'_, PyList>,
    ) -> PyResult<Option<Py<PyAny>>> {
        build_child_payload_if_changed_internal(obj, children, extras)
    }

    #[staticmethod]
    pub fn update_children(obj: &Bound<'_, PyAny>, children: &Bound<'_, PyDict>) -> PyResult<()> {
        update_children_internal(obj, children)
    }

    #[staticmethod]
    pub fn child_has_changed(
        obj: &Bound<'_, PyAny>,
        children: &Bound<'_, PyDict>,
    ) -> PyResult<bool> {
        child_has_changed_internal(obj, children)
    }

    pub fn build_payload(
        &self,
        obj: &Bound<'_, PyAny>,
        include_all: bool,
        children: &Bound<'_, PyDict>,
        extras: &Bound<'_, PyList>,
    ) -> PyResult<Py<PyAny>> {
        build_payload_internal(
            Some((self.field_names.as_slice(), &self.diff_field_set)),
            obj,
            include_all,
            children,
            extras,
        )
    }

    pub fn build_payload_if_changed(
        &self,
        obj: &Bound<'_, PyAny>,
        children: &Bound<'_, PyDict>,
        extras: &Bound<'_, PyList>,
    ) -> PyResult<Option<Py<PyAny>>> {
        if self.field_names.is_empty() {
            return Err(PyTypeError::new_err(
                "Field names not configured for DiffFieldSet",
            ));
        }
        build_payload_if_changed_internal(
            &self.field_names,
            &self.diff_field_set,
            obj,
            children,
            extras,
        )
    }

    pub fn has_changed_with_children(
        &self,
        obj: &Bound<'_, PyAny>,
        children: &Bound<'_, PyDict>,
    ) -> PyResult<bool> {
        if self.diff_field_set.has_changed() {
            return Ok(true);
        }
        child_has_changed_internal(obj, children)
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
    fill_py_dict(py, &dict, names, field_values)?;
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
    field_type: &FieldType,
    value: &Bound<'_, PyAny>,
    index: usize,
    field_name: Option<&str>,
) -> PyResult<FieldValue> {
    if value.is_none() {
        return Ok(FieldValue::None);
    }
    let label = || {
        field_name
            .map(|name| format!("{name} (index {index})"))
            .unwrap_or_else(|| format!("index {index}"))
    };

    match field_type {
        FieldType::Int => {
            if let Ok(int_value) = value.cast_exact::<PyInt>() {
                return Ok(FieldValue::Int(int_value.extract::<i32>()?));
            }
            match value.extract::<i32>() {
                Ok(val) => Ok(FieldValue::Int(val)),
                Err(_) => {
                    let value_type = value.get_type().name()?.to_string_lossy().into_owned();
                    let value_repr = value.repr()?.to_string_lossy().into_owned();
                    Err(PyTypeError::new_err(format!(
                        "Expected an integer value for {}, got {value_type} value {value_repr}",
                        label()
                    )))
                }
            }
        }
        FieldType::Float => {
            if let Ok(float_value) = value.cast_exact::<PyFloat>() {
                return Ok(FieldValue::Float(float_value.value() as f32));
            }
            match value.extract::<f32>() {
                Ok(val) => Ok(FieldValue::Float(val)),
                Err(_) => {
                    let value_type = value.get_type().name()?.to_string_lossy().into_owned();
                    let value_repr = value.repr()?.to_string_lossy().into_owned();
                    Err(PyTypeError::new_err(format!(
                        "Expected a float value for {}, got {value_type} value {value_repr}",
                        label()
                    )))
                }
            }
        }
        FieldType::Bool => {
            if let Ok(bool_value) = value.cast_exact::<PyBool>() {
                return Ok(FieldValue::Bool(bool_value.is_true()));
            }
            match value.extract::<bool>() {
                Ok(val) => Ok(FieldValue::Bool(val)),
                Err(_) => {
                    let value_type = value.get_type().name()?.to_string_lossy().into_owned();
                    let value_repr = value.repr()?.to_string_lossy().into_owned();
                    Err(PyTypeError::new_err(format!(
                        "Expected a boolean value for {}, got {value_type} value {value_repr}",
                        label()
                    )))
                }
            }
        }
        FieldType::String => {
            if let Ok(str_value) = value.cast_exact::<PyString>() {
                let text = str_value.to_str()?.to_owned();
                return Ok(FieldValue::String(text));
            }
            match value.extract::<String>() {
                Ok(val) => Ok(FieldValue::String(val)),
                Err(_) => {
                    let value_type = value.get_type().name()?.to_string_lossy().into_owned();
                    let value_repr = value.repr()?.to_string_lossy().into_owned();
                    Err(PyTypeError::new_err(format!(
                        "Expected a string value for {}, got {value_type} value {value_repr}",
                        label()
                    )))
                }
            }
        }
    }
}

fn fill_py_dict(
    py: Python,
    dict: &Bound<'_, PyDict>,
    names: &[String],
    field_values: SmallVec<[(usize, FieldValue); 16]>,
) -> PyResult<()> {
    for (index, value) in field_values {
        set_py_dict_value(py, dict, names, index, &value)?;
    }
    Ok(())
}

fn fill_py_dict_from_indices(
    py: Python,
    dict: &Bound<'_, PyDict>,
    names: &[String],
    fields: &[FieldValue],
    indices: &[usize],
) -> PyResult<()> {
    for &index in indices {
        let value = fields
            .get(index)
            .ok_or_else(|| PyTypeError::new_err(format!("Field index out of range: {index}")))?;
        set_py_dict_value(py, dict, names, index, value)?;
    }
    Ok(())
}

fn set_py_dict_value(
    py: Python,
    dict: &Bound<'_, PyDict>,
    names: &[String],
    index: usize,
    value: &FieldValue,
) -> PyResult<()> {
    let name = names
        .get(index)
        .ok_or_else(|| PyTypeError::new_err(format!("Field index out of range: {index}")))?;
    let py_value = match value {
        FieldValue::Int(val) => val.into_py_any(py)?,
        FieldValue::Float(val) => val.into_py_any(py)?,
        FieldValue::Bool(val) => val.into_py_any(py)?,
        FieldValue::String(val) => val.into_py_any(py)?,
        FieldValue::None => py.None(),
    };
    dict.set_item(name.as_str(), py_value)?;
    Ok(())
}

fn update_from_getters_internal(
    wrapper: &mut DiffFieldSetWrapper,
    obj: &Bound<'_, PyAny>,
    getters: &Bound<'_, PyList>,
) -> PyResult<()> {
    wrapper.diff_field_set.changed_fields.clear();
    wrapper.diff_field_set.fields_without_defaults.clear();
    for (index, getter) in getters.iter().enumerate() {
        let field_type = &wrapper.diff_field_set.field_types[index];
        let field_name = wrapper.field_names.get(index).map(String::as_str);
        let value = match getter.cast::<PyString>() {
            Ok(name) => obj.getattr(name)?,
            Err(_) => getter.call1((obj,))?,
        };
        let value = get_rust_value(field_type, &value, index, field_name)?;
        if wrapper.diff_field_set.fields[index] != value {
            wrapper.diff_field_set.fields[index] = value.clone();
            wrapper.diff_field_set.changed_fields.push(index);
        }
        if wrapper.diff_field_set.field_defaults[index] != value {
            wrapper.diff_field_set.fields_without_defaults.push(index);
        }
    }
    Ok(())
}

fn update_children_internal(obj: &Bound<'_, PyAny>, children: &Bound<'_, PyDict>) -> PyResult<()> {
    if children.is_empty() {
        return Ok(());
    }
    for (_, value) in children.iter() {
        let attr_name = value.cast::<PyString>()?;
        let child = obj.getattr(attr_name)?;
        if child.is_none() {
            continue;
        }
        child.call_method0("update_field_set")?;
    }
    Ok(())
}

fn child_has_changed_internal(
    obj: &Bound<'_, PyAny>,
    children: &Bound<'_, PyDict>,
) -> PyResult<bool> {
    if children.is_empty() {
        return Ok(false);
    }
    for (_, value) in children.iter() {
        let attr_name = value.cast::<PyString>()?;
        let child = obj.getattr(attr_name)?;
        if child.is_none() {
            continue;
        }
        if child.call_method0("has_changed")?.is_truthy()? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn build_child_payload_if_changed_internal(
    obj: &Bound<'_, PyAny>,
    children: &Bound<'_, PyDict>,
    extras: &Bound<'_, PyList>,
) -> PyResult<Option<Py<PyAny>>> {
    let py = obj.py();
    let mut child_payloads: Vec<(Bound<'_, PyAny>, Bound<'_, PyAny>)> = Vec::new();
    let mut changed = false;
    if !children.is_empty() {
        for (key, value) in children.iter() {
            let attr_name = value.cast::<PyString>()?;
            let child = obj.getattr(attr_name)?;
            if child.is_none() {
                continue;
            }
            if child.call_method0("has_changed")?.is_truthy()? {
                let payload = child.call_method0("get_diff")?;
                child_payloads.push((key, payload));
                changed = true;
            }
        }
    }
    if !changed {
        return Ok(None);
    }
    let dict = PyDict::new(py);
    for (key, payload) in child_payloads {
        dict.set_item(key, payload)?;
    }
    for extra in extras.iter() {
        let tuple = extra.cast::<pyo3::types::PyTuple>()?;
        let key = tuple.get_item(0)?;
        let getter = tuple.get_item(1)?;
        let value = match getter.cast::<PyString>() {
            Ok(name) => obj.getattr(name)?,
            Err(_) => getter.call1((obj,))?,
        };
        dict.set_item(key, value)?;
    }
    Ok(Some(dict.unbind().into_any()))
}

fn build_payload_if_changed_internal(
    names: &[String],
    field_set: &DiffFieldSet,
    obj: &Bound<'_, PyAny>,
    children: &Bound<'_, PyDict>,
    extras: &Bound<'_, PyList>,
) -> PyResult<Option<Py<PyAny>>> {
    let py = obj.py();
    let mut child_payloads: Vec<(Bound<'_, PyAny>, Bound<'_, PyAny>)> = Vec::new();
    let mut changed = field_set.has_changed();
    if !children.is_empty() {
        for (key, value) in children.iter() {
            let attr_name = value.cast::<PyString>()?;
            let child = obj.getattr(attr_name)?;
            if child.is_none() {
                continue;
            }
            if child.call_method0("has_changed")?.is_truthy()? {
                let payload = child.call_method0("get_diff")?;
                child_payloads.push((key, payload));
                changed = true;
            }
        }
    }
    if !changed {
        return Ok(None);
    }
    let dict = PyDict::new(py);
    fill_py_dict_from_indices(
        py,
        &dict,
        names,
        &field_set.fields,
        &field_set.changed_fields,
    )?;
    for (key, payload) in child_payloads {
        dict.set_item(key, payload)?;
    }
    for extra in extras.iter() {
        let tuple = extra.cast::<pyo3::types::PyTuple>()?;
        let key = tuple.get_item(0)?;
        let getter = tuple.get_item(1)?;
        let value = match getter.cast::<PyString>() {
            Ok(name) => obj.getattr(name)?,
            Err(_) => getter.call1((obj,))?,
        };
        dict.set_item(key, value)?;
    }
    Ok(Some(dict.unbind().into_any()))
}
fn build_payload_internal(
    fields: Option<(&[String], &DiffFieldSet)>,
    obj: &Bound<'_, PyAny>,
    include_all: bool,
    children: &Bound<'_, PyDict>,
    extras: &Bound<'_, PyList>,
) -> PyResult<Py<PyAny>> {
    let py = obj.py();
    let dict = PyDict::new(py);
    if let Some((names, field_set)) = fields {
        if names.is_empty() {
            return Err(PyTypeError::new_err(
                "Field names not configured for DiffFieldSet",
            ));
        }
        if include_all {
            fill_py_dict_from_indices(
                py,
                &dict,
                names,
                &field_set.fields,
                &field_set.fields_without_defaults,
            )?;
        } else {
            fill_py_dict_from_indices(
                py,
                &dict,
                names,
                &field_set.fields,
                &field_set.changed_fields,
            )?;
        }
    }

    for (key, value) in children.iter() {
        let attr_name = value.cast::<PyString>()?;
        let child = obj.getattr(attr_name)?;
        if child.is_none() {
            continue;
        }
        if include_all || child.call_method0("has_changed")?.is_truthy()? {
            let child_payload = if include_all {
                child.call_method0("get_all")?
            } else {
                child.call_method0("get_diff")?
            };
            dict.set_item(key, child_payload)?;
        }
    }

    for extra in extras.iter() {
        let tuple = extra.cast::<pyo3::types::PyTuple>()?;
        let key = tuple.get_item(0)?;
        let getter = tuple.get_item(1)?;
        let value = match getter.cast::<PyString>() {
            Ok(name) => obj.getattr(name)?,
            Err(_) => getter.call1((obj,))?,
        };
        dict.set_item(key, value)?;
    }

    Ok(dict.unbind().into_any())
}
