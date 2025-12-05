use sort_package_json::sort_package_json;
use std::fs;

#[test]
fn test_sort_package_json() {
    let input = fs::read_to_string("tests/fixtures/package.json").expect("Failed to read fixture");
    let result = sort_package_json(&input).expect("Failed to parse package.json");
    insta::assert_snapshot!(result);
}

#[test]
fn test_idempotency() {
    let input = fs::read_to_string("tests/fixtures/package.json").expect("Failed to read fixture");
    let first_sort = sort_package_json(&input).expect("Failed to parse package.json on first sort");
    let second_sort =
        sort_package_json(&first_sort).expect("Failed to parse package.json on second sort");
    assert_eq!(first_sort, second_sort, "Sorting should be idempotent");
}

#[test]
fn test_no_fields_removed() {
    use serde_json::Value;

    let input = fs::read_to_string("tests/fixtures/package.json").expect("Failed to read fixture");

    let sorted = sort_package_json(&input).expect("Failed to parse package.json");

    // Parse both as JSON values
    let input_value: Value = serde_json::from_str(&input).expect("Failed to parse input JSON");
    let sorted_value: Value = serde_json::from_str(&sorted).expect("Failed to parse sorted JSON");

    // Extract both as objects
    let input_obj = input_value.as_object().expect("Input should be an object");
    let sorted_obj = sorted_value.as_object().expect("Sorted should be an object");

    // Verify all keys from input exist in sorted output
    for key in input_obj.keys() {
        assert!(sorted_obj.contains_key(key), "Key '{}' was removed during sorting", key);
    }

    // Verify no extra keys were added
    for key in sorted_obj.keys() {
        assert!(input_obj.contains_key(key), "Key '{}' was added during sorting", key);
    }

    // Verify the key count is the same
    assert_eq!(input_obj.len(), sorted_obj.len(), "Number of fields changed during sorting");
}
