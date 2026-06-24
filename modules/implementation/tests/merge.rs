use genvm_modules::llm::merge::{merge_extra, merge_values, MergeStrategy};
use serde_json::json;
use std::collections::BTreeMap;

#[test]
fn none_overwrites_scalars() {
    let mut target = json!({"a": 1, "b": 2});
    merge_values(&mut target, json!({"a": 10, "c": 3}), MergeStrategy::None).unwrap();
    assert_eq!(target, json!({"a": 10, "b": 2, "c": 3}));
}

#[test]
fn none_recurses_into_objects() {
    let mut target = json!({"nested": {"a": 1, "b": 2}});
    merge_values(
        &mut target,
        json!({"nested": {"b": 20, "c": 3}}),
        MergeStrategy::None,
    )
    .unwrap();
    assert_eq!(target, json!({"nested": {"a": 1, "b": 20, "c": 3}}));
}

#[test]
fn none_errors_on_arrays() {
    let mut target = json!({"a": [1, 2]});
    let err = merge_values(&mut target, json!({"a": [3, 4]}), MergeStrategy::None);
    assert!(err.is_err());
}

#[test]
fn replace_replaces_entirely() {
    let mut target = json!({"a": 1, "b": 2});
    merge_values(&mut target, json!({"c": 3}), MergeStrategy::Replace).unwrap();
    assert_eq!(target, json!({"c": 3}));
}

#[test]
fn merge_left_arrays() {
    let mut target = json!([1, 2]);
    merge_values(&mut target, json!([3, 4]), MergeStrategy::MergeLeft).unwrap();
    assert_eq!(target, json!([1, 2, 3, 4]));
}

#[test]
fn merge_right_arrays() {
    let mut target = json!([1, 2]);
    merge_values(&mut target, json!([3, 4]), MergeStrategy::MergeRight).unwrap();
    assert_eq!(target, json!([3, 4, 1, 2]));
}

#[test]
fn merge_left_strings() {
    let mut target = json!("hello");
    merge_values(&mut target, json!(" world"), MergeStrategy::MergeLeft).unwrap();
    assert_eq!(target, json!("hello world"));
}

#[test]
fn merge_right_strings() {
    let mut target = json!("world");
    merge_values(&mut target, json!("hello "), MergeStrategy::MergeRight).unwrap();
    assert_eq!(target, json!("hello world"));
}

#[test]
fn merge_left_errors_on_objects() {
    let mut target = json!({"a": 1});
    let err = merge_values(&mut target, json!({"b": 2}), MergeStrategy::MergeLeft);
    assert!(err.is_err());
}

#[test]
fn map_per_key_strategies() {
    let mut target = json!({
        "items": [1, 2],
        "name": "old",
        "keep": "original"
    });
    let strategy = MergeStrategy::Map(BTreeMap::from([
        ("items".into(), MergeStrategy::MergeLeft),
        ("name".into(), MergeStrategy::Replace),
    ]));
    merge_values(
        &mut target,
        json!({"items": [3, 4], "name": "new", "extra": true}),
        strategy,
    )
    .unwrap();
    assert_eq!(
        target,
        json!({"items": [1, 2, 3, 4], "name": "new", "keep": "original", "extra": true})
    );
}

#[test]
fn map_default_errors_on_arrays() {
    let mut target = json!({"a": [1]});
    let strategy = MergeStrategy::Map(BTreeMap::new());
    let err = merge_values(&mut target, json!({"a": [2]}), strategy);
    assert!(err.is_err());
}

#[test]
fn map_inserts_missing_keys() {
    let mut target = json!({"a": 1});
    let strategy = MergeStrategy::Map(BTreeMap::new());
    merge_values(&mut target, json!({"b": 2}), strategy).unwrap();
    assert_eq!(target, json!({"a": 1, "b": 2}));
}

#[test]
fn merge_extra_applies_strategy() {
    let mut request = json!({"model": "gpt-4", "temperature": 0.7});
    merge_extra(
        &mut request,
        json!({"temperature": 0.9, "top_p": 0.5}),
        MergeStrategy::None,
    )
    .unwrap();
    assert_eq!(
        request,
        json!({"model": "gpt-4", "temperature": 0.9, "top_p": 0.5})
    );
}

#[test]
fn deserialize_string_variants() {
    assert!(matches!(
        serde_json::from_value::<MergeStrategy>(json!("none")).unwrap(),
        MergeStrategy::None
    ));
    assert!(matches!(
        serde_json::from_value::<MergeStrategy>(json!("replace")).unwrap(),
        MergeStrategy::Replace
    ));
    assert!(matches!(
        serde_json::from_value::<MergeStrategy>(json!("merge_left")).unwrap(),
        MergeStrategy::MergeLeft
    ));
    assert!(matches!(
        serde_json::from_value::<MergeStrategy>(json!("merge_right")).unwrap(),
        MergeStrategy::MergeRight
    ));
}

#[test]
fn deserialize_null_is_none() {
    assert!(matches!(
        serde_json::from_value::<MergeStrategy>(json!(null)).unwrap(),
        MergeStrategy::None
    ));
}

#[test]
fn deserialize_map() {
    let strategy: MergeStrategy =
        serde_json::from_value(json!({"items": "merge_left", "name": "replace"})).unwrap();
    let MergeStrategy::Map(map) = strategy else {
        panic!("expected Map variant");
    };
    assert!(matches!(map.get("items"), Some(MergeStrategy::MergeLeft)));
    assert!(matches!(map.get("name"), Some(MergeStrategy::Replace)));
}

#[test]
fn deserialize_invalid_string_errors() {
    assert!(serde_json::from_value::<MergeStrategy>(json!("invalid")).is_err());
}
