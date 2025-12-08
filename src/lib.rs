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
        match key.as_str() {
            "$schema" => known.push((0, key, value)),
            "name" => known.push((1, key, value)),
            "displayName" => known.push((2, key, value)),
            "version" => known.push((3, key, value)),
            "stableVersion" => known.push((4, key, value)),
            "private" => known.push((5, key, value)),
            "description" => known.push((6, key, value)),
            "categories" => known.push((7, key, transform_array(&value, sort_array_unique))),
            "keywords" => known.push((8, key, transform_array(&value, sort_array_unique))),
            "homepage" => known.push((9, key, value)),
            "bugs" => known.push((10, key, transform_with_key_order(&value, &["url", "email"]))),
            "repository" => {
                known.push((11, key, transform_with_key_order(&value, &["type", "url"])))
            }
            "author" => known.push((12, key, transform_value(&value, sort_people_object))),
            "maintainers" => known.push((13, key, transform_people_array(&value))),
            "contributors" => known.push((14, key, transform_people_array(&value))),
            "donate" => known.push((15, key, transform_with_key_order(&value, &["type", "url"]))),
            "funding" => known.push((16, key, transform_with_key_order(&value, &["type", "url"]))),
            "sponsor" => known.push((17, key, transform_with_key_order(&value, &["type", "url"]))),
            "license" => known.push((18, key, value)),
            "qna" => known.push((19, key, value)),
            "publisher" => known.push((20, key, value)),
            "sideEffects" => known.push((21, key, value)),
            "type" => known.push((22, key, value)),
            "imports" => known.push((23, key, value)),
            "exports" => known.push((24, key, transform_value(&value, sort_exports))),
            "main" => known.push((25, key, value)),
            "svelte" => known.push((26, key, value)),
            "umd:main" => known.push((27, key, value)),
            "jsdelivr" => known.push((28, key, value)),
            "unpkg" => known.push((29, key, value)),
            "module" => known.push((30, key, value)),
            "esnext" => known.push((31, key, value)),
            "es2020" => known.push((32, key, value)),
            "esm2020" => known.push((33, key, value)),
            "fesm2020" => known.push((34, key, value)),
            "es2015" => known.push((35, key, value)),
            "esm2015" => known.push((36, key, value)),
            "fesm2015" => known.push((37, key, value)),
            "es5" => known.push((38, key, value)),
            "esm5" => known.push((39, key, value)),
            "fesm5" => known.push((40, key, value)),
            "source" => known.push((41, key, value)),
            "jsnext:main" => known.push((42, key, value)),
            "browser" => known.push((43, key, value)),
            "umd" => known.push((44, key, value)),
            "react-native" => known.push((45, key, value)),
            "types" => known.push((46, key, value)),
            "typesVersions" => known.push((47, key, value)),
            "typings" => known.push((48, key, value)),
            "style" => known.push((49, key, value)),
            "example" => known.push((50, key, value)),
            "examplestyle" => known.push((51, key, value)),
            "assets" => known.push((52, key, value)),
            "bin" => known.push((53, key, transform_value(&value, sort_object_alphabetically))),
            "man" => known.push((54, key, value)),
            "directories" => {
                known.push((
                    55,
                    key,
                    transform_with_key_order(
                        &value,
                        &["lib", "bin", "man", "doc", "example", "test"],
                    ),
                ));
            }
            "files" => known.push((56, key, transform_array(&value, sort_array_unique))),
            "workspaces" => known.push((57, key, value)),
            "binary" => {
                known.push((
                    58,
                    key,
                    transform_with_key_order(
                        &value,
                        &["module_name", "module_path", "remote_path", "package_name", "host"],
                    ),
                ));
            }
            "scripts" => known.push((59, key, value)),
            "betterScripts" => known.push((60, key, value)),
            "l10n" => known.push((61, key, value)),
            "contributes" => known.push((62, key, value)),
            "activationEvents" => known.push((63, key, transform_array(&value, sort_array_unique))),
            "husky" => known.push((64, key, transform_value(&value, sort_object_recursive))),
            "simple-git-hooks" => known.push((65, key, value)),
            "pre-commit" => known.push((66, key, value)),
            "commitlint" => known.push((67, key, transform_value(&value, sort_object_recursive))),
            "lint-staged" => known.push((68, key, value)),
            "nano-staged" => known.push((69, key, value)),
            "resolutions" => {
                known.push((70, key, transform_value(&value, sort_object_alphabetically)))
            }
            "overrides" => {
                known.push((71, key, transform_value(&value, sort_object_alphabetically)))
            }
            "dependencies" => {
                known.push((72, key, transform_value(&value, sort_object_alphabetically)))
            }
            "devDependencies" => {
                known.push((73, key, transform_value(&value, sort_object_alphabetically)))
            }
            "dependenciesMeta" => known.push((74, key, value)),
            "peerDependencies" => {
                known.push((75, key, transform_value(&value, sort_object_alphabetically)))
            }
            "peerDependenciesMeta" => known.push((76, key, value)),
            "optionalDependencies" => {
                known.push((77, key, transform_value(&value, sort_object_alphabetically)))
            }
            "bundledDependencies" => {
                known.push((78, key, transform_array(&value, sort_array_unique)))
            }
            "bundleDependencies" => {
                known.push((79, key, transform_array(&value, sort_array_unique)))
            }
            "napi" => {
                known.push((80, key, transform_value(&value, sort_object_alphabetically)))
            }
            "extensionPack" => known.push((81, key, transform_array(&value, sort_array_unique))),
            "extensionDependencies" => {
                known.push((82, key, transform_array(&value, sort_array_unique)))
            }
            "extensionKind" => known.push((83, key, transform_array(&value, sort_array_unique))),
            "flat" => known.push((84, key, value)),
            "packageManager" => known.push((85, key, value)),
            "config" => known.push((86, key, transform_value(&value, sort_object_alphabetically))),
            "nodemonConfig" => {
                known.push((87, key, transform_value(&value, sort_object_recursive)))
            }
            "browserify" => known.push((88, key, transform_value(&value, sort_object_recursive))),
            "babel" => known.push((89, key, transform_value(&value, sort_object_recursive))),
            "browserslist" => known.push((90, key, value)),
            "xo" => known.push((91, key, transform_value(&value, sort_object_recursive))),
            "prettier" => known.push((92, key, transform_value(&value, sort_object_recursive))),
            "eslintConfig" => known.push((93, key, transform_value(&value, sort_object_recursive))),
            "eslintIgnore" => known.push((94, key, value)),
            "npmpkgjsonlint" => known.push((95, key, value)),
            "npmPackageJsonLintConfig" => known.push((96, key, value)),
            "npmpackagejsonlint" => known.push((97, key, value)),
            "release" => known.push((98, key, value)),
            "remarkConfig" => known.push((99, key, transform_value(&value, sort_object_recursive))),
            "stylelint" => known.push((100, key, transform_value(&value, sort_object_recursive))),
            "ava" => known.push((101, key, transform_value(&value, sort_object_recursive))),
            "jest" => known.push((102, key, transform_value(&value, sort_object_recursive))),
            "jest-junit" => known.push((103, key, value)),
            "jest-stare" => known.push((104, key, value)),
            "mocha" => known.push((105, key, transform_value(&value, sort_object_recursive))),
            "nyc" => known.push((106, key, transform_value(&value, sort_object_recursive))),
            "c8" => known.push((107, key, transform_value(&value, sort_object_recursive))),
            "tap" => known.push((108, key, value)),
            "oclif" => known.push((109, key, transform_value(&value, sort_object_recursive))),
            "engines" => {
                known.push((110, key, transform_value(&value, sort_object_alphabetically)))
            }
            "engineStrict" => known.push((111, key, value)),
            "volta" => known.push((112, key, transform_value(&value, sort_object_recursive))),
            "languageName" => known.push((113, key, value)),
            "os" => known.push((114, key, value)),
            "cpu" => known.push((115, key, value)),
            "libc" => known.push((116, key, transform_array(&value, sort_array_unique))),
            "devEngines" => {
                known.push((117, key, transform_value(&value, sort_object_alphabetically)))
            }
            "preferGlobal" => known.push((118, key, value)),
            "publishConfig" => {
                known.push((119, key, transform_value(&value, sort_object_alphabetically)))
            }
            "icon" => known.push((120, key, value)),
            "badges" => known.push((121, key, value)),
            "galleryBanner" => known.push((122, key, value)),
            "preview" => known.push((123, key, value)),
            "markdown" => known.push((124, key, value)),
            "pnpm" => known.push((125, key, value)),
            _ => {
                // Unknown field - check if private
                if key.starts_with('_') {
                    private.push((key, value));
                } else {
                    non_private.push((key, value));
                }
            }
        }
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
