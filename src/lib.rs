use serde_json::{Map, Value};

const FIELDS_ORDER: &[&str] = &[
    "$schema",
    "name",
    "displayName",
    "version",
    "stableVersion",
    "private",
    "description",
    "categories",
    "keywords",
    "homepage",
    "bugs",
    "repository",
    "funding",
    "license",
    "qna",
    "author",
    "maintainers",
    "contributors",
    "publisher",
    "sideEffects",
    "type",
    "imports",
    "exports",
    "main",
    "svelte",
    "umd:main",
    "jsdelivr",
    "unpkg",
    "module",
    "source",
    "jsnext:main",
    "browser",
    "react-native",
    "types",
    "typesVersions",
    "typings",
    "style",
    "example",
    "examplestyle",
    "assets",
    "bin",
    "man",
    "directories",
    "files",
    "workspaces",
    "binary",
    "scripts",
    "betterScripts",
    "l10n",
    "contributes",
    "activationEvents",
    "husky",
    "simple-git-hooks",
    "pre-commit",
    "commitlint",
    "lint-staged",
    "nano-staged",
    "config",
    "nodemonConfig",
    "browserify",
    "babel",
    "browserslist",
    "xo",
    "prettier",
    "eslintConfig",
    "eslintIgnore",
    "npmpkgjsonlint",
    "npmPackageJsonLintConfig",
    "npmpackagejsonlint",
    "release",
    "remarkConfig",
    "stylelint",
    "ava",
    "jest",
    "jest-junit",
    "jest-stare",
    "mocha",
    "nyc",
    "c8",
    "tap",
    "oclif",
    "resolutions",
    "overrides",
    "dependencies",
    "devDependencies",
    "dependenciesMeta",
    "peerDependencies",
    "peerDependenciesMeta",
    "optionalDependencies",
    "bundledDependencies",
    "bundleDependencies",
    "extensionPack",
    "extensionDependencies",
    "flat",
    "packageManager",
    "engines",
    "engineStrict",
    "volta",
    "languageName",
    "os",
    "cpu",
    "preferGlobal",
    "publishConfig",
    "icon",
    "badges",
    "galleryBanner",
    "preview",
    "markdown",
    "pnpm",
];

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

fn sort_scripts(obj: &Map<String, Value>) -> Map<String, Value> {
    // Simple alphabetical sorting for now
    // TODO: Implement pre/post grouping and npm-run-all detection
    sort_object_alphabetically(obj)
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
    let mut result = Map::new();
    let mut remaining_keys: Vec<String> = obj.keys().cloned().collect();

    // Process fields in FIELDS_ORDER with inline transformations
    for &field in FIELDS_ORDER {
        if let Some(value) = obj.get(field) {
            let transformed = match (field, value) {
                // Dependency-like fields - alphabetically sorted
                (
                    "dependencies"
                    | "devDependencies"
                    | "peerDependencies"
                    | "optionalDependencies"
                    | "resolutions"
                    | "overrides"
                    | "engines"
                    | "publishConfig"
                    | "config"
                    | "bin",
                    Value::Object(obj),
                ) => Value::Object(sort_object_alphabetically(obj)),

                // Config objects - recursively sorted
                (
                    "babel" | "jest" | "mocha" | "nyc" | "c8" | "ava" | "eslintConfig" | "prettier"
                    | "stylelint" | "nodemonConfig" | "browserify" | "xo" | "husky" | "commitlint"
                    | "remarkConfig" | "volta" | "oclif",
                    Value::Object(obj),
                ) => Value::Object(sort_object_recursive(obj)),

                // Array fields - deduplicate and sort
                (
                    "keywords"
                    | "files"
                    | "bundledDependencies"
                    | "bundleDependencies"
                    | "categories"
                    | "activationEvents"
                    | "extensionPack"
                    | "extensionDependencies",
                    Value::Array(arr),
                ) => Value::Array(sort_array_unique(arr)),

                // Scripts sorting
                ("scripts" | "betterScripts", Value::Object(obj)) => {
                    Value::Object(sort_scripts(obj))
                }

                // Exports sorting
                ("exports", Value::Object(obj)) => Value::Object(sort_exports(obj)),

                // Objects with specific key ordering
                ("bugs", Value::Object(obj)) => {
                    Value::Object(sort_object_by_key_order(obj, &["url", "email"]))
                }
                ("repository" | "funding", Value::Object(obj)) => {
                    Value::Object(sort_object_by_key_order(obj, &["type", "url"]))
                }
                ("author", Value::Object(obj)) => Value::Object(sort_people_object(obj)),
                ("directories", Value::Object(obj)) => Value::Object(sort_object_by_key_order(
                    obj,
                    &["lib", "bin", "man", "doc", "example", "test"],
                )),
                ("binary", Value::Object(obj)) => Value::Object(sort_object_by_key_order(
                    obj,
                    &["module_name", "module_path", "remote_path", "package_name", "host"],
                )),

                // People arrays
                ("maintainers" | "contributors", Value::Array(arr)) => {
                    let people: Vec<Value> = arr
                        .iter()
                        .map(|v| match v {
                            Value::Object(obj) => Value::Object(sort_people_object(obj)),
                            _ => v.clone(),
                        })
                        .collect();
                    Value::Array(people)
                }

                // No transformation needed
                _ => value.clone(),
            };

            result.insert(field.to_string(), transformed);
            remaining_keys.retain(|k| k != field);
        }
    }

    // Separate remaining keys into non-private and private
    let mut non_private: Vec<String> = Vec::new();
    let mut private: Vec<String> = Vec::new();

    for key in remaining_keys {
        if key.starts_with('_') {
            private.push(key);
        } else {
            non_private.push(key);
        }
    }

    // Sort non-private keys alphabetically
    non_private.sort();
    for key in non_private {
        if let Some(value) = obj.get(&key) {
            result.insert(key, value.clone());
        }
    }

    // Sort private keys alphabetically
    private.sort();
    for key in private {
        if let Some(value) = obj.get(&key) {
            result.insert(key, value.clone());
        }
    }

    result
}
