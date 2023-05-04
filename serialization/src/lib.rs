use smallvec::{SmallVec, smallvec};

#[derive(Debug, PartialEq, Clone)]
pub enum FieldValue {
    Int(i32),
    Float(f32),
    Bool(bool),
    String(String),
    None,
}

pub enum FieldType {
    Int = 0,
    Float = 1,
    Bool = 2,
    String = 3,
}

impl FieldType {
    pub fn from_int(value: i32) -> Result<Self, String> {
        match value {
            0 => Ok(FieldType::Int),
            1 => Ok(FieldType::Float),
            2 => Ok(FieldType::Bool),
            3 => Ok(FieldType::String),
            _ => Err(format!("Invalid field type: {}", value)),
        }
    }
}

pub struct DiffFieldSet {
    pub field_types: SmallVec<[FieldType; 16]>,
    pub field_defaults: SmallVec<[FieldValue; 16]>,
    pub fields: SmallVec<[FieldValue; 16]>,
    pub changed_fields: SmallVec<[usize; 16]>,
    pub fields_without_defaults: SmallVec<[usize; 16]>,
}

impl DiffFieldSet {
    pub fn new(field_types: SmallVec<[FieldType; 16]>, field_defaults: SmallVec<[FieldValue; 16]>) -> Self {
        let len = field_types.len();
        Self {
            field_types,
            field_defaults,
            fields: smallvec![FieldValue::None; len],
            fields_without_defaults: SmallVec::with_capacity(len),
            changed_fields: SmallVec::with_capacity(len),
        }
    }

    pub fn update(&mut self, updates: SmallVec<[(usize, FieldValue); 16]>) {
        self.changed_fields.clear();
        self.fields_without_defaults.clear();
        for (index, value) in updates.into_iter() {
            if self.fields[index] != value {
                self.fields[index] = value.clone();
                self.changed_fields.push(index);
            }
            if self.field_defaults[index] != value {
                self.fields_without_defaults.push(index);
            }
        }
    }

    pub fn has_changed(&self) -> bool {
        !self.changed_fields.is_empty()
    }

    pub fn get_diff(&self) -> SmallVec<[(usize, FieldValue); 16]> {
        self.changed_fields
            .iter()
            .map(|&index| (index, self.fields[index].clone()))
            .collect()
    }

    pub fn get_all(&self) -> SmallVec<[(usize, FieldValue); 16]> {
        self.fields_without_defaults
            .iter()
            .map(|&index| (index, self.fields[index].clone()))
            .collect()
    }
}
