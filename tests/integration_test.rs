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
fn test_utf8_bom_preservation() {
    // Test case based on https://github.com/vitejs/vite/blob/main/playground/resolve/utf8-bom-package/package.json
    const BOM: char = '\u{FEFF}';

    // Test 1: Files with BOM preserve it
    let input = fs::read_to_string("tests/fixtures/package-bom.json")
        .expect("Failed to read BOM fixture");
    assert!(input.starts_with(BOM), "Fixture should have BOM");

    let result = sort(&input);
    assert!(result.starts_with(BOM), "BOM should be preserved in output");

    let json_without_bom = &result[BOM.len_utf8()..];
    let parsed: Value = serde_json::from_str(json_without_bom)
        .expect("Output should be valid JSON after BOM");
    assert_eq!(parsed.get("name").and_then(|v| v.as_str()),
               Some("@vitejs/test-utf8-bom-package"));

    // Test 2: Files without BOM don't get BOM added
    let input_no_bom = r#"{"version": "1.0.0", "name": "test"}"#;
    let result_no_bom = sort(input_no_bom);
    assert!(!result_no_bom.starts_with(BOM), "BOM should not be added if not present");

    // Test 3: Idempotency - sorting twice produces same result
    let second_sort = sort(&result);
    assert_eq!(result, second_sort, "Sorting BOM files should be idempotent");
}
