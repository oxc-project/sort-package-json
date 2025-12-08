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

fn transform_value<F>(value: &Value, transform: F) -> Value
where
    F: FnOnce(&Map<String, Value>) -> Map<String, Value>,
{
    match value {
        Value::Object(o) => Value::Object(transform(o)),
        _ => value.clone(),
    }
}

fn transform_array<F>(value: &Value, transform: F) -> Value
where
    F: FnOnce(&[Value]) -> Vec<Value>,
{
    match value {
        Value::Array(arr) => Value::Array(transform(arr)),
        _ => value.clone(),
    }
}

fn transform_with_key_order(value: &Value, key_order: &[&str]) -> Value {
    transform_value(value, |o| sort_object_by_key_order(o, key_order))
}

fn transform_people_array(value: &Value) -> Value {
    transform_array(value, |arr| {
        arr.iter()
            .map(|v| match v {
                Value::Object(o) => Value::Object(sort_people_object(o)),
                _ => v.clone(),
            })
            .collect()
    })
}

fn sort_object_alphabetically(obj: &Map<String, Value>) -> Map<String, Value> {
    let mut keys: Vec<&String> = obj.keys().collect();
    keys.sort();

    let mut result = Map::new();
    for key in keys {
        if let Some(value) = obj.get(key) {
            result.insert(key.clone(), value.clone());
        }
    }
    result
}

fn sort_object_recursive(obj: &Map<String, Value>) -> Map<String, Value> {
    let mut keys: Vec<&String> = obj.keys().collect();
    keys.sort();

    let mut result = Map::new();
    for key in keys {
        if let Some(value) = obj.get(key) {
            let transformed_value = match value {
                Value::Object(nested) => Value::Object(sort_object_recursive(nested)),
                _ => value.clone(),
            };
            result.insert(key.clone(), transformed_value);
        }
    }
    result
}

fn sort_array_unique(arr: &[Value]) -> Vec<Value> {
    let mut strings: Vec<String> =
        arr.iter().filter_map(|v| v.as_str().map(String::from)).collect();

    strings.sort();
    strings.dedup();

    strings.into_iter().map(Value::String).collect()
}

fn sort_object_by_key_order(obj: &Map<String, Value>, key_order: &[&str]) -> Map<String, Value> {
    let mut result = Map::new();

    // Add keys in specified order
    for &key in key_order {
        if let Some(value) = obj.get(key) {
            result.insert(key.to_string(), value.clone());
        }
    }

    // Add remaining keys alphabetically
    let mut remaining: Vec<&String> =
        obj.keys().filter(|k| !key_order.contains(&k.as_str())).collect();
    remaining.sort();

    for key in remaining {
        if let Some(value) = obj.get(key) {
            result.insert(key.clone(), value.clone());
        }
    }

    result
}

fn sort_people_object(obj: &Map<String, Value>) -> Map<String, Value> {
    sort_object_by_key_order(obj, &["name", "email", "url"])
}

fn sort_exports(obj: &Map<String, Value>) -> Map<String, Value> {
    let mut paths = Vec::new();
    let mut types_conds = Vec::new();
    let mut other_conds = Vec::new();
    let mut default_cond = None;

    for (key, value) in obj.iter() {
        if key.starts_with('.') {
            paths.push(key);
        } else if key == "default" {
            default_cond = Some((key, value));
        } else if key == "types" || key.starts_with("types@") {
            types_conds.push(key);
        } else {
            other_conds.push(key);
        }
    }

    // Sort each category
    paths.sort();
    types_conds.sort();
    other_conds.sort();

    let mut result = Map::new();

    // Add in order: paths, types, others, default
    for key in paths {
        if let Some(value) = obj.get(key) {
            let transformed = match value {
                Value::Object(nested) => Value::Object(sort_exports(nested)),
                _ => value.clone(),
            };
            result.insert(key.clone(), transformed);
        }
    }

    for key in types_conds {
        if let Some(value) = obj.get(key) {
            result.insert(key.clone(), value.clone());
        }
    }

    for key in other_conds {
        if let Some(value) = obj.get(key) {
            result.insert(key.clone(), value.clone());
        }
    }

    if let Some((key, value)) = default_cond {
        result.insert(key.clone(), value.clone());
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
            0 => "$schema",
            1 => "name",
            2 => "displayName",
            3 => "version",
            4 => "stableVersion",
            5 => "private",
            6 => "description",
            7 => "categories" => transform_array(&value, sort_array_unique),
            8 => "keywords" => transform_array(&value, sort_array_unique),
            9 => "homepage",
            10 => "bugs" => transform_with_key_order(&value, &["url", "email"]),
            11 => "repository" => transform_with_key_order(&value, &["type", "url"]),
            12 => "author" => transform_value(&value, sort_people_object),
            13 => "maintainers" => transform_people_array(&value),
            14 => "contributors" => transform_people_array(&value),
            15 => "donate" => transform_with_key_order(&value, &["type", "url"]),
            16 => "funding" => transform_with_key_order(&value, &["type", "url"]),
            17 => "sponsor" => transform_with_key_order(&value, &["type", "url"]),
            18 => "license",
            19 => "qna",
            20 => "publisher",
            21 => "sideEffects",
            22 => "type",
            23 => "main",
            24 => "imports",
            25 => "exports" => transform_value(&value, sort_exports),
            26 => "svelte",
            27 => "umd:main",
            28 => "jsdelivr",
            29 => "unpkg",
            30 => "module",
            31 => "esnext",
            32 => "es2020",
            33 => "esm2020",
            34 => "fesm2020",
            35 => "es2015",
            36 => "esm2015",
            37 => "fesm2015",
            38 => "es5",
            39 => "esm5",
            40 => "fesm5",
            41 => "source",
            42 => "jsnext:main",
            43 => "browser",
            44 => "umd",
            45 => "react-native",
            46 => "types",
            47 => "typesVersions",
            48 => "typings",
            49 => "style",
            50 => "example",
            51 => "examplestyle",
            52 => "assets",
            53 => "bin" => transform_value(&value, sort_object_alphabetically),
            54 => "man",
            55 => "directories" => transform_with_key_order(
                &value,
                &["lib", "bin", "man", "doc", "example", "test"],
            ),
            56 => "files" => transform_array(&value, sort_array_unique),
            57 => "workspaces",
            58 => "binary" => transform_with_key_order(
                &value,
                &["module_name", "module_path", "remote_path", "package_name", "host"],
            ),
            59 => "scripts",
            60 => "betterScripts",
            61 => "l10n",
            62 => "contributes",
            63 => "activationEvents" => transform_array(&value, sort_array_unique),
            64 => "husky" => transform_value(&value, sort_object_recursive),
            65 => "simple-git-hooks",
            66 => "pre-commit",
            67 => "commitlint" => transform_value(&value, sort_object_recursive),
            68 => "lint-staged",
            69 => "nano-staged",
            70 => "resolutions" => transform_value(&value, sort_object_alphabetically),
            71 => "overrides" => transform_value(&value, sort_object_alphabetically),
            72 => "dependencies" => transform_value(&value, sort_object_alphabetically),
            73 => "devDependencies" => transform_value(&value, sort_object_alphabetically),
            74 => "dependenciesMeta",
            75 => "peerDependencies" => transform_value(&value, sort_object_alphabetically),
            76 => "peerDependenciesMeta",
            77 => "optionalDependencies" => transform_value(&value, sort_object_alphabetically),
            78 => "bundledDependencies" => transform_array(&value, sort_array_unique),
            79 => "bundleDependencies" => transform_array(&value, sort_array_unique),
            80 => "napi" => transform_value(&value, sort_object_alphabetically),
            81 => "extensionPack" => transform_array(&value, sort_array_unique),
            82 => "extensionDependencies" => transform_array(&value, sort_array_unique),
            83 => "extensionKind" => transform_array(&value, sort_array_unique),
            84 => "flat",
            85 => "packageManager",
            86 => "config" => transform_value(&value, sort_object_alphabetically),
            87 => "nodemonConfig" => transform_value(&value, sort_object_recursive),
            88 => "browserify" => transform_value(&value, sort_object_recursive),
            89 => "babel" => transform_value(&value, sort_object_recursive),
            90 => "browserslist",
            91 => "xo" => transform_value(&value, sort_object_recursive),
            92 => "prettier" => transform_value(&value, sort_object_recursive),
            93 => "eslintConfig" => transform_value(&value, sort_object_recursive),
            94 => "eslintIgnore",
            95 => "npmpkgjsonlint",
            96 => "npmPackageJsonLintConfig",
            97 => "npmpackagejsonlint",
            98 => "release",
            99 => "remarkConfig" => transform_value(&value, sort_object_recursive),
            100 => "stylelint" => transform_value(&value, sort_object_recursive),
            101 => "ava" => transform_value(&value, sort_object_recursive),
            102 => "jest" => transform_value(&value, sort_object_recursive),
            103 => "jest-junit",
            104 => "jest-stare",
            105 => "mocha" => transform_value(&value, sort_object_recursive),
            106 => "nyc" => transform_value(&value, sort_object_recursive),
            107 => "c8" => transform_value(&value, sort_object_recursive),
            108 => "tap",
            109 => "oclif" => transform_value(&value, sort_object_recursive),
            110 => "engines" => transform_value(&value, sort_object_alphabetically),
            111 => "engineStrict",
            112 => "volta" => transform_value(&value, sort_object_recursive),
            113 => "languageName",
            114 => "os",
            115 => "cpu",
            116 => "libc" => transform_array(&value, sort_array_unique),
            117 => "devEngines" => transform_value(&value, sort_object_alphabetically),
            118 => "preferGlobal",
            119 => "publishConfig" => transform_value(&value, sort_object_alphabetically),
            120 => "icon",
            121 => "badges",
            122 => "galleryBanner",
            123 => "preview",
            124 => "markdown",
            125 => "pnpm",
        ]);
    }

    // Sort each category
    known.sort_by_key(|(index, _, _)| *index);
    non_private.sort_by(|(a, _), (b, _)| a.cmp(b));
    private.sort_by(|(a, _), (b, _)| a.cmp(b));

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
