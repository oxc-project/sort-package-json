use serde_json::Value;
use sort_package_json::{SortOptions, sort_package_json_with_options};
use std::fs;

fn sort(s: &str) -> String {
    let options = SortOptions::new().with_sort_scripts(true);
    sort_package_json_with_options(s, &options).expect("Failed to parse package.json")
}

/// Find the text span of the object value for a given `"key":` in `json`, starting
/// search at byte offset `from`. Returns the substring `{ ... }` including braces.
fn find_object_text<'a>(json: &'a str, key: &str, from: usize) -> Option<(usize, &'a str)> {
    let needle = format!("\"{key}\":");
    let hit = json[from..].find(&needle)?;
    let after_colon = from + hit + needle.len();
    let brace_rel = json[after_colon..].find('{')?;
    let obj_start = after_colon + brace_rel;

    let mut depth = 0i32;
    let mut in_string = false;
    let mut prev_backslash = false;
    for (i, ch) in json[obj_start..].char_indices() {
        if in_string {
            if ch == '\\' && !prev_backslash {
                prev_backslash = true;
                continue;
            }
            if ch == '"' && !prev_backslash {
                in_string = false;
            }
            prev_backslash = false;
            continue;
        }
        match ch {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some((obj_start, &json[obj_start..=obj_start + i]));
                }
            }
            _ => {}
        }
    }
    None
}

/// Extract the immediate keys (depth-1) of a `{ ... }` JSON object string.
fn immediate_keys(obj_text: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut prev_backslash = false;
    let bytes = obj_text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let ch = bytes[i] as char;
        if in_string {
            if ch == '\\' && !prev_backslash {
                prev_backslash = true;
                i += 1;
                continue;
            }
            if ch == '"' && !prev_backslash {
                in_string = false;
            }
            prev_backslash = false;
            i += 1;
            continue;
        }
        match ch {
            '{' | '[' => {
                depth += 1;
                i += 1;
            }
            '}' | ']' => {
                depth -= 1;
                i += 1;
            }
            '"' if depth == 1 => {
                // Possibly a key. Find the end of this string.
                let start = i + 1;
                let mut end = start;
                let mut esc = false;
                while end < bytes.len() {
                    if esc {
                        esc = false;
                        end += 1;
                        continue;
                    }
                    if bytes[end] == b'\\' {
                        esc = true;
                        end += 1;
                        continue;
                    }
                    if bytes[end] == b'"' {
                        break;
                    }
                    end += 1;
                }
                let candidate = &obj_text[start..end];
                // Check if this is a key (followed by `:`).
                let after_quote = end + 1;
                let rest = obj_text[after_quote..].trim_start();
                if rest.starts_with(':') {
                    keys.push(candidate.to_string());
                }
                i = end + 1;
                in_string = false;
            }
            '"' => {
                in_string = true;
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }
    keys
}

/// Extract the keys of a specific top-level field's object value.
fn get_field_keys(json: &str, field: &str) -> Vec<String> {
    find_object_text(json, field, 0)
        .map(|(_, text)| immediate_keys(text))
        .unwrap_or_default()
}

/// Extract the keys of a nested field: json[field][subfield].
fn get_nested_field_keys(json: &str, field: &str, subfield: &str) -> Vec<String> {
    let (start, _parent_text) = match find_object_text(json, field, 0) {
        Some(v) => v,
        None => return Vec::new(),
    };
    find_object_text(json, subfield, start)
        .map(|(_, text)| immediate_keys(text))
        .unwrap_or_default()
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
    let input =
        fs::read_to_string("tests/fixtures/package-bom.json").expect("Failed to read BOM fixture");
    assert!(input.starts_with(BOM), "Fixture should have BOM");

    let result = sort(&input);
    assert!(result.starts_with(BOM), "BOM should be preserved in output");

    let json_without_bom = &result[BOM.len_utf8()..];
    let parsed: Value =
        serde_json::from_str(json_without_bom).expect("Output should be valid JSON after BOM");
    assert_eq!(parsed.get("name").and_then(|v| v.as_str()), Some("@vitejs/test-utf8-bom-package"));

    // Test 2: Files without BOM don't get BOM added
    let input_no_bom = r#"{"version": "1.0.0", "name": "test"}"#;
    let result_no_bom = sort(input_no_bom);
    assert!(!result_no_bom.starts_with(BOM), "BOM should not be added if not present");

    // Test 3: Idempotency - sorting twice produces same result
    let second_sort = sort(&result);
    assert_eq!(result, second_sort, "Sorting BOM files should be idempotent");
}

#[test]
fn test_json_representation_edge_cases() {
    let input = r#"{
  "version": "first",
  "description": "line\n\u0061",
  "name": "pkg",
  "version": "last",
  "nested": { "duplicate": 1, "duplicate": 2 },
  "number": 1e+01
}"#;

    let pretty = sort_package_json_with_options(input, &SortOptions::new()).unwrap();
    assert_eq!(
        pretty,
        "{\n  \"name\": \"pkg\",\n  \"version\": \"last\",\n  \"description\": \"line\\na\",\n  \"nested\": {\n    \"duplicate\": 2\n  },\n  \"number\": 10.0\n}\n"
    );

    let compact =
        sort_package_json_with_options(input, &SortOptions::new().with_pretty(false)).unwrap();
    assert_eq!(
        compact,
        r#"{"name":"pkg","version":"last","description":"line\na","nested":{"duplicate":2},"number":10.0}"#
    );
}

// ===== exports / imports condition sorting ===================================
//
// These tests are adapted from the npm `sort-package-json` test suite
// (keithamus/sort-package-json, tests/exports.js) and extended with additional
// edge cases for nested structures and the `imports` field.

#[test]
fn exports_paths_should_come_first() {
    let input = r#"{
  "exports": {
    "unknown": "./unknown.unknown",
    "./path-not-really-makes-no-sense": {}
  }
}"#;
    let result = sort(input);
    let keys = get_field_keys(&result, "exports");
    assert_eq!(keys, vec!["./path-not-really-makes-no-sense", "unknown"]);
}

#[test]
fn exports_default_should_be_last() {
    let input = r#"{
  "exports": {
    "unknown": "./unknown.unknown",
    "default": "./default.js",
    "./path-not-really-makes-no-sense": {}
  }
}"#;
    let result = sort(input);
    let keys = get_field_keys(&result, "exports");
    assert_eq!(keys, vec!["./path-not-really-makes-no-sense", "unknown", "default"]);
}

#[test]
fn exports_conditions_retain_original_order() {
    // Condition keys that are not paths and not `default` keep their declaration order.
    // This is critical because Node.js evaluates conditions in source order.
    let input = r#"{
  "exports": {
    "unknown-3": "./unknown.unknown3",
    "./path-not-really-makes-no-sense": {},
    "unknown-1": "./unknown.unknown1",
    "default": "./whatever/index.js",
    "types": "./types.d.ts",
    "unknown-2": "./unknown.unknown2",
    "types@<=1": "./v1/types.d.ts"
  }
}"#;
    let result = sort(input);
    let keys = get_field_keys(&result, "exports");
    assert_eq!(
        keys,
        vec![
            "./path-not-really-makes-no-sense",
            "unknown-3",
            "unknown-1",
            "types",
            "unknown-2",
            "types@<=1",
            "default",
        ]
    );
}

#[test]
fn exports_only_types() {
    let input = r#"{ "exports": { "types": "./types.d.ts" } }"#;
    let result = sort(input);
    let keys = get_field_keys(&result, "exports");
    assert_eq!(keys, vec!["types"]);
}

#[test]
fn exports_only_default() {
    let input = r#"{ "exports": { "default": "./default.js" } }"#;
    let result = sort(input);
    let keys = get_field_keys(&result, "exports");
    assert_eq!(keys, vec!["default"]);
}

#[test]
fn exports_well_formed_types_then_default() {
    // Already well-formed: types before default.
    let input = r#"{ "exports": { "types": "./types.d.ts", "default": "./default.js" } }"#;
    let result = sort(input);
    let keys = get_field_keys(&result, "exports");
    assert_eq!(keys, vec!["types", "default"]);
}

// -- Deep / nested -----------------------------------------------------------

#[test]
fn exports_deep_paths_should_come_first() {
    let input = r#"{
  "exports": {
    "./deep": {
      "unknown": "./unknown.unknown",
      "./path-not-really-makes-no-sense": {}
    }
  }
}"#;
    let result = sort(input);
    let keys = get_nested_field_keys(&result, "exports", "./deep");
    assert_eq!(keys, vec!["./path-not-really-makes-no-sense", "unknown"]);
}

#[test]
fn exports_deep_default_should_be_last() {
    let input = r#"{
  "exports": {
    "./deep": {
      "unknown": "./unknown.unknown",
      "default": "./default.js",
      "./path-not-really-makes-no-sense": {}
    }
  }
}"#;
    let result = sort(input);
    let keys = get_nested_field_keys(&result, "exports", "./deep");
    assert_eq!(keys, vec!["./path-not-really-makes-no-sense", "unknown", "default"]);
}

#[test]
fn exports_deep_conditions_retain_original_order() {
    let input = r#"{
  "exports": {
    "./deep": {
      "unknown-3": "./unknown.unknown3",
      "./path-not-really-makes-no-sense": {},
      "unknown-1": "./unknown.unknown1",
      "default": "./whatever/index.js",
      "types": "./types.d.ts",
      "unknown-2": "./unknown.unknown2",
      "types@<=1": "./v1/types.d.ts"
    }
  }
}"#;
    let result = sort(input);
    let keys = get_nested_field_keys(&result, "exports", "./deep");
    assert_eq!(
        keys,
        vec![
            "./path-not-really-makes-no-sense",
            "unknown-3",
            "unknown-1",
            "types",
            "unknown-2",
            "types@<=1",
            "default",
        ]
    );
}

// -- Real-world patterns -----------------------------------------------------

#[test]
fn exports_typical_dual_package() {
    // A realistic dual CJS/ESM package with conditions out of order.
    let input = r#"{
  "exports": {
    ".": {
      "default": "./dist/index.js",
      "require": "./dist/index.cjs",
      "types": "./dist/index.d.ts",
      "import": "./dist/index.esm.js"
    },
    "./package.json": "./package.json",
    "./utils": {
      "import": "./dist/utils.esm.js",
      "default": "./dist/utils.js",
      "types": "./dist/utils.d.ts"
    }
  }
}"#;
    let result = sort(input);

    // Path keys preserve their original order.
    let keys = get_field_keys(&result, "exports");
    assert_eq!(keys, vec![".", "./package.json", "./utils"]);

    // Within ".", conditions retain order but "default" moves to end.
    let dot_keys = get_nested_field_keys(&result, "exports", ".");
    assert_eq!(dot_keys, vec!["require", "types", "import", "default"]);

    // Within "./utils", same rule.
    let utils_keys = get_nested_field_keys(&result, "exports", "./utils");
    assert_eq!(utils_keys, vec!["import", "types", "default"]);
}

#[test]
fn exports_three_level_nesting() {
    // Conditions can nest arbitrarily: exports → path → condition → condition.
    let input = r#"{
  "exports": {
    ".": {
      "node": {
        "default": "./node.cjs",
        "import": "./node.mjs"
      },
      "default": "./browser.js"
    }
  }
}"#;
    let result = sort(input);

    // Top-level of ".": "node" condition stays, "default" moves to end.
    let dot_keys = get_nested_field_keys(&result, "exports", ".");
    assert_eq!(dot_keys, vec!["node", "default"]);

    // Inside "node": "import" keeps its relative position, "default" at end.
    // We need to find "node" inside exports > ".".
    // Use a targeted text search: find `"node":` after the exports section.
    let node_start = result.find("\"node\":").expect("node key exists");
    let (_, node_text) = find_object_text(&result, "node", node_start - 6)
        .expect("node should be an object");
    let node_keys = immediate_keys(node_text);
    assert_eq!(node_keys, vec!["import", "default"]);
}

#[test]
fn exports_empty_object() {
    let input = r#"{ "exports": {} }"#;
    let result = sort(input);
    let keys = get_field_keys(&result, "exports");
    assert!(keys.is_empty());
}

#[test]
fn exports_string_value_passthrough() {
    // When exports is a plain string, it should be left unchanged.
    let input = r#"{ "exports": "./index.js" }"#;
    let result = sort(input);
    let parsed: Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["exports"], "./index.js");
}

#[test]
fn exports_array_value_passthrough() {
    // When exports is an array (rare but valid), it should be left unchanged.
    let input = r#"{ "exports": ["./a.js", "./b.js"] }"#;
    let result = sort(input);
    let parsed: Value = serde_json::from_str(&result).unwrap();
    assert!(parsed["exports"].is_array());
    assert_eq!(parsed["exports"][0], "./a.js");
    assert_eq!(parsed["exports"][1], "./b.js");
}

#[test]
fn exports_no_default_key() {
    // When there is no "default" key, nothing should be appended.
    let input = r#"{
  "exports": {
    "import": "./index.mjs",
    "require": "./index.cjs",
    "types": "./index.d.ts"
  }
}"#;
    let result = sort(input);
    let keys = get_field_keys(&result, "exports");
    assert_eq!(keys, vec!["import", "require", "types"]);
}

#[test]
fn exports_only_paths_no_conditions() {
    // Object with only path keys, no conditions at all.
    let input = r#"{
  "exports": {
    "./b": "./b.js",
    "./a": "./a.js",
    ".": "./index.js"
  }
}"#;
    let result = sort(input);
    // Path keys preserve original order.
    let keys = get_field_keys(&result, "exports");
    assert_eq!(keys, vec!["./b", "./a", "."]);
}

#[test]
fn exports_mixed_path_condition_same_level() {
    // Path keys and condition keys at the same level.
    let input = r#"{
  "exports": {
    "require": "./index.cjs",
    "./utils": "./utils.js",
    "default": "./index.js",
    ".": "./main.js",
    "import": "./index.mjs"
  }
}"#;
    let result = sort(input);
    let keys = get_field_keys(&result, "exports");
    // Paths first (preserving order), then conditions (preserving order), default last.
    assert_eq!(keys, vec!["./utils", ".", "require", "import", "default"]);
}

#[test]
fn exports_nested_string_values_unchanged() {
    // String values within nested objects should not be touched.
    let input = r#"{
  "exports": {
    ".": {
      "default": "./dist/index.js",
      "types": "./dist/index.d.ts"
    }
  }
}"#;
    let result = sort(input);
    let parsed: Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["exports"]["."]["types"], "./dist/index.d.ts");
    assert_eq!(parsed["exports"]["."]["default"], "./dist/index.js");
}

// -- imports field ------------------------------------------------------------

#[test]
fn imports_default_should_be_last() {
    let input = r##"{
  "imports": {
    "#internal": {
      "default": "./src/internal.js",
      "types": "./src/internal.d.ts",
      "node": "./src/internal-node.js"
    }
  }
}"##;
    let result = sort(input);
    let keys = get_nested_field_keys(&result, "imports", "#internal");
    assert_eq!(keys, vec!["types", "node", "default"]);
}

#[test]
fn imports_paths_first_conditions_after() {
    // The `#` prefix is not `.`, so these are treated as conditions, not paths.
    // "default" goes last as usual.
    let input = r##"{
  "imports": {
    "default": "./fallback.js",
    "#utils": "./src/utils.js",
    "#lib": {
      "default": "./lib.cjs",
      "import": "./lib.mjs"
    }
  }
}"##;
    let result = sort(input);
    let keys = get_field_keys(&result, "imports");
    // "#utils" and "#lib" are not paths (no "."), so they are conditions retaining order.
    // "default" goes last.
    assert_eq!(keys, vec!["#utils", "#lib", "default"]);

    // Nested condition sorting inside "#lib".
    let lib_keys = get_nested_field_keys(&result, "imports", "#lib");
    assert_eq!(lib_keys, vec!["import", "default"]);
}

// -- publishConfig exports ----------------------------------------------------

#[test]
fn publish_config_exports_sorted() {
    // publishConfig applies the full sort_object_keys treatment, which now includes
    // exports condition sorting.
    let input = r#"{
  "publishConfig": {
    "exports": {
      ".": {
        "default": "./dist/index.js",
        "import": "./dist/index.mjs",
        "types": "./dist/index.d.ts"
      }
    }
  }
}"#;
    let result = sort(input);
    // Find the "." inside publishConfig > exports.
    let pc_start = result.find("\"publishConfig\"").expect("publishConfig exists");
    let exports_start = result[pc_start..].find("\"exports\"").expect("exports exists") + pc_start;
    let (dot_start, _) = find_object_text(&result, "exports", exports_start)
        .expect("exports should be an object");
    let (_, dot_text) = find_object_text(&result, ".", dot_start)
        .expect(". should be an object");
    let keys = immediate_keys(dot_text);
    assert_eq!(keys, vec!["import", "types", "default"]);
}

// -- Idempotency for exports --------------------------------------------------

#[test]
fn exports_sorting_is_idempotent() {
    let input = r#"{
  "exports": {
    ".": {
      "default": "./dist/index.js",
      "require": "./dist/index.cjs",
      "types": "./dist/index.d.ts",
      "import": "./dist/index.esm.js"
    },
    "./utils": {
      "import": "./dist/utils.esm.js",
      "default": "./dist/utils.js",
      "types": "./dist/utils.d.ts"
    }
  }
}"#;
    let first = sort(input);
    let second = sort(&first);
    assert_eq!(first, second, "Exports sorting should be idempotent");
}
