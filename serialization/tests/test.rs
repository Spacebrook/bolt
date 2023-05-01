use maplit;
use serialization::*;
use std::collections::HashMap;

#[test]
fn test_diff_field_set() {
    let mut fields = HashMap::new();
    fields.insert(String::from("x"), FieldValue::Int(1));
    fields.insert(String::from("y"), FieldValue::Int(2));
    fields.insert(
        String::from("field"),
        FieldValue::String(String::from("value")),
    );

    let mut diff_field_set = DiffFieldSet::new(None);
    diff_field_set.start_update();
    diff_field_set.update_one(String::from("x"), fields["x"].clone());
    diff_field_set.update_one(String::from("y"), fields["y"].clone());
    diff_field_set.update_one(String::from("field"), fields["field"].clone());

    assert!(diff_field_set.has_changed());
    assert_eq!(diff_field_set.get_diff(), &fields);
    assert_eq!(diff_field_set.get_all(), &fields);

    fields.insert(
        String::from("field"),
        FieldValue::String(String::from("new value")),
    );
    diff_field_set.start_update();
    diff_field_set.update_one(String::from("x"), fields["x"].clone());
    diff_field_set.update_one(String::from("y"), fields["y"].clone());
    diff_field_set.update_one(String::from("field"), fields["field"].clone());

    assert!(diff_field_set.has_changed());
    assert_eq!(
        diff_field_set.get_diff(),
        &maplit::hashmap! {
            String::from("field") => FieldValue::String(String::from("new value"))
        }
    );
    assert_eq!(diff_field_set.get_all(), &fields);

    // These functions should be idempotent.
    assert!(diff_field_set.has_changed());
    assert_eq!(
        diff_field_set.get_diff(),
        &maplit::hashmap! {
            String::from("field") => FieldValue::String(String::from("new value"))
        }
    );
    assert_eq!(diff_field_set.get_all(), &fields);

    // Check that updating with no diff will change get_diff.
    diff_field_set.start_update();
    diff_field_set.update_one(String::from("x"), fields["x"].clone());
    diff_field_set.update_one(String::from("y"), fields["y"].clone());
    diff_field_set.update_one(String::from("field"), fields["field"].clone());

    assert!(!diff_field_set.has_changed());
    assert_eq!(diff_field_set.get_diff(), &HashMap::new());
    assert_eq!(diff_field_set.get_all(), &fields);
}
