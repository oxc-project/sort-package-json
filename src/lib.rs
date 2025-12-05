use serde_json::{Map, Value};

pub fn sort_package_json(input: &str) -> Result<String, serde_json::Error> {
    let value: Value = serde_json::from_str(input)?;

    let sorted_value =
        if let Value::Object(obj) = value { Value::Object(sort_object_keys(obj)) } else { value };

    let mut result = serde_json::to_string_pretty(&sorted_value)?;
    result.push('\n');
    Ok(result)
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
            "categories" => {
                let transformed = match &value {
                    Value::Array(arr) => Value::Array(sort_array_unique(arr)),
                    _ => value,
                };
                known.push((7, key, transformed));
            }
            "keywords" => {
                let transformed = match &value {
                    Value::Array(arr) => Value::Array(sort_array_unique(arr)),
                    _ => value,
                };
                known.push((8, key, transformed));
            }
            "homepage" => known.push((9, key, value)),
            "bugs" => {
                let transformed = match &value {
                    Value::Object(o) => {
                        Value::Object(sort_object_by_key_order(o, &["url", "email"]))
                    }
                    _ => value,
                };
                known.push((10, key, transformed));
            }
            "repository" => {
                let transformed = match &value {
                    Value::Object(o) => {
                        Value::Object(sort_object_by_key_order(o, &["type", "url"]))
                    }
                    _ => value,
                };
                known.push((11, key, transformed));
            }
            "funding" => {
                let transformed = match &value {
                    Value::Object(o) => {
                        Value::Object(sort_object_by_key_order(o, &["type", "url"]))
                    }
                    _ => value,
                };
                known.push((12, key, transformed));
            }
            "license" => known.push((13, key, value)),
            "qna" => known.push((14, key, value)),
            "author" => {
                let transformed = match &value {
                    Value::Object(o) => Value::Object(sort_people_object(o)),
                    _ => value,
                };
                known.push((15, key, transformed));
            }
            "maintainers" => {
                let transformed = match &value {
                    Value::Array(arr) => {
                        let people: Vec<Value> = arr
                            .iter()
                            .map(|v| match v {
                                Value::Object(o) => Value::Object(sort_people_object(o)),
                                _ => v.clone(),
                            })
                            .collect();
                        Value::Array(people)
                    }
                    _ => value,
                };
                known.push((16, key, transformed));
            }
            "contributors" => {
                let transformed = match &value {
                    Value::Array(arr) => {
                        let people: Vec<Value> = arr
                            .iter()
                            .map(|v| match v {
                                Value::Object(o) => Value::Object(sort_people_object(o)),
                                _ => v.clone(),
                            })
                            .collect();
                        Value::Array(people)
                    }
                    _ => value,
                };
                known.push((17, key, transformed));
            }
            "publisher" => known.push((18, key, value)),
            "sideEffects" => known.push((19, key, value)),
            "type" => known.push((20, key, value)),
            "imports" => known.push((21, key, value)),
            "exports" => {
                let transformed = match &value {
                    Value::Object(o) => Value::Object(sort_exports(o)),
                    _ => value,
                };
                known.push((22, key, transformed));
            }
            "main" => known.push((23, key, value)),
            "svelte" => known.push((24, key, value)),
            "umd:main" => known.push((25, key, value)),
            "jsdelivr" => known.push((26, key, value)),
            "unpkg" => known.push((27, key, value)),
            "module" => known.push((28, key, value)),
            "source" => known.push((29, key, value)),
            "jsnext:main" => known.push((30, key, value)),
            "browser" => known.push((31, key, value)),
            "react-native" => known.push((32, key, value)),
            "types" => known.push((33, key, value)),
            "typesVersions" => known.push((34, key, value)),
            "typings" => known.push((35, key, value)),
            "style" => known.push((36, key, value)),
            "example" => known.push((37, key, value)),
            "examplestyle" => known.push((38, key, value)),
            "assets" => known.push((39, key, value)),
            "bin" => {
                let transformed = match &value {
                    Value::Object(o) => Value::Object(sort_object_alphabetically(o)),
                    _ => value,
                };
                known.push((40, key, transformed));
            }
            "man" => known.push((41, key, value)),
            "directories" => {
                let transformed = match &value {
                    Value::Object(o) => Value::Object(sort_object_by_key_order(
                        o,
                        &["lib", "bin", "man", "doc", "example", "test"],
                    )),
                    _ => value,
                };
                known.push((42, key, transformed));
            }
            "files" => {
                let transformed = match &value {
                    Value::Array(arr) => Value::Array(sort_array_unique(arr)),
                    _ => value,
                };
                known.push((43, key, transformed));
            }
            "workspaces" => known.push((44, key, value)),
            "binary" => {
                let transformed = match &value {
                    Value::Object(o) => Value::Object(sort_object_by_key_order(
                        o,
                        &["module_name", "module_path", "remote_path", "package_name", "host"],
                    )),
                    _ => value,
                };
                known.push((45, key, transformed));
            }
            "scripts" => known.push((46, key, value)),
            "betterScripts" => known.push((47, key, value)),
            "l10n" => known.push((48, key, value)),
            "contributes" => known.push((49, key, value)),
            "activationEvents" => {
                let transformed = match &value {
                    Value::Array(arr) => Value::Array(sort_array_unique(arr)),
                    _ => value,
                };
                known.push((50, key, transformed));
            }
            "husky" => {
                let transformed = match &value {
                    Value::Object(o) => Value::Object(sort_object_recursive(o)),
                    _ => value,
                };
                known.push((51, key, transformed));
            }
            "simple-git-hooks" => known.push((52, key, value)),
            "pre-commit" => known.push((53, key, value)),
            "commitlint" => {
                let transformed = match &value {
                    Value::Object(o) => Value::Object(sort_object_recursive(o)),
                    _ => value,
                };
                known.push((54, key, transformed));
            }
            "lint-staged" => known.push((55, key, value)),
            "nano-staged" => known.push((56, key, value)),
            "config" => {
                let transformed = match &value {
                    Value::Object(o) => Value::Object(sort_object_alphabetically(o)),
                    _ => value,
                };
                known.push((57, key, transformed));
            }
            "nodemonConfig" => {
                let transformed = match &value {
                    Value::Object(o) => Value::Object(sort_object_recursive(o)),
                    _ => value,
                };
                known.push((58, key, transformed));
            }
            "browserify" => {
                let transformed = match &value {
                    Value::Object(o) => Value::Object(sort_object_recursive(o)),
                    _ => value,
                };
                known.push((59, key, transformed));
            }
            "babel" => {
                let transformed = match &value {
                    Value::Object(o) => Value::Object(sort_object_recursive(o)),
                    _ => value,
                };
                known.push((60, key, transformed));
            }
            "browserslist" => known.push((61, key, value)),
            "xo" => {
                let transformed = match &value {
                    Value::Object(o) => Value::Object(sort_object_recursive(o)),
                    _ => value,
                };
                known.push((62, key, transformed));
            }
            "prettier" => {
                let transformed = match &value {
                    Value::Object(o) => Value::Object(sort_object_recursive(o)),
                    _ => value,
                };
                known.push((63, key, transformed));
            }
            "eslintConfig" => {
                let transformed = match &value {
                    Value::Object(o) => Value::Object(sort_object_recursive(o)),
                    _ => value,
                };
                known.push((64, key, transformed));
            }
            "eslintIgnore" => known.push((65, key, value)),
            "npmpkgjsonlint" => known.push((66, key, value)),
            "npmPackageJsonLintConfig" => known.push((67, key, value)),
            "npmpackagejsonlint" => known.push((68, key, value)),
            "release" => known.push((69, key, value)),
            "remarkConfig" => {
                let transformed = match &value {
                    Value::Object(o) => Value::Object(sort_object_recursive(o)),
                    _ => value,
                };
                known.push((70, key, transformed));
            }
            "stylelint" => {
                let transformed = match &value {
                    Value::Object(o) => Value::Object(sort_object_recursive(o)),
                    _ => value,
                };
                known.push((71, key, transformed));
            }
            "ava" => {
                let transformed = match &value {
                    Value::Object(o) => Value::Object(sort_object_recursive(o)),
                    _ => value,
                };
                known.push((72, key, transformed));
            }
            "jest" => {
                let transformed = match &value {
                    Value::Object(o) => Value::Object(sort_object_recursive(o)),
                    _ => value,
                };
                known.push((73, key, transformed));
            }
            "jest-junit" => known.push((74, key, value)),
            "jest-stare" => known.push((75, key, value)),
            "mocha" => {
                let transformed = match &value {
                    Value::Object(o) => Value::Object(sort_object_recursive(o)),
                    _ => value,
                };
                known.push((76, key, transformed));
            }
            "nyc" => {
                let transformed = match &value {
                    Value::Object(o) => Value::Object(sort_object_recursive(o)),
                    _ => value,
                };
                known.push((77, key, transformed));
            }
            "c8" => {
                let transformed = match &value {
                    Value::Object(o) => Value::Object(sort_object_recursive(o)),
                    _ => value,
                };
                known.push((78, key, transformed));
            }
            "tap" => known.push((79, key, value)),
            "oclif" => {
                let transformed = match &value {
                    Value::Object(o) => Value::Object(sort_object_recursive(o)),
                    _ => value,
                };
                known.push((80, key, transformed));
            }
            "resolutions" => {
                let transformed = match &value {
                    Value::Object(o) => Value::Object(sort_object_alphabetically(o)),
                    _ => value,
                };
                known.push((81, key, transformed));
            }
            "overrides" => {
                let transformed = match &value {
                    Value::Object(o) => Value::Object(sort_object_alphabetically(o)),
                    _ => value,
                };
                known.push((82, key, transformed));
            }
            "dependencies" => {
                let transformed = match &value {
                    Value::Object(o) => Value::Object(sort_object_alphabetically(o)),
                    _ => value,
                };
                known.push((83, key, transformed));
            }
            "devDependencies" => {
                let transformed = match &value {
                    Value::Object(o) => Value::Object(sort_object_alphabetically(o)),
                    _ => value,
                };
                known.push((84, key, transformed));
            }
            "dependenciesMeta" => known.push((85, key, value)),
            "peerDependencies" => {
                let transformed = match &value {
                    Value::Object(o) => Value::Object(sort_object_alphabetically(o)),
                    _ => value,
                };
                known.push((86, key, transformed));
            }
            "peerDependenciesMeta" => known.push((87, key, value)),
            "optionalDependencies" => {
                let transformed = match &value {
                    Value::Object(o) => Value::Object(sort_object_alphabetically(o)),
                    _ => value,
                };
                known.push((88, key, transformed));
            }
            "bundledDependencies" => {
                let transformed = match &value {
                    Value::Array(arr) => Value::Array(sort_array_unique(arr)),
                    _ => value,
                };
                known.push((89, key, transformed));
            }
            "bundleDependencies" => {
                let transformed = match &value {
                    Value::Array(arr) => Value::Array(sort_array_unique(arr)),
                    _ => value,
                };
                known.push((90, key, transformed));
            }
            "extensionPack" => {
                let transformed = match &value {
                    Value::Array(arr) => Value::Array(sort_array_unique(arr)),
                    _ => value,
                };
                known.push((91, key, transformed));
            }
            "extensionDependencies" => {
                let transformed = match &value {
                    Value::Array(arr) => Value::Array(sort_array_unique(arr)),
                    _ => value,
                };
                known.push((92, key, transformed));
            }
            "flat" => known.push((93, key, value)),
            "packageManager" => known.push((94, key, value)),
            "engines" => {
                let transformed = match &value {
                    Value::Object(o) => Value::Object(sort_object_alphabetically(o)),
                    _ => value,
                };
                known.push((95, key, transformed));
            }
            "engineStrict" => known.push((96, key, value)),
            "volta" => {
                let transformed = match &value {
                    Value::Object(o) => Value::Object(sort_object_recursive(o)),
                    _ => value,
                };
                known.push((97, key, transformed));
            }
            "languageName" => known.push((98, key, value)),
            "os" => known.push((99, key, value)),
            "cpu" => known.push((100, key, value)),
            "preferGlobal" => known.push((101, key, value)),
            "publishConfig" => {
                let transformed = match &value {
                    Value::Object(o) => Value::Object(sort_object_alphabetically(o)),
                    _ => value,
                };
                known.push((102, key, transformed));
            }
            "icon" => known.push((103, key, value)),
            "badges" => known.push((104, key, value)),
            "galleryBanner" => known.push((105, key, value)),
            "preview" => known.push((106, key, value)),
            "markdown" => known.push((107, key, value)),
            "pnpm" => known.push((108, key, value)),
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
