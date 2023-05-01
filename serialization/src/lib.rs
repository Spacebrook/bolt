use std::collections::HashMap;

#[derive(Debug, PartialEq, Clone)]
pub enum FieldValue {
    Int(i32),
    Float(f32),
    Bool(bool),
    String(String),
}

pub struct DiffFieldSet {
    pub defaults: HashMap<String, FieldValue>,
    pub fields: HashMap<String, FieldValue>,
    pub fields_without_defaults: HashMap<String, FieldValue>,
    pub changed_fields: HashMap<String, FieldValue>,
    pub old_fields: HashMap<String, FieldValue>,
}

impl DiffFieldSet {
    pub fn new(defaults: Option<HashMap<String, FieldValue>>) -> Self {
        Self {
            defaults: defaults.unwrap_or_default(),
            fields: HashMap::new(),
            fields_without_defaults: HashMap::new(),
            changed_fields: HashMap::new(),
            old_fields: HashMap::new(),
        }
    }

    pub fn start_update(&mut self) {
        self.changed_fields.clear();
    }

    pub fn update_one(&mut self, key: String, value: FieldValue) {
        self.fields.insert(key.clone(), value.clone());

        if !self.old_fields.contains_key(&key) || self.old_fields[&key] != value {
            self.changed_fields.insert(key.clone(), value.clone());
            self.old_fields.insert(key.clone(), value.clone());
        }

        if let Some(default_value) = self.defaults.get(&key) {
            if default_value != &value {
                self.fields_without_defaults
                    .insert(key.clone(), value.clone());
            } else {
                self.fields_without_defaults.remove(&key);
            }
        } else {
            self.fields_without_defaults.insert(key, value);
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
