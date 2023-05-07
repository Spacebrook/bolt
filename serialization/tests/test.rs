use serialization::*;
use smallvec::SmallVec;

#[test]
fn test_diff_field_set() {
    // Define field types
    let field_types = SmallVec::from(vec![FieldType::Int, FieldType::Int, FieldType::String]);

    // Define default field values
    let field_defaults = SmallVec::from(vec![
        FieldValue::Int(1),
        FieldValue::Int(2),
        FieldValue::None,
    ]);

    let mut diff_field_set = DiffFieldSet::new(field_types, field_defaults);

    // Update with a list of index-value pairs using the update method
    diff_field_set.update(SmallVec::from(vec![
        FieldValue::Int(1),
        FieldValue::Int(2),
        FieldValue::String(String::from("value")),
    ]));

    assert!(diff_field_set.has_changed());
    assert_eq!(
        diff_field_set.get_diff(),
        SmallVec::<[(usize, FieldValue); 16]>::from(vec![
            (0, FieldValue::Int(1)),
            (1, FieldValue::Int(2)),
            (2, FieldValue::String(String::from("value"))),
        ])
    );
    assert_eq!(
        diff_field_set.get_all(),
        SmallVec::<[(usize, FieldValue); 16]>::from(vec![(
            2,
            FieldValue::String(String::from("value"))
        ),])
    );

    // Update with a list of index-value pairs using the update method
    diff_field_set.update(SmallVec::from(vec![
        FieldValue::Int(1),
        FieldValue::Int(2),
        FieldValue::String(String::from("new value")),
    ]));

    assert!(diff_field_set.has_changed());
    assert_eq!(
        diff_field_set.get_diff(),
        SmallVec::<[(usize, FieldValue); 16]>::from(vec![(
            2,
            FieldValue::String(String::from("new value"))
        ),])
    );
    assert_eq!(
        diff_field_set.get_all(),
        SmallVec::<[(usize, FieldValue); 16]>::from(vec![(
            2,
            FieldValue::String(String::from("new value"))
        ),])
    );

    // These functions should be idempotent.
    assert!(diff_field_set.has_changed());
    assert_eq!(
        diff_field_set.get_diff(),
        SmallVec::<[(usize, FieldValue); 16]>::from(vec![(
            2,
            FieldValue::String(String::from("new value"))
        ),])
    );
    assert_eq!(
        diff_field_set.get_all(),
        SmallVec::<[(usize, FieldValue); 16]>::from(vec![(
            2,
            FieldValue::String(String::from("new value"))
        ),])
    );

    // Check that updating with no diff will change get_diff.
    // Update with a list of index-value pairs using the update method
    diff_field_set.update(SmallVec::from(vec![
        FieldValue::Int(1),
        FieldValue::Int(2),
        FieldValue::String(String::from("new value")),
    ]));

    assert!(!diff_field_set.has_changed());
    assert_eq!(
        diff_field_set.get_diff(),
        SmallVec::<[(usize, FieldValue); 16]>::new()
    );
    assert_eq!(
        diff_field_set.get_all(),
        SmallVec::<[(usize, FieldValue); 16]>::from(vec![(
            2,
            FieldValue::String(String::from("new value"))
        ),])
    );
}
