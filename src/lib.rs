use serde_json::{Map, Value};

/// Options for controlling JSON formatting when sorting
#[derive(Debug, Clone)]
pub struct SortOptions {
    /// Whether to pretty-print the output JSON
    pub pretty: bool,
}

impl Default for SortOptions {
    fn default() -> Self {
        Self { pretty: true }
    }
}

/// Sorts a package.json string with custom options
pub fn sort_package_json_with_options(
    input: &str,
    options: &SortOptions,
) -> Result<String, serde_json::Error> {
    let value: Value = serde_json::from_str(input)?;

    let sorted_value =
        if let Value::Object(obj) = value { Value::Object(sort_object_keys(obj)) } else { value };

    let result = if options.pretty {
        let mut s = serde_json::to_string_pretty(&sorted_value)?;
        s.push('\n');
        s
    } else {
        serde_json::to_string(&sorted_value)?
    };

    Ok(result)
}

/// Sorts a package.json string with default options (pretty-printed)
pub fn sort_package_json(input: &str) -> Result<String, serde_json::Error> {
    sort_package_json_with_options(input, &SortOptions::default())
}

/// Declares package.json field ordering with transformations.
///
/// This macro generates a match statement that handles known package.json fields
/// in a specific order using explicit indices. It supports optional transformation
/// expressions for fields that need special processing.
///
/// # Usage
///
/// ```ignore
/// declare_field_order!(key, value, known, non_private, private; [
///     0 => "$schema",
///     1 => "name",
///     7 => "categories" => transform_array(&value, sort_array_unique),
/// ]);
/// ```
///
/// # Parameters
///
/// - `key`: The field name identifier
/// - `value`: The field value identifier
/// - `known`: The vector to push known fields to
/// - `non_private`: The vector to push non-private unknown fields to
/// - `private`: The vector to push private (underscore-prefixed) fields to
/// - Followed by an array of field declarations in the format:
///   - `index => "field_name"` for fields without transformation
///   - `index => "field_name" => transformation_expr` for fields with transformation
macro_rules! declare_field_order {
    (
        $key:ident, $value:ident, $known:ident, $non_private:ident, $private:ident;
        [
            $( $idx:literal => $field_name:literal $( => $transform:expr )? ),* $(,)?
        ]
    ) => {
        {
            // Compile-time validation: ensure indices are literals
            $( let _ = $idx; )*

            // Generate the match statement
            match $key.as_str() {
                $(
                    $field_name => {
                        $known.push((
                            $idx,
                            $key,
                            declare_field_order!(@value $value $(, $transform)?)
                        ));
                    },
                )*
                _ => {
                    // Unknown field - check if private
                    if $key.starts_with('_') {
                        $private.push(($key, $value));
                    } else {
                        $non_private.push(($key, $value));
                    }
                }
            }
        }
    };

    // Helper: extract value without transformation
    (@value $value:ident) => { $value };

    // Helper: extract value with transformation
    (@value $value:ident, $transform:expr) => { $transform };
}

fn transform_value<F>(value: Value, transform: F) -> Value
where
    F: FnOnce(Map<String, Value>) -> Map<String, Value>,
{
    match value {
        Value::Object(o) => Value::Object(transform(o)),
        _ => value,
    }
}

fn transform_array<F>(value: Value, transform: F) -> Value
where
    F: FnOnce(Vec<Value>) -> Vec<Value>,
{
    match value {
        Value::Array(arr) => Value::Array(transform(arr)),
        _ => value,
    }
}

fn transform_with_key_order(value: Value, key_order: &[&str]) -> Value {
    transform_value(value, |o| sort_object_by_key_order(o, key_order))
}

fn transform_people_array(value: Value) -> Value {
    transform_array(value, |arr| {
        arr.into_iter()
            .map(|v| match v {
                Value::Object(o) => Value::Object(sort_people_object(o)),
                _ => v,
            })
            .collect()
    })
}

fn sort_object_alphabetically(obj: Map<String, Value>) -> Map<String, Value> {
    let mut entries: Vec<(String, Value)> = obj.into_iter().collect();
    entries.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));
    entries.into_iter().collect()
}

fn sort_object_recursive(obj: Map<String, Value>) -> Map<String, Value> {
    let mut entries: Vec<(String, Value)> = obj.into_iter().collect();
    entries.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));

    entries
        .into_iter()
        .map(|(key, value)| {
            let transformed_value = match value {
                Value::Object(nested) => Value::Object(sort_object_recursive(nested)),
                _ => value,
            };
            (key, transformed_value)
        })
        .collect()
}

fn sort_array_unique(arr: Vec<Value>) -> Vec<Value> {
    let mut strings: Vec<String> =
        arr.iter().filter_map(|v| v.as_str().map(String::from)).collect();

    strings.sort();
    strings.dedup();

    strings.into_iter().map(Value::String).collect()
}

fn sort_object_by_key_order(mut obj: Map<String, Value>, key_order: &[&str]) -> Map<String, Value> {
    let mut result = Map::new();

    // Add keys in specified order
    for &key in key_order {
        if let Some(value) = obj.remove(key) {
            result.insert(key.to_string(), value);
        }
    }

    // Add remaining keys alphabetically
    let mut remaining: Vec<(String, Value)> = obj.into_iter().collect();
    remaining.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));

    for (key, value) in remaining {
        result.insert(key, value);
    }

    result
}

fn sort_people_object(obj: Map<String, Value>) -> Map<String, Value> {
    sort_object_by_key_order(obj, &["name", "email", "url"])
}

fn sort_exports(obj: Map<String, Value>) -> Map<String, Value> {
    let mut paths = Vec::new();
    let mut types_conds = Vec::new();
    let mut other_conds = Vec::new();
    let mut default_cond = None;

    for (key, value) in obj {
        if key.starts_with('.') {
            paths.push((key, value));
        } else if key == "default" {
            default_cond = Some((key, value));
        } else if key == "types" || key.starts_with("types@") {
            types_conds.push((key, value));
        } else {
            other_conds.push((key, value));
        }
    }

    let mut result = Map::new();

    // Add in order: paths, types, others, default
    for (key, value) in paths {
        let transformed = match value {
            Value::Object(nested) => Value::Object(sort_exports(nested)),
            _ => value,
        };
        result.insert(key, transformed);
    }

    for (key, value) in types_conds {
        let transformed = match value {
            Value::Object(nested) => Value::Object(sort_exports(nested)),
            _ => value,
        };
        result.insert(key, transformed);
    }

    for (key, value) in other_conds {
        let transformed = match value {
            Value::Object(nested) => Value::Object(sort_exports(nested)),
            _ => value,
        };
        result.insert(key, transformed);
    }

    if let Some((key, value)) = default_cond {
        let transformed = match value {
            Value::Object(nested) => Value::Object(sort_exports(nested)),
            _ => value,
        };
        result.insert(key, transformed);
    }

    result
}

fn sort_object_keys(obj: Map<String, Value>) -> Map<String, Value> {
    // Storage for categorized keys with their values and ordering information
    let mut known: Vec<(usize, String, Value)> = Vec::new(); // (order_index, key, value)
    let mut non_private: Vec<(String, Value)> = Vec::new();
    let mut private: Vec<(String, Value)> = Vec::new();

    // Single pass through all keys using into_iter()
    for (key, value) in obj {
        declare_field_order!(key, value, known, non_private, private; [
            // Core Package Metadata
            0 => "$schema",
            1 => "name",
            2 => "displayName",
            3 => "version",
            4 => "stableVersion",
            5 => "private",
            6 => "description",
            7 => "categories" => transform_array(value, sort_array_unique),
            8 => "keywords" => transform_array(value, sort_array_unique),
            9 => "homepage",
            10 => "bugs" => transform_with_key_order(value, &["url", "email"]),
            // License & People
            11 => "license",
            12 => "author" => transform_value(value, sort_people_object),
            13 => "maintainers" => transform_people_array(value),
            14 => "contributors" => transform_people_array(value),
            // Repository & Funding
            15 => "repository" => transform_with_key_order(value, &["type", "url"]),
            16 => "funding" => transform_with_key_order(value, &["type", "url"]),
            17 => "donate" => transform_with_key_order(value, &["type", "url"]),
            18 => "sponsor" => transform_with_key_order(value, &["type", "url"]),
            19 => "qna",
            20 => "publisher",
            // Package Content & Distribution
            21 => "man",
            22 => "style",
            23 => "example",
            24 => "examplestyle",
            25 => "assets",
            26 => "directories" => transform_with_key_order(value, &["lib", "bin", "man", "doc", "example", "test"]),
            27 => "workspaces",
            28 => "binary" => transform_with_key_order(value, &["module_name", "module_path", "remote_path", "package_name", "host"]),
            29 => "files" => transform_array(value, sort_array_unique),
            30 => "os",
            31 => "cpu",
            32 => "libc" => transform_array(value, sort_array_unique),
            // Package Entry Points
            33 => "type",
            34 => "main",
            35 => "browser",
            36 => "bin" => transform_value(value, sort_object_alphabetically),
            37 => "umd",
            38 => "types",
            39 => "typings",
            40 => "typesVersions",
            41 => "react-native",
            42 => "svelte",
            43 => "source",
            44 => "jsnext:main",
            45 => "umd:main",
            46 => "jsdelivr",
            47 => "unpkg",
            48 => "module",
            49 => "esnext",
            50 => "es2020",
            51 => "esm2020",
            52 => "fesm2020",
            53 => "es2015",
            54 => "esm2015",
            55 => "fesm2015",
            56 => "es5",
            57 => "esm5",
            58 => "fesm5",
            59 => "sideEffects",
            60 => "imports",
            61 => "exports" => transform_value(value, sort_exports),
            62 => "publishConfig" => transform_value(value, sort_object_alphabetically),
            // Scripts
            63 => "scripts",
            64 => "betterScripts",
            // Dependencies
            65 => "dependencies" => transform_value(value, sort_object_alphabetically),
            66 => "devDependencies" => transform_value(value, sort_object_alphabetically),
            67 => "dependenciesMeta",
            68 => "peerDependencies" => transform_value(value, sort_object_alphabetically),
            69 => "peerDependenciesMeta",
            70 => "optionalDependencies" => transform_value(value, sort_object_alphabetically),
            71 => "bundledDependencies" => transform_array(value, sort_array_unique),
            72 => "bundleDependencies" => transform_array(value, sort_array_unique),
            73 => "resolutions" => transform_value(value, sort_object_alphabetically),
            74 => "overrides" => transform_value(value, sort_object_alphabetically),
            // Git Hooks & Commit Tools
            75 => "husky" => transform_value(value, sort_object_recursive),
            76 => "simple-git-hooks",
            77 => "pre-commit",
            78 => "lint-staged",
            79 => "nano-staged",
            80 => "commitlint" => transform_value(value, sort_object_recursive),
            // VSCode Extension Specific
            81 => "l10n",
            82 => "contributes",
            83 => "activationEvents" => transform_array(value, sort_array_unique),
            84 => "extensionPack" => transform_array(value, sort_array_unique),
            85 => "extensionDependencies" => transform_array(value, sort_array_unique),
            86 => "extensionKind" => transform_array(value, sort_array_unique),
            87 => "icon",
            88 => "badges",
            89 => "galleryBanner",
            90 => "preview",
            91 => "markdown",
            // Build & Tool Configuration
            92 => "napi" => transform_value(value, sort_object_alphabetically),
            93 => "flat",
            94 => "config" => transform_value(value, sort_object_alphabetically),
            95 => "nodemonConfig" => transform_value(value, sort_object_recursive),
            96 => "browserify" => transform_value(value, sort_object_recursive),
            97 => "babel" => transform_value(value, sort_object_recursive),
            98 => "browserslist",
            99 => "xo" => transform_value(value, sort_object_recursive),
            100 => "prettier" => transform_value(value, sort_object_recursive),
            101 => "eslintConfig" => transform_value(value, sort_object_recursive),
            102 => "eslintIgnore",
            103 => "npmpkgjsonlint",
            104 => "npmPackageJsonLintConfig",
            105 => "npmpackagejsonlint",
            106 => "release",
            107 => "remarkConfig" => transform_value(value, sort_object_recursive),
            108 => "stylelint" => transform_value(value, sort_object_recursive),
            // Testing
            109 => "ava" => transform_value(value, sort_object_recursive),
            110 => "jest" => transform_value(value, sort_object_recursive),
            111 => "jest-junit",
            112 => "jest-stare",
            113 => "mocha" => transform_value(value, sort_object_recursive),
            114 => "nyc" => transform_value(value, sort_object_recursive),
            115 => "c8" => transform_value(value, sort_object_recursive),
            116 => "tap",
            117 => "oclif" => transform_value(value, sort_object_recursive),
            // Runtime & Package Manager
            118 => "languageName",
            119 => "preferGlobal",
            120 => "devEngines" => transform_value(value, sort_object_alphabetically),
            121 => "engines" => transform_value(value, sort_object_alphabetically),
            122 => "engineStrict",
            123 => "volta" => transform_value(value, sort_object_recursive),
            124 => "packageManager",
            125 => "pnpm",
        ]);
    }

    // Sort each category (using unstable sort for better performance)
    known.sort_unstable_by_key(|(index, _, _)| *index);
    non_private.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));
    private.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));

    // Build result map
    let mut result = Map::new();

    // Insert known fields (already transformed)
    for (_index, key, value) in known {
        result.insert(key, value);
    }

    // Insert non-private unknown fields
    for (key, value) in non_private {
        result.insert(key, value);
    }

    // Insert private fields
    for (key, value) in private {
        result.insert(key, value);
    }

    result
}
