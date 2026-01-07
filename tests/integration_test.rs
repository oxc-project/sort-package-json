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

#[test]
fn test_comprehensive_no_data_deletion() {
    // Comprehensive test covering ALL fields that might have transformations
    let input = r#"{
  "name": "test",
  "version": "1.0.0",
  "keywords": ["z", "a", 1, true, null, {"x": "y"}],
  "categories": ["cat2", "cat1", false, [1, 2]],
  "bugs": {"extra": "field", "url": "https://example.com", "email": "test@example.com"},
  "author": {"extra": "data", "name": "Author", "email": "author@example.com", "url": "https://example.com"},
  "repository": {"extra": "info", "type": "git", "url": "https://github.com/test/repo"},
  "funding": {"extra": true, "type": "github", "url": "https://github.com/sponsors/test"},
  "bin": {"zzz": "./z.js", "aaa": "./a.js", "extra": 123},
  "directories": {"extra": "dir", "lib": "./lib", "bin": "./bin", "doc": "./docs"},
  "files": ["z.js", "a.js", 999, {"pattern": "*.ts"}],
  "exports": {
    "./path": {
      "extra": "value",
      "types": "./types.d.ts",
      "import": "./index.mjs",
      "default": "./index.js"
    },
    "extra": "field"
  },
  "publishConfig": {"zzz": "last", "aaa": "first", "extra": null},
  "dependencies": {"zzz": "1.0.0", "aaa": "2.0.0", "extra": 123},
  "devDependencies": {"zzz": "1.0.0", "aaa": "2.0.0", "extra": false},
  "peerDependencies": {"zzz": ">=1.0.0", "aaa": ">=2.0.0"},
  "optionalDependencies": {"zzz": "1.0.0", "aaa": "2.0.0"},
  "bundledDependencies": ["zzz", "aaa", 789, null],
  "bundleDependencies": ["pkg2", "pkg1", true],
  "resolutions": {"zzz": "1.0.0", "aaa": "2.0.0"},
  "overrides": {"zzz": "1.0.0", "aaa": "2.0.0"},
  "engines": {"zzz": ">=1.0.0", "node": ">=18.0.0", "extra": true},
  "libc": ["glibc", "musl", 456, false],
  "activationEvents": ["onLanguage:javascript", 123],
  "extensionPack": ["ext1", "ext2", null],
  "extensionDependencies": ["dep1", "dep2", true],
  "extensionKind": ["ui", "workspace", {"obj": "val"}],
  "customField1": "preserved",
  "customField2": {"nested": "data", "count": 42},
  "_private1": "also preserved",
  "_private2": [1, 2, 3]
}"#;

    let result = sort(input);
    let parsed: Value = serde_json::from_str(&result).expect("Failed to parse result");

    // Count total fields in input vs output
    let input_parsed: Value = serde_json::from_str(input).expect("Failed to parse input");
    let input_fields = input_parsed.as_object().unwrap().len();
    let output_fields = parsed.as_object().unwrap().len();
    assert_eq!(input_fields, output_fields, "Field count should be preserved: input={}, output={}", input_fields, output_fields);

    // Verify array lengths are preserved
    assert_eq!(parsed.get("keywords").unwrap().as_array().unwrap().len(), 6, "keywords array length");
    assert_eq!(parsed.get("categories").unwrap().as_array().unwrap().len(), 4, "categories array length");
    assert_eq!(parsed.get("files").unwrap().as_array().unwrap().len(), 4, "files array length");
    assert_eq!(parsed.get("bundledDependencies").unwrap().as_array().unwrap().len(), 4, "bundledDependencies array length");
    assert_eq!(parsed.get("bundleDependencies").unwrap().as_array().unwrap().len(), 3, "bundleDependencies array length");
    assert_eq!(parsed.get("libc").unwrap().as_array().unwrap().len(), 4, "libc array length");
    assert_eq!(parsed.get("activationEvents").unwrap().as_array().unwrap().len(), 2, "activationEvents array length");
    assert_eq!(parsed.get("extensionPack").unwrap().as_array().unwrap().len(), 3, "extensionPack array length");
    assert_eq!(parsed.get("extensionDependencies").unwrap().as_array().unwrap().len(), 3, "extensionDependencies array length");
    assert_eq!(parsed.get("extensionKind").unwrap().as_array().unwrap().len(), 3, "extensionKind array length");

    // Verify object field counts are preserved (including "extra" fields)
    assert_eq!(parsed.get("bugs").unwrap().as_object().unwrap().len(), 3, "bugs object field count");
    assert_eq!(parsed.get("author").unwrap().as_object().unwrap().len(), 4, "author object field count");
    assert_eq!(parsed.get("repository").unwrap().as_object().unwrap().len(), 3, "repository object field count");
    assert_eq!(parsed.get("funding").unwrap().as_object().unwrap().len(), 3, "funding object field count");
    assert_eq!(parsed.get("bin").unwrap().as_object().unwrap().len(), 3, "bin object field count");
    assert_eq!(parsed.get("directories").unwrap().as_object().unwrap().len(), 4, "directories object field count");
    assert_eq!(parsed.get("publishConfig").unwrap().as_object().unwrap().len(), 3, "publishConfig object field count");
    assert_eq!(parsed.get("dependencies").unwrap().as_object().unwrap().len(), 3, "dependencies object field count");
    assert_eq!(parsed.get("devDependencies").unwrap().as_object().unwrap().len(), 3, "devDependencies object field count");
    assert_eq!(parsed.get("engines").unwrap().as_object().unwrap().len(), 3, "engines object field count");
    
    // Verify nested exports structure is preserved
    let exports = parsed.get("exports").unwrap().as_object().unwrap();
    assert_eq!(exports.len(), 2, "exports object field count");
    assert!(exports.contains_key("extra"), "exports should have 'extra' field");
    let path_export = exports.get("./path").unwrap().as_object().unwrap();
    assert_eq!(path_export.len(), 4, "exports path object should have 4 fields including 'extra'");
    assert!(path_export.contains_key("extra"), "exports path should have 'extra' field");

    // Verify custom fields are preserved
    assert!(parsed.get("customField1").is_some(), "customField1 should exist");
    assert!(parsed.get("customField2").is_some(), "customField2 should exist");
    
    // Verify private fields are preserved
    assert!(parsed.get("_private1").is_some(), "_private1 should exist");
    assert!(parsed.get("_private2").is_some(), "_private2 should exist");
}

#[test]
fn test_nested_object_field_preservation() {
    // Test that nested objects in recursive sorting preserve all fields
    let input = r#"{
  "name": "test",
  "version": "1.0.0",
  "babel": {
    "zzz": "last",
    "aaa": "first",
    "nested": {
      "zzz": "last",
      "aaa": "first",
      "deep": {
        "zzz": "last",
        "aaa": "first",
        "number": 123,
        "bool": true
      }
    }
  }
}"#;

    let result = sort(input);
    let parsed: Value = serde_json::from_str(&result).expect("Failed to parse result");

    let babel = parsed.get("babel").unwrap().as_object().unwrap();
    assert_eq!(babel.len(), 3, "babel should have 3 fields");
    
    let nested = babel.get("nested").unwrap().as_object().unwrap();
    assert_eq!(nested.len(), 3, "nested should have 3 fields");
    
    let deep = nested.get("deep").unwrap().as_object().unwrap();
    assert_eq!(deep.len(), 4, "deep should have 4 fields");
    assert!(deep.contains_key("number"), "should preserve number field");
    assert!(deep.contains_key("bool"), "should preserve bool field");
}

#[test]
fn test_array_deduplication_only_for_strings() {
    // Test that deduplication only applies to strings, not other types
    let input = r#"{
  "name": "test",
  "version": "1.0.0",
  "keywords": ["duplicate", "duplicate", 123, 123, true, true, null, null]
}"#;

    let result = sort(input);
    let parsed: Value = serde_json::from_str(&result).expect("Failed to parse result");

    let keywords = parsed.get("keywords").unwrap().as_array().unwrap();
    // String "duplicate" should be deduplicated to 1, but numbers, bools, nulls preserved
    // Expected: ["duplicate", 123, 123, true, true, null, null] = 7 elements
    assert_eq!(keywords.len(), 7, "keywords should have 7 elements: 1 deduplicated string + 6 non-string values");
    
    // Verify the deduplicated string is first
    assert_eq!(keywords[0].as_str(), Some("duplicate"));
    
    // Verify non-strings are preserved
    assert_eq!(keywords[1].as_u64(), Some(123));
    assert_eq!(keywords[2].as_u64(), Some(123));
    assert_eq!(keywords[3].as_bool(), Some(true));
    assert_eq!(keywords[4].as_bool(), Some(true));
    assert!(keywords[5].is_null());
    assert!(keywords[6].is_null());
}
