use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    Int32,
    UInt32,
    Float,
    Bool,
    String,
    Bytes,
    Enum,
    Message,
}

#[derive(Debug, Clone)]
pub struct FieldSchema {
    pub name: String,
    pub number: u16,
    pub kind: FieldKind,
    pub is_repeated: bool,
    pub type_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MessageSchema {
    pub name: String,
    pub fields: Vec<FieldSchema>,
    pub fields_by_name: HashMap<String, FieldSchema>,
    pub fields_by_number: HashMap<u16, FieldSchema>,
}

#[derive(Debug, Clone)]
pub struct NetSchema {
    pub messages: HashMap<String, MessageSchema>,
}

#[derive(Debug, Deserialize)]
struct RawSchema {
    messages: HashMap<String, RawMessageSchema>,
}

#[derive(Debug, Deserialize)]
struct RawMessageSchema {
    name: String,
    fields: Vec<RawFieldSchema>,
}

#[derive(Debug, Deserialize)]
struct RawFieldSchema {
    name: String,
    number: u16,
    #[serde(rename = "type")]
    field_type: String,
    label: String,
    #[serde(default)]
    type_name: Option<String>,
}

fn parse_kind(field_type: &str) -> FieldKind {
    match field_type {
        "int32" | "sint32" | "sfixed32" => FieldKind::Int32,
        "uint32" | "fixed32" => FieldKind::UInt32,
        "float" => FieldKind::Float,
        "bool" => FieldKind::Bool,
        "string" => FieldKind::String,
        "bytes" => FieldKind::Bytes,
        "enum" => FieldKind::Enum,
        "message" => FieldKind::Message,
        other => panic!("Unsupported field type: {other}"),
    }
}

fn build_message(raw: RawMessageSchema) -> MessageSchema {
    let mut fields = Vec::with_capacity(raw.fields.len());
    let mut fields_by_name = HashMap::new();
    let mut fields_by_number = HashMap::new();
    for field in raw.fields {
        let schema = FieldSchema {
            name: field.name.clone(),
            number: field.number,
            kind: parse_kind(&field.field_type),
            is_repeated: field.label == "repeated",
            type_name: field.type_name,
        };
        fields_by_name.insert(field.name, schema.clone());
        fields_by_number.insert(schema.number, schema.clone());
        fields.push(schema);
    }
    MessageSchema {
        name: raw.name,
        fields,
        fields_by_name,
        fields_by_number,
    }
}

fn load_schema() -> NetSchema {
    let raw: RawSchema =
        serde_json::from_str(include_str!("../schema/net_schema.json"))
            .expect("Invalid net_schema.json");
    let mut messages = HashMap::new();
    for (name, message) in raw.messages {
        messages.insert(name, build_message(message));
    }
    NetSchema { messages }
}

pub static NET_SCHEMA: Lazy<NetSchema> = Lazy::new(load_schema);
