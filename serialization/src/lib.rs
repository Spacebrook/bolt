use std::borrow::Cow;
use std::collections::HashMap;
use smallvec::SmallVec;

#[derive(Debug, PartialEq, Clone)]
pub enum FieldValue {
    Int(i32),
    Float(f32),
    Bool(bool),
    String(String),
    None,
}

pub struct DiffFieldSet {
    pub defaults: HashMap<String, FieldValue>,
    pub fields: HashMap<String, FieldValue>,
    pub fields_without_defaults: HashMap<String, FieldValue>,
    pub changed_fields: HashMap<String, FieldValue>,
    pub old_fields: HashMap<String, FieldValue>,
}

impl DiffFieldSet {
    const CAPACITY: usize = 32;

    pub fn new(defaults: Option<HashMap<String, FieldValue>>) -> Self {
        Self {
            defaults: defaults.unwrap_or_default(),
            fields: HashMap::with_capacity(Self::CAPACITY),
            fields_without_defaults: HashMap::with_capacity(Self::CAPACITY),
            changed_fields: HashMap::with_capacity(Self::CAPACITY),
            old_fields: HashMap::with_capacity(Self::CAPACITY),
        }
    }

    pub fn update(&mut self, updates: SmallVec<[(Cow<str>, FieldValue); 16]>) {
        self.changed_fields.retain(|key, value| {
            if let Some((_, update_value)) = updates.iter().find(|(k, _)| k.as_ref() == key) {
                update_value != value
            } else {
                false
            }
        });

        for (key, value) in updates {
            let key_ref = key.as_ref();
            let old_value = self.old_fields.get(key_ref);
            if old_value != Some(&value) {
                self.changed_fields.insert(key_ref.to_string(), value.clone());
                self.old_fields.insert(key_ref.to_string(), value.clone());
            }

            match self.defaults.get(key_ref) {
                Some(default_value) if default_value == &value => {
                    self.fields_without_defaults.remove(key_ref);
                }
                _ => {
                    self.fields_without_defaults.insert(key_ref.to_string(), value.clone());
                }
            }

            self.fields.insert(key.into_owned(), value);
        }
    }

    pub fn has_changed(&self) -> bool {
        !self.changed_fields.is_empty()
    }

    pub fn get_diff(&self) -> &HashMap<String, FieldValue> {
        &self.changed_fields
    }

    pub fn get_all(&self) -> &HashMap<String, FieldValue> {
        &self.fields_without_defaults
    }
}