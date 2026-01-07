use serde_json::Value;
use sort_package_json::{SortOptions, sort_package_json_with_options};
use std::fs;

fn sort(s: &str) -> String {
    sort_package_json_with_options(s, &SortOptions { pretty: true, sort_scripts: true })
        .expect("Failed to parse package.json")
}

#[test]
fn test_sort_package_json() {
    let input = fs::read_to_string("tests/fixtures/package.json").expect("Failed to read fixture");
    let result = sort(&input);
    insta::assert_snapshot!(result);
}

#[test]
fn test_idempotency() {
    let input = fs::read_to_string("tests/fixtures/package.json").expect("Failed to read fixture");
    let first_sort = sort(&input);
    let second_sort = sort(&first_sort);
    assert_eq!(first_sort, second_sort, "Sorting should be idempotent");
}

#[test]
fn test_size_limit_preservation() {
    let input = r#"{
  "$schema": "https://json.schemastore.org/package.json",
  "name": "test",
  "version": "1.0.0",
  "size-limit": [
    {
      "name": "useQuery only from source",
      "path": "src/index.ts",
      "import": "{ useQuery, PiniaColada }",
      "ignore": ["vue", "pinia", "@vue/devtools-api"]
    }
  ]
}"#;

    let result = sort(input);
    let parsed: Value = serde_json::from_str(&result).expect("Failed to parse result");

    // Check that size-limit field exists
    assert!(parsed.get("size-limit").is_some(), "size-limit field should exist");

    // Check that it's an array
    let size_limit = parsed.get("size-limit").unwrap();
    assert!(size_limit.is_array(), "size-limit should be an array");

    // Check that the array has one element
    let size_limit_array = size_limit.as_array().unwrap();
    assert_eq!(size_limit_array.len(), 1, "size-limit should have 1 element");

    // Check that the element is an object with expected properties
    let first_entry = &size_limit_array[0];
    assert!(first_entry.is_object(), "size-limit entry should be an object");
    assert_eq!(first_entry.get("name").and_then(|v| v.as_str()), Some("useQuery only from source"));
    assert_eq!(first_entry.get("path").and_then(|v| v.as_str()), Some("src/index.ts"));
    assert_eq!(
        first_entry.get("import").and_then(|v| v.as_str()),
        Some("{ useQuery, PiniaColada }")
    );

    // Check that the ignore array is preserved
    let ignore = first_entry.get("ignore").unwrap();
    assert!(ignore.is_array(), "ignore should be an array");
    let ignore_array = ignore.as_array().unwrap();
    assert_eq!(ignore_array.len(), 3, "ignore should have 3 elements");
}

#[test]
fn test_unknown_fields_preservation() {
    let input = r#"{
  "name": "test",
  "version": "1.0.0",
  "customField": "custom value",
  "anotherCustom": {
    "nested": "data",
    "count": 42
  },
  "customArray": [1, 2, 3]
}"#;

    let result = sort(input);
    let parsed: Value = serde_json::from_str(&result).expect("Failed to parse result");

    // All custom fields should be preserved
    assert!(parsed.get("customField").is_some(), "customField should exist");
    assert_eq!(parsed.get("customField").and_then(|v| v.as_str()), Some("custom value"));

    assert!(parsed.get("anotherCustom").is_some(), "anotherCustom should exist");
    let another_custom = parsed.get("anotherCustom").unwrap();
    assert_eq!(another_custom.get("nested").and_then(|v| v.as_str()), Some("data"));
    assert_eq!(another_custom.get("count").and_then(|v| v.as_u64()), Some(42));

    assert!(parsed.get("customArray").is_some(), "customArray should exist");
    let custom_array = parsed.get("customArray").unwrap().as_array().unwrap();
    assert_eq!(custom_array.len(), 3, "customArray should have 3 elements");
}

#[test]
fn test_array_fields_preserve_non_strings() {
    // Test that array fields that use sort_array_unique preserve non-string values
    let input = r#"{
  "name": "test",
  "version": "1.0.0",
  "keywords": ["string", 123, true, {"obj": "val"}, ["nested"]],
  "categories": ["cat1", null, false, 456]
}"#;

    let result = sort(input);
    let parsed: Value = serde_json::from_str(&result).expect("Failed to parse result");

    // Keywords should preserve all 5 values, not just strings
    let keywords = parsed.get("keywords").unwrap().as_array().unwrap();
    assert_eq!(keywords.len(), 5, "keywords should preserve all 5 values including non-strings, but got {}", keywords.len());

    // Categories should preserve all 4 values
    let categories = parsed.get("categories").unwrap().as_array().unwrap();
    assert_eq!(categories.len(), 4, "categories should preserve all 4 values including non-strings, but got {}", categories.len());
}

#[test]
fn test_private_fields_preservation() {
    let input = r#"{
  "name": "test",
  "version": "1.0.0",
  "_internal": "hidden data",
  "_private": {
    "secret": "value",
    "count": 99
  }
}"#;

    let result = sort(input);
    let parsed: Value = serde_json::from_str(&result).expect("Failed to parse result");

    // Private fields should be preserved
    assert!(parsed.get("_internal").is_some(), "_internal should exist");
    assert_eq!(parsed.get("_internal").and_then(|v| v.as_str()), Some("hidden data"));

    assert!(parsed.get("_private").is_some(), "_private should exist");
    let private_obj = parsed.get("_private").unwrap();
    assert_eq!(private_obj.get("secret").and_then(|v| v.as_str()), Some("value"));
    assert_eq!(private_obj.get("count").and_then(|v| v.as_u64()), Some(99));
}

#[test]
fn test_invalid_value_types_preservation() {
    // Test that fields with unexpected types are preserved as-is
    let input = r#"{
  "name": "test",
  "version": 123,
  "scripts": ["should", "be", "object"],
  "dependencies": "should be object",
  "keywords": "should be array"
}"#;

    let result = sort(input);
    let parsed: Value = serde_json::from_str(&result).expect("Failed to parse result");

    // All fields should be preserved even with wrong types
    assert!(parsed.get("name").is_some());
    assert!(parsed.get("version").is_some());
    assert_eq!(parsed.get("version").and_then(|v| v.as_u64()), Some(123));
    assert!(parsed.get("scripts").is_some());
    assert!(parsed.get("scripts").unwrap().is_array());
    assert!(parsed.get("dependencies").is_some());
    assert!(parsed.get("dependencies").unwrap().is_string());
    assert!(parsed.get("keywords").is_some());
    assert!(parsed.get("keywords").unwrap().is_string());
}

#[test]
fn test_bundled_dependencies_preserve_all_values() {
    // Test that bundledDependencies preserves non-string values
    let input = r#"{
  "name": "test",
  "version": "1.0.0",
  "bundledDependencies": ["pkg1", "pkg2", 123, true, {"obj": "value"}]
}"#;

    let result = sort(input);
    let parsed: Value = serde_json::from_str(&result).expect("Failed to parse result");

    let bundled = parsed.get("bundledDependencies").unwrap().as_array().unwrap();
    // Should preserve all 5 elements, not just the 2 string values
    assert_eq!(bundled.len(), 5, "bundledDependencies should preserve all 5 values including non-strings, but got {}", bundled.len());
}

#[test]
fn test_files_preserve_non_strings() {
    // Test that the "files" field preserves non-string values
    let input = r#"{
  "name": "test",
  "version": "1.0.0",
  "files": ["src", "dist/lib", 123, false, {"pattern": "*.js"}]
}"#;

    let result = sort(input);
    let parsed: Value = serde_json::from_str(&result).expect("Failed to parse result");

    let files = parsed.get("files").unwrap().as_array().unwrap();
    // Should preserve all 5 elements, not just the 2 string values
    assert_eq!(files.len(), 5, "files should preserve all 5 values including non-strings, but got {}", files.len());
}
