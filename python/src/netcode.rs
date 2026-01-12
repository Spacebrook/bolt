use netcode::{FieldKind, MessageSchema, NET_SCHEMA};
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyBytesMethods, PyDict, PyDictMethods, PyList, PyListMethods};
use pyo3::{Bound, IntoPyObjectExt};

const FRAME_VERSION: u8 = 1;

#[pyclass(name = "NetCodec")]
pub struct NetCodec;

#[pymethods]
impl NetCodec {
    #[new]
    pub fn new() -> Self {
        NetCodec
    }

    pub fn encode_frame(&self, py: Python, payload: &Bound<'_, PyDict>) -> PyResult<Py<PyBytes>> {
        let mut buffer = Vec::with_capacity(2048);

        let sequence = get_u32(payload, "sequence")?.unwrap_or(0);

        let complete = get_bool(payload, "complete")?;
        let complete_global = get_bool(payload, "complete_global")?;
        let reset = get_bool(payload, "reset")?;

        let self_id = get_u32(payload, "self_id")?;
        let tick_rate = get_f32(payload, "tick_rate")?;
        let pong = get_u32(payload, "pong")?;

        let area = get_message_bytes(py, payload, "area", "Area")?;
        let map = get_message_bytes(py, payload, "map", "Map")?;
        let chat = get_message_bytes(py, payload, "chat", "Chat")?;
        let settings = get_message_bytes(py, payload, "settings", "Settings")?;
        let mod_tools_response =
            get_message_bytes(py, payload, "mod_tools_response", "ModToolsResponse")?;
        let quest_data = get_message_bytes(py, payload, "quest_data", "QuestData")?;

        let x_entities = get_bytes(payload, "x_entities")?;
        let y_entities = get_bytes(payload, "y_entities")?;
        let xy_entities = get_bytes(payload, "xy_entities")?;
        let xy_radius_entities = get_bytes(payload, "xy_radius_entities")?;

        let mut flags1 = 0u8;
        if complete {
            flags1 |= 1 << 0;
        }
        if complete_global {
            flags1 |= 1 << 1;
        }
        if reset {
            flags1 |= 1 << 2;
        }
        if self_id.is_some() {
            flags1 |= 1 << 3;
        }
        if tick_rate.is_some() {
            flags1 |= 1 << 4;
        }
        if pong.is_some() {
            flags1 |= 1 << 5;
        }
        if area.is_some() {
            flags1 |= 1 << 6;
        }
        if map.is_some() {
            flags1 |= 1 << 7;
        }

        let mut flags2 = 0u8;
        if chat.is_some() {
            flags2 |= 1 << 0;
        }
        if settings.is_some() {
            flags2 |= 1 << 1;
        }
        if mod_tools_response.is_some() {
            flags2 |= 1 << 2;
        }
        if quest_data.is_some() {
            flags2 |= 1 << 3;
        }
        if x_entities.is_some() {
            flags2 |= 1 << 4;
        }
        if y_entities.is_some() {
            flags2 |= 1 << 5;
        }
        if xy_entities.is_some() {
            flags2 |= 1 << 6;
        }
        if xy_radius_entities.is_some() {
            flags2 |= 1 << 7;
        }

        buffer.push(FRAME_VERSION);
        buffer.push(flags1);
        buffer.push(flags2);
        write_u32(&mut buffer, sequence);

        if let Some(value) = self_id {
            write_u32(&mut buffer, value);
        }
        if let Some(value) = tick_rate {
            write_f32(&mut buffer, value);
        }
        if let Some(value) = pong {
            write_u32(&mut buffer, value);
        }

        if let Some(value) = area {
            write_bytes(&mut buffer, &value);
        }
        if let Some(value) = map {
            write_bytes(&mut buffer, &value);
        }
        if let Some(value) = chat {
            write_bytes(&mut buffer, &value);
        }
        if let Some(value) = settings {
            write_bytes(&mut buffer, &value);
        }
        if let Some(value) = mod_tools_response {
            write_bytes(&mut buffer, &value);
        }
        if let Some(value) = quest_data {
            write_bytes(&mut buffer, &value);
        }
        if let Some(value) = x_entities {
            write_bytes(&mut buffer, &value);
        }
        if let Some(value) = y_entities {
            write_bytes(&mut buffer, &value);
        }
        if let Some(value) = xy_entities {
            write_bytes(&mut buffer, &value);
        }
        if let Some(value) = xy_radius_entities {
            write_bytes(&mut buffer, &value);
        }

        let entities = get_list(payload, "entities")?;
        let global_entities = get_list(payload, "global_entities")?;
        let entity_schema = get_schema("Entity")?;
        encode_entity_list(py, entity_schema, entities, &mut buffer)?;
        encode_entity_list(py, entity_schema, global_entities, &mut buffer)?;

        Ok(PyBytes::new(py, &buffer).unbind())
    }

    pub fn encode_message(
        &self,
        py: Python,
        name: &str,
        payload: &Bound<'_, PyDict>,
    ) -> PyResult<Py<PyBytes>> {
        let schema = get_schema(name)?;
        let mut buffer = Vec::with_capacity(256);
        encode_message(py, schema, payload, &mut buffer)?;
        Ok(PyBytes::new(py, &buffer).unbind())
    }

    pub fn decode_message(
        &self,
        py: Python,
        name: &str,
        bytes: &Bound<'_, PyBytes>,
    ) -> PyResult<Py<PyDict>> {
        let schema = get_schema(name)?;
        let mut cursor = Cursor::new(bytes.as_bytes());
        let dict = decode_message(py, schema, &mut cursor)?;
        Ok(dict.into())
    }
}

fn encode_entity_list<'py>(
    py: Python<'py>,
    schema: &'py MessageSchema,
    list: Option<Bound<'py, PyList>>,
    buffer: &mut Vec<u8>,
) -> PyResult<()> {
    let entities = match list {
        Some(list) => list,
        None => {
            write_u32(buffer, 0);
            return Ok(());
        }
    };

    write_u32(buffer, entities.len() as u32);
    for item in entities.iter() {
        let dict = item.cast::<PyDict>()?;
        encode_message(py, schema, dict, buffer)?;
    }
    Ok(())
}

fn encode_message<'py>(
    py: Python<'py>,
    schema: &'py MessageSchema,
    dict: &Bound<'py, PyDict>,
    buffer: &mut Vec<u8>,
) -> PyResult<()> {
    let mut entries: Vec<(u16, &netcode::FieldSchema, Bound<'py, PyAny>)> = Vec::new();

    for (key, value) in dict.iter() {
        if value.is_none() {
            continue;
        }
        let key_str: String = key.extract()?;
        if key_str == "hero" {
            let hero = value.cast::<PyDict>()?;
            append_hero_fields(schema, hero, &mut entries)?;
            continue;
        }
        if let Some(field) = schema.fields_by_name.get(&key_str) {
            entries.push((field.number, field, value));
        }
    }

    entries.sort_by_key(|(number, _, _)| *number);
    write_u16(buffer, entries.len() as u16);

    for (number, field, value) in entries {
        write_u16(buffer, number);
        encode_field_value(py, field, &value, buffer)?;
    }
    Ok(())
}

fn append_hero_fields<'py>(
    schema: &'py MessageSchema,
    hero: &Bound<'py, PyDict>,
    entries: &mut Vec<(u16, &'py netcode::FieldSchema, Bound<'py, PyAny>)>,
) -> PyResult<()> {
    for (key, value) in hero.iter() {
        if value.is_none() {
            continue;
        }
        let key_str: String = key.extract()?;
        if let Some(field) = schema.fields_by_name.get(&key_str) {
            entries.push((field.number, field, value));
        }
    }
    Ok(())
}

fn encode_field_value<'py>(
    py: Python<'py>,
    field: &netcode::FieldSchema,
    value: &Bound<'py, PyAny>,
    buffer: &mut Vec<u8>,
) -> PyResult<()> {
    if field.is_repeated {
        let list = value.cast::<PyList>()?;
        write_u16(buffer, list.len() as u16);
        for item in list.iter() {
            encode_single_value(py, field, &item, buffer)?;
        }
        return Ok(());
    }

    encode_single_value(py, field, value, buffer)
}

fn encode_single_value<'py>(
    py: Python<'py>,
    field: &netcode::FieldSchema,
    value: &Bound<'py, PyAny>,
    buffer: &mut Vec<u8>,
) -> PyResult<()> {
    match field.kind {
        FieldKind::Int32 | FieldKind::Enum => {
            let value = extract_i32(value, field.name.as_str())?;
            write_i32(buffer, value);
        }
        FieldKind::UInt32 => {
            let value = extract_u32(value, field.name.as_str())?;
            write_u32(buffer, value);
        }
        FieldKind::Float => {
            let value = extract_f32(value, field.name.as_str())?;
            write_f32(buffer, value);
        }
        FieldKind::Bool => {
            let value = extract_bool(value, field.name.as_str())?;
            buffer.push(if value { 1 } else { 0 });
        }
        FieldKind::String => {
            let value: String = value.extract()?;
            write_bytes(buffer, value.as_bytes());
        }
        FieldKind::Bytes => {
            let bytes = value.cast::<PyBytes>()?;
            write_bytes(buffer, bytes.as_bytes());
        }
        FieldKind::Message => {
            let dict = value.cast::<PyDict>()?;
            let message_schema = match field.type_name.as_deref() {
                Some(name) => get_schema(name)?,
                other => {
                    return Err(PyTypeError::new_err(format!(
                        "Unsupported message type for {}: {other:?}",
                        field.name
                    )))
                }
            };
            encode_message(py, message_schema, dict, buffer)?;
        }
    }
    Ok(())
}

fn extract_i32(value: &Bound<'_, PyAny>, name: &str) -> PyResult<i32> {
    if let Ok(v) = value.extract::<i32>() {
        return Ok(v);
    }
    if let Ok(v) = value.extract::<f32>() {
        return Ok(v as i32);
    }
    Err(PyTypeError::new_err(format!("Expected int32 for {name}")))
}

fn extract_u32(value: &Bound<'_, PyAny>, name: &str) -> PyResult<u32> {
    if let Ok(v) = value.extract::<u32>() {
        return Ok(v);
    }
    if let Ok(v) = value.extract::<i32>() {
        if v < 0 {
            return Err(PyTypeError::new_err(format!(
                "Expected non-negative uint32 for {name}"
            )));
        }
        return Ok(v as u32);
    }
    Err(PyTypeError::new_err(format!("Expected uint32 for {name}")))
}

fn extract_f32(value: &Bound<'_, PyAny>, name: &str) -> PyResult<f32> {
    if let Ok(v) = value.extract::<f32>() {
        return Ok(v);
    }
    if let Ok(v) = value.extract::<i32>() {
        return Ok(v as f32);
    }
    Err(PyTypeError::new_err(format!("Expected float for {name}")))
}

fn extract_bool(value: &Bound<'_, PyAny>, name: &str) -> PyResult<bool> {
    if let Ok(v) = value.extract::<bool>() {
        return Ok(v);
    }
    let type_name = value
        .get_type()
        .name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "unknown".to_string());
    let repr = value
        .repr()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "<unrepr>".to_string());
    Err(PyTypeError::new_err(format!(
        "Expected bool for {name}, got {type_name} value {repr}"
    )))
}

fn write_u16(buffer: &mut Vec<u8>, value: u16) {
    buffer.extend_from_slice(&value.to_le_bytes());
}

fn write_u32(buffer: &mut Vec<u8>, value: u32) {
    buffer.extend_from_slice(&value.to_le_bytes());
}

fn write_i32(buffer: &mut Vec<u8>, value: i32) {
    buffer.extend_from_slice(&value.to_le_bytes());
}

fn write_f32(buffer: &mut Vec<u8>, value: f32) {
    buffer.extend_from_slice(&value.to_le_bytes());
}

fn write_bytes(buffer: &mut Vec<u8>, bytes: &[u8]) {
    let len = bytes.len() as u32;
    write_u32(buffer, len);
    buffer.extend_from_slice(bytes);
}

fn get_u32(payload: &Bound<'_, PyDict>, key: &str) -> PyResult<Option<u32>> {
    match payload.get_item(key)? {
        Some(value) if !value.is_none() => value.extract::<u32>().map(Some),
        _ => Ok(None),
    }
}

fn get_f32(payload: &Bound<'_, PyDict>, key: &str) -> PyResult<Option<f32>> {
    match payload.get_item(key)? {
        Some(value) if !value.is_none() => value.extract::<f32>().map(Some),
        _ => Ok(None),
    }
}

fn get_bool(payload: &Bound<'_, PyDict>, key: &str) -> PyResult<bool> {
    match payload.get_item(key)? {
        Some(value) if !value.is_none() => value.extract::<bool>(),
        _ => Ok(false),
    }
}

fn get_bytes(payload: &Bound<'_, PyDict>, key: &str) -> PyResult<Option<Vec<u8>>> {
    match payload.get_item(key)? {
        Some(value) if !value.is_none() => {
            let bytes = value.cast::<PyBytes>()?;
            Ok(Some(bytes.as_bytes().to_vec()))
        }
        _ => Ok(None),
    }
}

fn get_message_bytes(
    py: Python,
    payload: &Bound<'_, PyDict>,
    key: &str,
    message_name: &str,
) -> PyResult<Option<Vec<u8>>> {
    match payload.get_item(key)? {
        Some(value) if !value.is_none() => {
            if let Ok(bytes) = value.cast::<PyBytes>() {
                return Ok(Some(bytes.as_bytes().to_vec()));
            }
            let dict = value.cast::<PyDict>()?;
            let schema = get_schema(message_name)?;
            let mut buffer = Vec::with_capacity(128);
            encode_message(py, schema, dict, &mut buffer)?;
            Ok(Some(buffer))
        }
        _ => Ok(None),
    }
}

fn get_list<'py>(payload: &Bound<'py, PyDict>, key: &str) -> PyResult<Option<Bound<'py, PyList>>> {
    match payload.get_item(key)? {
        Some(value) if !value.is_none() => Ok(Some(value.cast::<PyList>()?.clone())),
        _ => Ok(None),
    }
}

fn get_schema(name: &str) -> PyResult<&'static MessageSchema> {
    NET_SCHEMA
        .messages
        .get(name)
        .ok_or_else(|| PyTypeError::new_err(format!("Unknown message schema: {name}")))
}

struct Cursor<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }

    fn read_u8(&mut self) -> PyResult<u8> {
        if self.offset >= self.data.len() {
            return Err(PyTypeError::new_err("Unexpected end of buffer"));
        }
        let value = self.data[self.offset];
        self.offset += 1;
        Ok(value)
    }

    fn read_u16(&mut self) -> PyResult<u16> {
        if self.offset + 2 > self.data.len() {
            return Err(PyTypeError::new_err("Unexpected end of buffer"));
        }
        let value = u16::from_le_bytes(self.data[self.offset..self.offset + 2].try_into().unwrap());
        self.offset += 2;
        Ok(value)
    }

    fn read_u32(&mut self) -> PyResult<u32> {
        if self.offset + 4 > self.data.len() {
            return Err(PyTypeError::new_err("Unexpected end of buffer"));
        }
        let value = u32::from_le_bytes(self.data[self.offset..self.offset + 4].try_into().unwrap());
        self.offset += 4;
        Ok(value)
    }

    fn read_i32(&mut self) -> PyResult<i32> {
        if self.offset + 4 > self.data.len() {
            return Err(PyTypeError::new_err("Unexpected end of buffer"));
        }
        let value = i32::from_le_bytes(self.data[self.offset..self.offset + 4].try_into().unwrap());
        self.offset += 4;
        Ok(value)
    }

    fn read_f32(&mut self) -> PyResult<f32> {
        if self.offset + 4 > self.data.len() {
            return Err(PyTypeError::new_err("Unexpected end of buffer"));
        }
        let value = f32::from_le_bytes(self.data[self.offset..self.offset + 4].try_into().unwrap());
        self.offset += 4;
        Ok(value)
    }

    fn read_bytes(&mut self) -> PyResult<&'a [u8]> {
        let len = self.read_u32()? as usize;
        if self.offset + len > self.data.len() {
            return Err(PyTypeError::new_err("Unexpected end of buffer"));
        }
        let start = self.offset;
        let end = start + len;
        self.offset = end;
        Ok(&self.data[start..end])
    }
}

fn decode_message(py: Python, schema: &MessageSchema, cursor: &mut Cursor) -> PyResult<Py<PyDict>> {
    let field_count = cursor.read_u16()? as usize;
    let dict = PyDict::new(py);
    for _ in 0..field_count {
        let number = cursor.read_u16()?;
        let field = schema
            .fields_by_number
            .get(&number)
            .ok_or_else(|| PyTypeError::new_err(format!("Unknown field number: {number}")))?;
        if field.is_repeated {
            let count = cursor.read_u16()? as usize;
            let list = PyList::empty(py);
            for _ in 0..count {
                let value = decode_single_value(py, field, cursor)?;
                list.append(value)?;
            }
            dict.set_item(field.name.as_str(), list)?;
        } else {
            let value = decode_single_value(py, field, cursor)?;
            dict.set_item(field.name.as_str(), value)?;
        }
    }
    Ok(dict.into())
}

fn decode_single_value(
    py: Python,
    field: &netcode::FieldSchema,
    cursor: &mut Cursor,
) -> PyResult<Py<PyAny>> {
    match field.kind {
        FieldKind::Int32 | FieldKind::Enum => Ok(cursor.read_i32()?.into_py_any(py)?),
        FieldKind::UInt32 => Ok(cursor.read_u32()?.into_py_any(py)?),
        FieldKind::Float => Ok(cursor.read_f32()?.into_py_any(py)?),
        FieldKind::Bool => Ok((cursor.read_u8()? == 1).into_py_any(py)?),
        FieldKind::String => {
            let bytes = cursor.read_bytes()?;
            let value = String::from_utf8(bytes.to_vec())
                .map_err(|_| PyTypeError::new_err("Invalid UTF-8 string"))?;
            Ok(value.into_py_any(py)?)
        }
        FieldKind::Bytes => {
            let bytes = cursor.read_bytes()?;
            Ok(PyBytes::new(py, bytes).unbind().into())
        }
        FieldKind::Message => {
            let message_schema = match field.type_name.as_deref() {
                Some(name) => get_schema(name)?,
                None => {
                    return Err(PyTypeError::new_err(format!(
                        "Missing message type for {}",
                        field.name
                    )))
                }
            };
            let value = decode_message(py, message_schema, cursor)?;
            Ok(value.into())
        }
    }
}
