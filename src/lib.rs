use serde_json::{Map, Value};

/// UTF-8 BOM (`U+FEFF`).
const BOM_STR: &str = "\u{FEFF}";

/// Options for controlling JSON formatting when sorting.
#[derive(Debug, Clone)]
pub struct SortOptions {
    /// Whether to pretty-print the output JSON.
    pub pretty: bool,
    /// Whether to sort the scripts field alphabetically.
    pub sort_scripts: bool,
}

impl Default for SortOptions {
    fn default() -> Self {
        Self { pretty: true, sort_scripts: false }
    }
}

/// Sorts a `package.json` string with custom options.
pub fn sort_package_json_with_options(
    input: &str,
    options: &SortOptions,
) -> Result<String, serde_json::Error> {
    let (has_bom, body) =
        input.strip_prefix(BOM_STR).map_or((false, input), |stripped| (true, stripped));

    let value: Value = serde_json::from_str(body)?;

    let sorted = match value {
        Value::Object(obj) => Value::Object(sort_object_keys(obj, options)),
        other => other,
    };

    // Serialize directly into a byte buffer so the (optional) BOM, the JSON body, and the
    // trailing newline are all written into a single allocation. This skips the extra
    // String allocation + copy that `to_string_pretty` followed by manual BOM-prepending
    // would incur.
    //
    // Sized for the common case where the input is already pretty-printed: output ≈ input
    // in length. The `+ 16` absorbs the trailing `'\n'` push and minor reformatting slop
    // without forcing a final realloc.
    let mut buf: Vec<u8> = Vec::with_capacity(input.len() + 16);
    if has_bom {
        buf.extend_from_slice(BOM_STR.as_bytes());
    }
    if options.pretty {
        serde_json::to_writer_pretty(&mut buf, &sorted)?;
        buf.push(b'\n');
    } else {
        serde_json::to_writer(&mut buf, &sorted)?;
    }
    // SAFETY: `serde_json::to_writer{,_pretty}` are contractually required to emit valid
    // UTF-8 (this is also what `serde_json::to_string_pretty` itself relies on). The BOM
    // bytes and the trailing `\n` are also valid UTF-8.
    Ok(unsafe { String::from_utf8_unchecked(buf) })
}

/// Sorts a `package.json` string with default options (pretty-printed).
pub fn sort_package_json(input: &str) -> Result<String, serde_json::Error> {
    sort_package_json_with_options(input, &SortOptions::default())
}

// ===== Value-level transformations ==========================================

#[inline]
fn transform_value<F>(value: Value, f: F) -> Value
where
    F: FnOnce(Map<String, Value>) -> Map<String, Value>,
{
    match value {
        Value::Object(o) => Value::Object(f(o)),
        other => other,
    }
}

#[inline]
fn transform_array<F>(value: Value, f: F) -> Value
where
    F: FnOnce(Vec<Value>) -> Vec<Value>,
{
    match value {
        Value::Array(arr) => Value::Array(f(arr)),
        other => other,
    }
}

#[inline]
fn transform_with_key_order(value: Value, key_order: &[&str]) -> Value {
    transform_value(value, |o| sort_object_by_key_order(o, key_order))
}

fn sort_object_alphabetically(mut obj: Map<String, Value>) -> Map<String, Value> {
    obj.sort_keys();
    obj
}

fn sort_object_recursive(mut obj: Map<String, Value>) -> Map<String, Value> {
    sort_object_recursive_in_place(&mut obj);
    obj
}

fn sort_object_recursive_in_place(obj: &mut Map<String, Value>) {
    for value in obj.values_mut() {
        if let Value::Object(nested) = value {
            sort_object_recursive_in_place(nested);
        }
    }
    obj.sort_keys();
}

/// Filters non-strings, sorts ascending, and removes duplicates.
fn sort_array_unique(mut arr: Vec<Value>) -> Vec<Value> {
    arr.retain(Value::is_string);
    // `unwrap` is sound: `retain` above guarantees every element is a string.
    arr.sort_unstable_by(|a, b| a.as_str().unwrap().cmp(b.as_str().unwrap()));
    arr.dedup_by(|a, b| a.as_str() == b.as_str());
    arr
}

/// Removes duplicate string entries while preserving original order. Used for fields
/// where order matters (e.g., `files` with `!` negation patterns).
fn dedupe_array(mut arr: Vec<Value>) -> Vec<Value> {
    let mut write = 0;
    for read in 0..arr.len() {
        let keep = match arr[read].as_str() {
            Some(s) => !arr[..write].iter().any(|seen| seen.as_str() == Some(s)),
            None => false,
        };
        if keep {
            if write != read {
                arr.swap(write, read);
            }
            write += 1;
        }
    }
    arr.truncate(write);
    arr
}

/// Reorders `obj` so that any keys present in `key_order` appear first (in the given
/// order), with the remaining keys following alphabetically.
///
/// Single-pass classification + merge — avoids `IndexMap::shift_remove`'s O(n) tail-shift
/// per requested key.
fn sort_object_by_key_order(obj: Map<String, Value>, key_order: &[&str]) -> Map<String, Value> {
    let mut known: Vec<Option<(String, Value)>> = (0..key_order.len()).map(|_| None).collect();
    let mut others: Vec<(String, Value)> = Vec::new();

    for (key, value) in obj {
        match key_order.iter().position(|kn| *kn == key.as_str()) {
            Some(idx) => known[idx] = Some((key, value)),
            None => others.push((key, value)),
        }
    }

    others.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));

    let mut result = Map::with_capacity(known.len() + others.len());
    for (key, value) in known.into_iter().flatten() {
        result.insert(key, value);
    }
    for (key, value) in others {
        result.insert(key, value);
    }
    result
}

fn sort_people_object(obj: Map<String, Value>) -> Map<String, Value> {
    sort_object_by_key_order(obj, &["name", "email", "url"])
}

// ===== Top-level field ordering =============================================

/// Declares the canonical order for known top-level `package.json` fields. For each
/// matched key, the field is bucketed with its order index; an optional transformation
/// expression (with `value` and `options` in scope) rewrites the value before storage.
/// Unknown fields fall through to the catch-all arm.
macro_rules! declare_field_order {
    (
        $key:ident, $value:ident, $known:ident, $unknown:ident;
        [ $( $idx:literal => $field_name:literal $( => $transform:expr )? ),* $(,)? ]
    ) => {
        match $key.as_str() {
            $(
                $field_name => $known.push((
                    $idx,
                    $key,
                    declare_field_order!(@value $value $(, $transform)?),
                )),
            )*
            _ => $unknown.push(($key, $value)),
        }
    };
    (@value $value:ident) => { $value };
    (@value $value:ident, $transform:expr) => { $transform };
}

fn sort_object_keys(obj: Map<String, Value>, options: &SortOptions) -> Map<String, Value> {
    // `known` collects fields with a canonical position; `unknown` collects everything
    // else, sorted with private (`_`-prefixed) keys after non-private ones.
    let mut known: Vec<(usize, String, Value)> = Vec::new();
    let mut unknown: Vec<(String, Value)> = Vec::new();

    for (key, value) in obj {
        declare_field_order!(key, value, known, unknown; [
            // Core Package Metadata
            0 => "$schema",
            1 => "name",
            2 => "displayName",
            3 => "version",
            4 => "stableVersion",
            5 => "gitHead",
            6 => "private",
            7 => "description",
            8 => "categories" => transform_array(value, sort_array_unique),
            9 => "keywords" => transform_array(value, sort_array_unique),
            10 => "homepage",
            11 => "bugs" => transform_with_key_order(value, &["url", "email"]),
            // License & People
            12 => "license",
            13 => "author" => transform_value(value, sort_people_object),
            14 => "maintainers",
            15 => "contributors",
            // Repository & Funding
            16 => "repository" => transform_with_key_order(value, &["type", "url"]),
            17 => "funding" => transform_with_key_order(value, &["type", "url"]),
            18 => "donate" => transform_with_key_order(value, &["type", "url"]),
            19 => "sponsor" => transform_with_key_order(value, &["type", "url"]),
            20 => "qna",
            21 => "publisher",
            // Package Content & Distribution
            22 => "man",
            23 => "style",
            24 => "example",
            25 => "examplestyle",
            26 => "assets",
            27 => "bin" => transform_value(value, sort_object_alphabetically),
            28 => "source",
            29 => "directories" => transform_with_key_order(value, &["lib", "bin", "man", "doc", "example", "test"]),
            30 => "workspaces",
            31 => "binary" => transform_with_key_order(value, &["module_name", "module_path", "remote_path", "package_name", "host"]),
            32 => "files" => transform_array(value, dedupe_array),
            33 => "os",
            34 => "cpu",
            35 => "libc" => transform_array(value, sort_array_unique),
            // Package Entry Points
            36 => "type",
            37 => "sideEffects",
            38 => "main",
            39 => "module",
            40 => "browser",
            41 => "types",
            42 => "typings",
            43 => "typesVersions",
            44 => "typeScriptVersion",
            45 => "typesPublisherContentHash",
            46 => "react-native",
            47 => "svelte",
            48 => "unpkg",
            49 => "jsdelivr",
            50 => "jsnext:main",
            51 => "umd",
            52 => "umd:main",
            53 => "es5",
            54 => "esm5",
            55 => "fesm5",
            56 => "es2015",
            57 => "esm2015",
            58 => "fesm2015",
            59 => "es2020",
            60 => "esm2020",
            61 => "fesm2020",
            62 => "esnext",
            63 => "imports",
            64 => "exports",
            65 => "publishConfig" => transform_value(value, |o| sort_object_keys(o, options)),
            // Scripts
            66 => "scripts" => if options.sort_scripts { transform_value(value, sort_object_alphabetically) } else { value },
            67 => "betterScripts" => if options.sort_scripts { transform_value(value, sort_object_alphabetically) } else { value },
            // Dependencies
            68 => "dependencies" => transform_value(value, sort_object_alphabetically),
            69 => "devDependencies" => transform_value(value, sort_object_alphabetically),
            70 => "dependenciesMeta",
            71 => "peerDependencies" => transform_value(value, sort_object_alphabetically),
            72 => "peerDependenciesMeta",
            73 => "optionalDependencies" => transform_value(value, sort_object_alphabetically),
            74 => "bundledDependencies" => transform_array(value, sort_array_unique),
            75 => "bundleDependencies" => transform_array(value, sort_array_unique),
            76 => "resolutions" => transform_value(value, sort_object_alphabetically),
            77 => "overrides" => transform_value(value, sort_object_alphabetically),
            // Git Hooks & Commit Tools
            78 => "husky" => transform_value(value, sort_object_recursive),
            79 => "simple-git-hooks",
            80 => "vite-staged",
            81 => "lint-staged",
            82 => "nano-staged",
            83 => "pre-commit",
            84 => "commitlint" => transform_value(value, sort_object_recursive),
            // VSCode Extension Specific
            85 => "l10n",
            86 => "contributes",
            87 => "activationEvents" => transform_array(value, sort_array_unique),
            88 => "extensionPack" => transform_array(value, sort_array_unique),
            89 => "extensionDependencies" => transform_array(value, sort_array_unique),
            90 => "extensionKind" => transform_array(value, sort_array_unique),
            91 => "icon",
            92 => "badges",
            93 => "galleryBanner",
            94 => "preview",
            95 => "markdown",
            // Build & Tool Configuration
            96 => "napi" => transform_value(value, sort_object_alphabetically),
            97 => "flat",
            98 => "config" => transform_value(value, sort_object_alphabetically),
            99 => "nodemonConfig" => transform_value(value, sort_object_recursive),
            100 => "browserify" => transform_value(value, sort_object_recursive),
            101 => "babel" => transform_value(value, sort_object_recursive),
            102 => "browserslist",
            103 => "xo" => transform_value(value, sort_object_recursive),
            104 => "prettier" => transform_value(value, sort_object_recursive),
            105 => "eslintConfig" => transform_value(value, sort_object_recursive),
            106 => "eslintIgnore",
            107 => "standard" => transform_value(value, sort_object_recursive),
            108 => "npmpkgjsonlint",
            109 => "npmPackageJsonLintConfig",
            110 => "npmpackagejsonlint",
            111 => "release",
            112 => "auto-changelog" => transform_value(value, sort_object_recursive),
            113 => "remarkConfig" => transform_value(value, sort_object_recursive),
            114 => "stylelint" => transform_value(value, sort_object_recursive),
            115 => "typescript" => transform_value(value, sort_object_recursive),
            116 => "typedoc" => transform_value(value, sort_object_recursive),
            117 => "tshy" => transform_value(value, sort_object_recursive),
            118 => "tsdown" => transform_value(value, sort_object_recursive),
            119 => "size-limit",
            // Testing
            120 => "ava" => transform_value(value, sort_object_recursive),
            121 => "jest" => transform_value(value, sort_object_recursive),
            122 => "jest-junit",
            123 => "jest-stare",
            124 => "mocha" => transform_value(value, sort_object_recursive),
            125 => "nyc" => transform_value(value, sort_object_recursive),
            126 => "c8" => transform_value(value, sort_object_recursive),
            127 => "tap",
            128 => "tsd" => transform_value(value, sort_object_recursive),
            129 => "typeCoverage" => transform_value(value, sort_object_recursive),
            130 => "oclif" => transform_value(value, sort_object_recursive),
            // Runtime & Package Manager
            131 => "languageName",
            132 => "preferGlobal",
            133 => "devEngines" => transform_value(value, sort_object_alphabetically),
            134 => "engines" => transform_value(value, sort_object_alphabetically),
            135 => "engineStrict",
            136 => "volta" => transform_value(value, sort_object_recursive),
            137 => "packageManager",
            138 => "pnpm",
        ]);
    }

    known.sort_unstable_by_key(|(idx, _, _)| *idx);
    // Single sort over all unknowns: non-private (`!_`) before private (`_`-prefixed),
    // each group alphabetical.
    unknown.sort_unstable_by(|(a, _), (b, _)| {
        let a_priv = a.starts_with('_');
        let b_priv = b.starts_with('_');
        a_priv.cmp(&b_priv).then_with(|| a.cmp(b))
    });

    let mut result = Map::with_capacity(known.len() + unknown.len());
    for (_, key, value) in known {
        result.insert(key, value);
    }
    for (key, value) in unknown {
        result.insert(key, value);
    }
    result
}
