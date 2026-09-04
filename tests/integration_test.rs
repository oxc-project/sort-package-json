use serde_json::Value;
use sort_package_json::{SortOptions, sort_package_json_with_options};
use std::fs;

fn sort(s: &str) -> String {
    let options = SortOptions::new().with_sort_scripts(true);
    sort_package_json_with_options(s, &options).expect("Failed to parse package.json")
}

/// Extract the immediate keys of the first `{ ... }` object value for a given
/// `"key":` in the JSON text, preserving serialization order.
///
/// We cannot use `serde_json::Value` for this because its default `Map` is a
/// `BTreeMap` that loses insertion order.
fn get_script_keys(json: &str, field: &str) -> Vec<String> {
    let needle = format!("\"{field}\":");
    let start = match json.find(&needle) {
        Some(pos) => pos + needle.len(),
        None => return Vec::new(),
    };
    let brace_rel = match json[start..].find('{') {
        Some(pos) => pos,
        None => return Vec::new(),
    };
    let obj_start = start + brace_rel;

    // Walk the object text, tracking brace/bracket/string state,
    // and collect keys at depth == 1.
    let mut keys = Vec::new();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut prev_backslash = false;
    let bytes = json.as_bytes();
    let mut i = obj_start;
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
                if depth == 0 {
                    break;
                }
                i += 1;
            }
            '"' if depth == 1 => {
                // Possibly a key. Find the closing quote.
                let s = i + 1;
                let mut e = s;
                let mut esc = false;
                while e < bytes.len() {
                    if esc {
                        esc = false;
                        e += 1;
                        continue;
                    }
                    if bytes[e] == b'\\' {
                        esc = true;
                        e += 1;
                        continue;
                    }
                    if bytes[e] == b'"' {
                        break;
                    }
                    e += 1;
                }
                let candidate = &json[s..e];
                let after = &json[e + 1..].trim_start();
                if after.starts_with(':') {
                    keys.push(candidate.to_string());
                }
                i = e + 1;
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

// ===== Scripts lifecycle sorting =============================================
//
// Tests adapted from the npm `sort-package-json` test suite
// (keithamus/sort-package-json, tests/scripts.js) and extended with additional
// edge cases.

/// Ported from npm sort-package-json: the main fixture with pre/post lifecycle
/// scripts, custom scripts, and a hyphenated name that looks like "pre-*".
#[test]
fn scripts_lifecycle_main_fixture() {
    let input = r#"{
  "scripts": {
    "test": "node test.js",
    "multiply": "2 * 3",
    "watch": "watch things",
    "prewatch": "echo about to watch",
    "postinstall": "echo Installed",
    "preinstall": "echo Installing",
    "start": "node server.js",
    "posttest": "abc",
    "pretest": "xyz",
    "postprettier": "echo so pretty",
    "preprettier": "echo not pretty",
    "prettier": "prettier -l **/*.js",
    "prepare": "npm run build",
    "pre-fetch-info": "foo"
  }
}"#;
    let result = sort(input);
    let keys = get_script_keys(&result, "scripts");
    assert_eq!(
        keys,
        vec![
            "preinstall",
            "postinstall",
            "multiply",
            "pre-fetch-info",
            "prepare",
            "preprettier",
            "prettier",
            "postprettier",
            "start",
            "pretest",
            "test",
            "posttest",
            "prewatch",
            "watch",
        ]
    );
}

/// Ported from npm: pre/post scripts with colons should NOT be grouped with
/// their base lifecycle. `prebuild:1` is a colon-variant of `prebuild`, not a
/// pre-script for `build:1`.
#[test]
fn scripts_does_not_sort_pre_post_colon_together() {
    let input = r#"{
  "scripts": {
    "prebuild": "run-s prebuild:*",
    "prebuild:1": "node prebuild.js 1",
    "prebuild:2": "node prebuild.js 2",
    "prebuild:3": "node prebuild.js 3",
    "build": "run-s build:*",
    "build:bar": "node bar.js",
    "build:baz": "node baz.js",
    "build:foo": "node foo.js",
    "postbuild": "run-s prebuild:*",
    "postbuild:1": "node prebuild.js 1",
    "postbuild:2": "node prebuild.js 2",
    "postbuild:3": "node prebuild.js 3",
    "d-unrelated": "..",
    "e-unrelated": "..",
    "f-unrelated": ".."
  }
}"#;
    let result = sort(input);
    let keys = get_script_keys(&result, "scripts");
    assert_eq!(
        keys,
        vec![
            "prebuild",
            "build",
            "postbuild",
            "build:bar",
            "build:baz",
            "build:foo",
            "d-unrelated",
            "e-unrelated",
            "f-unrelated",
            "postbuild:1",
            "postbuild:2",
            "postbuild:3",
            "prebuild:1",
            "prebuild:2",
            "prebuild:3",
        ]
    );
}

/// Ported from npm: pre/post scripts for colon-namespaced scripts.
/// `pretest:es-check` and `posttest:es-check` should surround `test:es-check`.
#[test]
fn scripts_pre_post_for_colon_scripts() {
    let input = r#"{
  "scripts": {
    "pretest:es-check": "echo",
    "posttest:es-check": "echo",
    "test": "echo",
    "test:coverage": "echo",
    "test:es-check": "echo",
    "test:types": "echo"
  }
}"#;
    let result = sort(input);
    let keys = get_script_keys(&result, "scripts");
    assert_eq!(
        keys,
        vec![
            "test",
            "test:coverage",
            "pretest:es-check",
            "test:es-check",
            "posttest:es-check",
            "test:types",
        ]
    );
}

/// Ported from npm: base and colon scripts grouped together, not split by
/// unrelated scripts that happen to sort between them.
#[test]
fn scripts_group_base_and_colon_together() {
    let input = r#"{
  "scripts": {
    "test": "run-s test:a test:b",
    "test:a": "foo",
    "test:b": "bar",
    "test-coverage": "c8 node --run test"
  }
}"#;
    let result = sort(input);
    let keys = get_script_keys(&result, "scripts");
    assert_eq!(keys, vec!["test", "test:a", "test:b", "test-coverage"]);
}

/// Ported from npm: scripts with multiple colons are recursively grouped.
#[test]
fn scripts_multiple_colons() {
    let input = r#"{
  "scripts": {
    "test": "run-s test:a test:b",
    "test:a": "foo",
    "test:b": "bar",
    "pretest:a": "foo",
    "posttest:a": "foo",
    "pretest:a:a": "foo",
    "posttest:a:a": "foo",
    "test:a:a": "foofoo",
    "test:a:b": "foobar",
    "pretest:ab": "foobar",
    "test:ab": "foobar",
    "test:a-coverage": "foobar",
    "test:b:a": "barfoo",
    "test:b:b": "barbar",
    "test-coverage": "c8 node --run test"
  }
}"#;
    let result = sort(input);
    let keys = get_script_keys(&result, "scripts");
    assert_eq!(
        keys,
        vec![
            "test",
            "pretest:a",
            "test:a",
            "posttest:a",
            "pretest:a:a",
            "test:a:a",
            "posttest:a:a",
            "test:a:b",
            "test:a-coverage",
            "pretest:ab",
            "test:ab",
            "test:b",
            "test:b:a",
            "test:b:b",
            "test-coverage",
        ]
    );
}

/// Ported from npm: handles names starting with colon and double colons.
#[test]
fn scripts_colon_prefix_and_double_colons() {
    let input = r#"{
  "scripts": {
    "::delta": "echo",
    "test": "echo",
    ":beta": "echo",
    ":alpha:sub": "echo",
    ":alpha": "echo",
    "test::coverage": "echo",
    ":alpha::extra": "echo",
    "test:lint": "echo",
    "test::smoke": "echo"
  }
}"#;
    let result = sort(input);
    let keys = get_script_keys(&result, "scripts");
    assert_eq!(
        keys,
        vec![
            "::delta",
            ":alpha",
            ":alpha::extra",
            ":alpha:sub",
            ":beta",
            "test",
            "test::coverage",
            "test::smoke",
            "test:lint",
        ]
    );
}

/// Ported from npm: nested production and format variants grouped and sorted.
#[test]
fn scripts_nested_production_variants() {
    let input = r#"{
  "scripts": {
    "test": "echo",
    "test:a": "echo",
    "test:b": "echo",
    "test:ab": "echo",
    "test-coverage": "echo",
    "test:production": "echo",
    "test:production:a": "echo",
    "test:production:b": "echo",
    "test:production-coverage": "echo",
    "test:production2": "echo",
    "test:production$2": "echo",
    "test:production:cjs": "echo",
    "test:production:cjs:a": "echo",
    "test:production:cjs:b": "echo",
    "test:production:cjs-coverage": "echo",
    "test:production:mjs": "echo",
    "test:production:mjs:a": "echo",
    "test:production:mjs:b": "echo",
    "test:production:mjs-coverage": "echo"
  }
}"#;
    let result = sort(input);
    let keys = get_script_keys(&result, "scripts");
    assert_eq!(
        keys,
        vec![
            "test",
            "test:a",
            "test:ab",
            "test:b",
            "test:production",
            "test:production:a",
            "test:production:b",
            "test:production:cjs",
            "test:production:cjs:a",
            "test:production:cjs:b",
            "test:production:cjs-coverage",
            "test:production:mjs",
            "test:production:mjs:a",
            "test:production:mjs:b",
            "test:production:mjs-coverage",
            "test:production$2",
            "test:production-coverage",
            "test:production2",
            "test-coverage",
        ]
    );
}

// -- Additional edge cases ---------------------------------------------------

/// `preinstall` and `postinstall` recognized even without explicit `install` script.
#[test]
fn scripts_lifecycle_without_base_script() {
    let input = r#"{
  "scripts": {
    "postinstall": "echo done",
    "preinstall": "echo start",
    "build": "tsc"
  }
}"#;
    let result = sort(input);
    let keys = get_script_keys(&result, "scripts");
    assert_eq!(keys, vec!["build", "preinstall", "postinstall"]);
}

/// Only `pre` without matching `post` (or vice versa).
#[test]
fn scripts_pre_only() {
    let input = r#"{
  "scripts": {
    "pretest": "lint",
    "test": "jest"
  }
}"#;
    let result = sort(input);
    let keys = get_script_keys(&result, "scripts");
    assert_eq!(keys, vec!["pretest", "test"]);
}

#[test]
fn scripts_post_only() {
    let input = r#"{
  "scripts": {
    "posttest": "coverage",
    "test": "jest"
  }
}"#;
    let result = sort(input);
    let keys = get_script_keys(&result, "scripts");
    assert_eq!(keys, vec!["test", "posttest"]);
}

/// `pre-*` with a hyphen is NOT a lifecycle prefix — it is an ordinary script.
#[test]
fn scripts_hyphenated_pre_not_lifecycle() {
    let input = r#"{
  "scripts": {
    "pre-deploy": "build",
    "deploy": "push",
    "predeploy": "lint"
  }
}"#;
    let result = sort(input);
    let keys = get_script_keys(&result, "scripts");
    // "predeploy" IS a lifecycle prefix for "deploy"; "pre-deploy" is just alphabetical.
    assert_eq!(keys, vec!["predeploy", "deploy", "pre-deploy"]);
}

/// Empty scripts object.
#[test]
fn scripts_empty() {
    let input = r#"{ "scripts": {} }"#;
    let result = sort(input);
    let keys = get_script_keys(&result, "scripts");
    assert!(keys.is_empty());
}

/// Single script.
#[test]
fn scripts_single() {
    let input = r#"{ "scripts": { "build": "tsc" } }"#;
    let result = sort(input);
    let keys = get_script_keys(&result, "scripts");
    assert_eq!(keys, vec!["build"]);
}

/// Pure alphabetical when no pre/post or colons.
#[test]
fn scripts_plain_alphabetical() {
    let input = r#"{
  "scripts": {
    "z": "z",
    "a": "a",
    "m": "m"
  }
}"#;
    let result = sort(input);
    let keys = get_script_keys(&result, "scripts");
    assert_eq!(keys, vec!["a", "m", "z"]);
}

/// `betterScripts` field gets the same treatment.
#[test]
fn better_scripts_same_sorting() {
    let input = r#"{
  "betterScripts": {
    "posttest": "coverage",
    "test": "jest",
    "pretest": "lint",
    "build": "tsc"
  }
}"#;
    let result = sort(input);
    let keys = get_script_keys(&result, "betterScripts");
    assert_eq!(keys, vec!["build", "pretest", "test", "posttest"]);
}

/// When `sort_scripts` is false, order is preserved.
#[test]
fn scripts_sort_disabled() {
    let input = r#"{
  "scripts": {
    "posttest": "coverage",
    "test": "jest",
    "pretest": "lint",
    "build": "tsc"
  }
}"#;
    let options = SortOptions::new().with_sort_scripts(false);
    let result = sort_package_json_with_options(input, &options).unwrap();
    let keys = get_script_keys(&result, "scripts");
    assert_eq!(keys, vec!["posttest", "test", "pretest", "build"]);
}

/// Scripts sorting is idempotent.
#[test]
fn scripts_idempotent() {
    let input = r#"{
  "scripts": {
    "test": "node test.js",
    "posttest": "abc",
    "pretest": "xyz",
    "build": "tsc",
    "build:watch": "tsc -w",
    "prebuild": "clean"
  }
}"#;
    let first = sort(input);
    let second = sort(&first);
    assert_eq!(first, second, "Scripts sorting should be idempotent");
}

/// Orphan `pre`/`post` scripts (no matching base or lifecycle) treated as regular.
#[test]
fn scripts_orphan_pre_post() {
    let input = r#"{
  "scripts": {
    "prefoo": "echo pre",
    "postbar": "echo post",
    "build": "tsc"
  }
}"#;
    let result = sort(input);
    let keys = get_script_keys(&result, "scripts");
    // No "foo" or "bar" script exists and they are not default npm lifecycle names,
    // so "prefoo" and "postbar" are treated as ordinary scripts sorted alphabetically.
    assert_eq!(keys, vec!["build", "postbar", "prefoo"]);
}

/// All default npm lifecycle scripts recognized without explicit base.
#[test]
fn scripts_all_default_lifecycle_scripts() {
    let input = r#"{
  "scripts": {
    "postversion": "push",
    "preversion": "lint",
    "poststop": "log",
    "prestop": "warn",
    "poststart": "notify",
    "prestart": "check"
  }
}"#;
    let result = sort(input);
    let keys = get_script_keys(&result, "scripts");
    assert_eq!(
        keys,
        vec![
            "prestart",
            "poststart",
            "prestop",
            "poststop",
            "preversion",
            "postversion",
        ]
    );
}

/// Scripts values are preserved (not just keys).
#[test]
fn scripts_values_preserved() {
    let input = r#"{
  "scripts": {
    "posttest": "echo post",
    "test": "jest --coverage",
    "pretest": "echo pre"
  }
}"#;
    let result = sort(input);
    let parsed: Value = serde_json::from_str(&result).unwrap();
    let scripts = parsed.get("scripts").unwrap();
    assert_eq!(scripts.get("pretest").and_then(|v| v.as_str()), Some("echo pre"));
    assert_eq!(scripts.get("test").and_then(|v| v.as_str()), Some("jest --coverage"));
    assert_eq!(scripts.get("posttest").and_then(|v| v.as_str()), Some("echo post"));
}
