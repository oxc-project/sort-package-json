use sort_package_json::{SortOptions, sort_package_json, sort_package_json_with_options};
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

#[test]
fn test_sort_with_compact_format() {
    use serde_json::Value;

    let input = r#"{"version": "1.0.0", "name": "test-package", "description": "A test"}"#;

    let options = SortOptions { pretty: false };
    let result =
        sort_package_json_with_options(input, &options).expect("Failed to sort package.json");

    // Verify it's valid JSON
    let parsed: Value = serde_json::from_str(&result).expect("Result should be valid JSON");

    // Verify it's compact (no newlines or extra whitespace)
    assert!(!result.contains('\n'), "Compact format should not contain newlines");
    assert!(!result.contains("  "), "Compact format should not contain double spaces");

    // Verify field order is correct (name before version)
    let name_pos = result.find("\"name\"").expect("Should contain name field");
    let version_pos = result.find("\"version\"").expect("Should contain version field");
    assert!(name_pos < version_pos, "name should come before version");

    // Verify all fields are present
    let obj = parsed.as_object().expect("Result should be an object");
    assert_eq!(obj.len(), 3, "Should have 3 fields");
    assert!(obj.contains_key("name"));
    assert!(obj.contains_key("version"));
    assert!(obj.contains_key("description"));
}

#[test]
fn test_compact_format_idempotency() {
    let input = fs::read_to_string("tests/fixtures/package.json").expect("Failed to read fixture");

    let options = SortOptions { pretty: false };
    let first_sort = sort_package_json_with_options(&input, &options).expect("First sort failed");
    let second_sort =
        sort_package_json_with_options(&first_sort, &options).expect("Second sort failed");

    assert_eq!(first_sort, second_sort, "Compact format sorting should be idempotent");
}
