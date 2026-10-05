//! Sort `package.json` fields according to established npm conventions.
//!
//! The default API returns pretty-printed JSON with a trailing newline:
//!
//! ```
//! let input = r#"{"version":"1.0.0","name":"example"}"#;
//! let sorted = sort_package_json::sort_package_json(input)?;
//!
//! assert_eq!(sorted, "{\n  \"name\": \"example\",\n  \"version\": \"1.0.0\"\n}\n");
//! # Ok::<(), serde_json::Error>(())
//! ```

mod value;

use std::borrow::Cow;

use value::{Object, Value};

/// UTF-8 BOM (`U+FEFF`).
const BOM_STR: &str = "\u{FEFF}";

/// Options for controlling JSON formatting when sorting.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[non_exhaustive]
pub struct SortOptions {
    /// Whether to pretty-print the output JSON.
    pub pretty: bool,
    /// Whether to sort the scripts field alphabetically.
    pub sort_scripts: bool,
}

impl SortOptions {
    /// Creates options with the default behavior.
    #[must_use]
    pub const fn new() -> Self {
        Self { pretty: true, sort_scripts: false }
    }

    /// Sets whether the output is pretty-printed.
    #[must_use]
    pub const fn with_pretty(mut self, pretty: bool) -> Self {
        self.pretty = pretty;
        self
    }

    /// Sets whether the `scripts`, `betterScripts`, and `wireit` fields are sorted.
    #[must_use]
    pub const fn with_sort_scripts(mut self, sort_scripts: bool) -> Self {
        self.sort_scripts = sort_scripts;
        self
    }
}

impl Default for SortOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Sorts a `package.json` string using custom [`SortOptions`].
///
/// A UTF-8 byte order mark in `input` is preserved. Pretty-printed output ends in a newline;
/// compact output does not.
///
/// # Errors
///
/// Returns an error when `input` is not valid JSON.
pub fn sort_package_json_with_options(
    input: &str,
    options: &SortOptions,
) -> Result<String, serde_json::Error> {
    let (has_bom, body) =
        input.strip_prefix(BOM_STR).map_or((false, input), |stripped| (true, stripped));

    let value: Value<'_> = serde_json::from_str(body)?;

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

/// Sorts a `package.json` string with the default options.
///
/// The output is pretty-printed with a trailing newline. Script order is preserved.
///
/// # Errors
///
/// Returns an error when `input` is not valid JSON.
pub fn sort_package_json(input: &str) -> Result<String, serde_json::Error> {
    sort_package_json_with_options(input, &SortOptions::default())
}

// ===== Value-level transformations ==========================================

#[inline]
fn transform_value<'a, F>(value: Value<'a>, f: F) -> Value<'a>
where
    F: FnOnce(Object<'a>) -> Object<'a>,
{
    match value {
        Value::Object(o) => Value::Object(f(o)),
        other => other,
    }
}

#[inline]
fn transform_array<'a, F>(value: Value<'a>, f: F) -> Value<'a>
where
    F: FnOnce(Vec<Value<'a>>) -> Vec<Value<'a>>,
{
    match value {
        Value::Array(arr) => Value::Array(f(arr)),
        other => other,
    }
}

#[inline]
fn transform_with_key_order<'a>(value: Value<'a>, key_order: &[&str]) -> Value<'a> {
    transform_value(value, |o| sort_object_by_key_order(o, key_order))
}

fn sort_object_alphabetically(mut obj: Object<'_>) -> Object<'_> {
    obj.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));
    obj
}

fn sort_object_recursive(mut obj: Object<'_>) -> Object<'_> {
    sort_object_recursive_in_place(&mut obj);
    obj
}

fn sort_object_recursive_in_place(obj: &mut Object<'_>) {
    for (_, value) in &mut *obj {
        if let Value::Object(nested) = value {
            sort_object_recursive_in_place(nested);
        }
    }
    obj.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));
}

/// Filters non-strings, sorts ascending, and removes duplicates.
fn sort_array_unique(mut arr: Vec<Value<'_>>) -> Vec<Value<'_>> {
    arr.retain(Value::is_string);
    // `unwrap` is sound: `retain` above guarantees every element is a string.
    arr.sort_unstable_by(|a, b| a.as_str().unwrap().cmp(b.as_str().unwrap()));
    arr.dedup_by(|a, b| a.as_str() == b.as_str());
    arr
}

/// Removes duplicate string entries while preserving original order. Used for fields
/// where order matters (e.g., `files` with `!` negation patterns).
fn dedupe_array(mut arr: Vec<Value<'_>>) -> Vec<Value<'_>> {
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
/// Single-pass classification followed by an alphabetical sort of unrecognized keys.
fn sort_object_by_key_order<'a>(obj: Object<'a>, key_order: &[&str]) -> Object<'a> {
    let mut known: Vec<Option<(Cow<'a, str>, Value<'a>)>> =
        (0..key_order.len()).map(|_| None).collect();
    let mut others: Object<'a> = Vec::new();

    for (key, value) in obj {
        match key_order.iter().position(|kn| *kn == key.as_ref()) {
            Some(idx) => known[idx] = Some((key, value)),
            None => others.push((key, value)),
        }
    }

    others.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));

    let mut result = Vec::with_capacity(known.len() + others.len());
    for (key, value) in known.into_iter().flatten() {
        result.push((key, value));
    }
    for (key, value) in others {
        result.push((key, value));
    }
    result
}

fn sort_people_object(obj: Object<'_>) -> Object<'_> {
    sort_object_by_key_order(obj, &["name", "email", "url"])
}

// ===== Scripts sorting with lifecycle grouping ==============================

/// npm lifecycle scripts whose `pre` / `post` prefixes are always recognized,
/// even when the base name itself is not explicitly present in the `scripts`
/// object. See <https://docs.npmjs.com/cli/v10/using-npm/scripts>.
const DEFAULT_NPM_SCRIPTS: &[&str] = &[
    "install",
    "pack",
    "prepare",
    "publish",
    "restart",
    "shrinkwrap",
    "start",
    "stop",
    "test",
    "uninstall",
    "version",
];

/// Recursively sort script names by colon-delimited namespace.
///
/// 1. All names are partitioned into groups. The group key for a name is the
///    text before the **first** colon after `prefix`. Names that have no colon
///    (i.e. are exactly the group key) are "direct" members.
/// 2. Groups are sorted alphabetically by group key.
/// 3. Within each group the direct member (if any) comes first, followed by
///    the colon-children **recursively sorted** using the group key as the new
///    prefix.
///
/// This mirrors the JS `sortScriptNames` in `sort-package-json`.
fn sort_script_names(names: &[&str], prefix: &str) -> Vec<String> {
    // Collect groups: group_key → list of full names.
    let mut groups: Vec<(&str, Vec<&str>)> = Vec::new();
    let mut group_index = std::collections::HashMap::<&str, usize>::new();

    for &name in names {
        // Portion of the name after the prefix (+ separator colon).
        let rest = if prefix.is_empty() { name } else { &name[prefix.len() + 1..] };
        let colon_idx = rest.find(':');
        let group_key = match colon_idx {
            // Has a colon after at least one character → group key is the part before it.
            // If the colon is at position 0 (name starts with `:` or `prefix::`) the
            // name is treated as having no sub-namespace — same as `None`.
            Some(idx) if idx > 0 => {
                let base_len = if prefix.is_empty() { idx } else { prefix.len() + 1 + idx };
                &name[..base_len]
            }
            // No colon, or colon at position 0 → the name itself is the group key.
            _ => name,
        };

        if let Some(&gi) = group_index.get(group_key) {
            groups[gi].1.push(name);
        } else {
            group_index.insert(group_key, groups.len());
            groups.push((group_key, vec![name]));
        }
    }

    // Sort groups by group key.
    groups.sort_by_key(|(a, _)| *a);

    let mut result = Vec::with_capacity(names.len());
    for (group_key, children) in groups {
        if children.len() > 1
            && children.iter().any(|k| *k != group_key && k.starts_with(group_key))
        {
            // Separate direct members from colon-children.
            let mut direct: Vec<&str> = children
                .iter()
                .filter(|k| **k == group_key || !k.starts_with(&format!("{group_key}:")))
                .copied()
                .collect();
            direct.sort();

            let nested: Vec<&str> = children
                .iter()
                .filter(|k| k.starts_with(&format!("{group_key}:")))
                .copied()
                .collect();

            result.extend(direct.into_iter().map(String::from));
            result.extend(sort_script_names(&nested, group_key));
        } else {
            let mut sorted: Vec<&str> = children;
            sorted.sort();
            result.extend(sorted.into_iter().map(String::from));
        }
    }
    result
}

/// Sorts a `scripts` (or `betterScripts`) object with colon-namespace grouping
/// and lifecycle (`pre` / `post`) script collocation.
///
/// The algorithm (matching the npm `sort-package-json` package):
///
/// 1. **Deduplicate prefixable names** — for every name starting with `pre` or
///    `post`, check whether the remainder is either a default npm lifecycle
///    script **or** an explicit script in the object. If so, record the base
///    name as *prefixable* and replace the original with its base name in the
///    sort input.
/// 2. **Sort by colon namespace** — call [`sort_script_names`].
/// 3. **Expand prefixable names** — each prefixable base name is expanded to
///    `[pre<name>, <name>, post<name>]`.
/// 4. **Reorder the object** according to the computed name list. Scripts whose
///    names do not appear in the list (e.g. orphan `pre` / `post` scripts after
///    expansion that have no corresponding entry in the original object) are
///    silently omitted, and scripts present in the original but absent from the
///    list are appended in their original order.
fn sort_scripts_object(obj: Object<'_>) -> Object<'_> {
    let keys: Vec<&str> = obj.iter().map(|(k, _)| k.as_ref()).collect();
    let key_set: std::collections::HashSet<&str> = keys.iter().copied().collect();

    // Step 1: Determine which base names are prefixable.
    let mut prefixable = std::collections::HashSet::new();
    let mut deduped_names: Vec<&str> = Vec::with_capacity(keys.len());

    for &name in &keys {
        if let Some(base) = name.strip_prefix("pre") {
            if !base.is_empty()
                && (DEFAULT_NPM_SCRIPTS.contains(&base) || key_set.contains(base))
            {
                prefixable.insert(base);
                if !deduped_names.contains(&base) {
                    deduped_names.push(base);
                }
                continue;
            }
        }
        if let Some(base) = name.strip_prefix("post") {
            if !base.is_empty()
                && (DEFAULT_NPM_SCRIPTS.contains(&base) || key_set.contains(base))
            {
                prefixable.insert(base);
                if !deduped_names.contains(&base) {
                    deduped_names.push(base);
                }
                continue;
            }
        }
        if !deduped_names.contains(&name) {
            deduped_names.push(name);
        }
    }

    // Step 2: Sort by colon namespace.
    let sorted_names = sort_script_names(&deduped_names, "");

    // Step 3: Expand prefixable names.
    let mut ordered_names: Vec<String> = Vec::with_capacity(keys.len());
    for name in &sorted_names {
        if prefixable.contains(name.as_str()) {
            let pre = format!("pre{name}");
            let post = format!("post{name}");
            ordered_names.push(pre);
            ordered_names.push(name.clone());
            ordered_names.push(post);
        } else {
            ordered_names.push(name.clone());
        }
    }

    // Step 4: Reorder object entries.
    // Build a position map for O(1) lookup.
    let position: std::collections::HashMap<&str, usize> = ordered_names
        .iter()
        .enumerate()
        .map(|(i, name)| (name.as_str(), i))
        .collect();

    let mut entries: Vec<(usize, Cow<'_, str>, Value<'_>)> = Vec::with_capacity(obj.len());
    let mut unordered: Object<'_> = Vec::new();

    for (key, value) in obj {
        match position.get(key.as_ref()) {
            Some(&pos) => entries.push((pos, key, value)),
            // Original key not in expansion list → append at end.
            None => unordered.push((key, value)),
        }
    }

    entries.sort_by_key(|(pos, _, _)| *pos);

    let mut result = Vec::with_capacity(entries.len() + unordered.len());
    for (_, key, value) in entries {
        result.push((key, value));
    }
    result.extend(unordered);
    result
}

fn sort_dev_engine(value: Value<'_>) -> Value<'_> {
    const KEY_ORDER: &[&str] = &["name", "version", "onFail"];

    match value {
        Value::Array(arr) => Value::Array(
            arr.into_iter().map(|value| transform_with_key_order(value, KEY_ORDER)).collect(),
        ),
        other => transform_with_key_order(other, KEY_ORDER),
    }
}

fn sort_dev_engines(obj: Object<'_>) -> Object<'_> {
    let mut obj: Object<'_> =
        obj.into_iter().map(|(key, value)| (key, sort_dev_engine(value))).collect();
    obj.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));
    obj
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
        match $key.as_ref() {
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

fn sort_object_keys<'a>(obj: Object<'a>, options: &SortOptions) -> Object<'a> {
    // `known` collects fields with a canonical position; `unknown` collects everything
    // else, sorted with private (`_`-prefixed) keys after non-private ones.
    let mut known: Vec<(usize, Cow<'a, str>, Value<'a>)> = Vec::new();
    let mut unknown: Object<'a> = Vec::new();

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
            66 => "scripts" => if options.sort_scripts { transform_value(value, sort_scripts_object) } else { value },
            67 => "betterScripts" => if options.sort_scripts { transform_value(value, sort_scripts_object) } else { value },
            68 => "wireit" => if options.sort_scripts { transform_value(value, sort_object_alphabetically) } else { value },
            // Dependencies
            69 => "dependencies" => transform_value(value, sort_object_alphabetically),
            70 => "devDependencies" => transform_value(value, sort_object_alphabetically),
            71 => "dependenciesMeta",
            72 => "peerDependencies" => transform_value(value, sort_object_alphabetically),
            73 => "peerDependenciesMeta",
            74 => "optionalDependencies" => transform_value(value, sort_object_alphabetically),
            75 => "bundledDependencies" => transform_array(value, sort_array_unique),
            76 => "bundleDependencies" => transform_array(value, sort_array_unique),
            77 => "resolutions" => transform_value(value, sort_object_alphabetically),
            78 => "overrides" => transform_value(value, sort_object_alphabetically),
            // Git Hooks & Commit Tools
            79 => "husky" => transform_value(value, sort_object_recursive),
            80 => "simple-git-hooks",
            81 => "vite-staged",
            82 => "lint-staged",
            83 => "nano-staged",
            84 => "pre-commit",
            85 => "commitlint" => transform_value(value, sort_object_recursive),
            // VSCode Extension Specific
            86 => "l10n",
            87 => "contributes",
            88 => "activationEvents" => transform_array(value, sort_array_unique),
            89 => "extensionPack" => transform_array(value, sort_array_unique),
            90 => "extensionDependencies" => transform_array(value, sort_array_unique),
            91 => "extensionKind" => transform_array(value, sort_array_unique),
            92 => "icon",
            93 => "badges",
            94 => "galleryBanner",
            95 => "preview",
            96 => "markdown",
            // Build & Tool Configuration
            97 => "napi" => transform_value(value, sort_object_alphabetically),
            98 => "flat",
            99 => "config" => transform_value(value, sort_object_alphabetically),
            100 => "nodemonConfig" => transform_value(value, sort_object_recursive),
            101 => "browserify" => transform_value(value, sort_object_recursive),
            102 => "babel" => transform_value(value, sort_object_recursive),
            103 => "browserslist",
            104 => "xo" => transform_value(value, sort_object_recursive),
            105 => "prettier" => transform_value(value, sort_object_recursive),
            106 => "eslintConfig" => transform_value(value, sort_object_recursive),
            107 => "eslintIgnore",
            108 => "standard" => transform_value(value, sort_object_recursive),
            109 => "npmpkgjsonlint",
            110 => "npmPackageJsonLintConfig",
            111 => "npmpackagejsonlint",
            112 => "release",
            // Only sorts top-level keys: `replaceText` maps regexes to replacements applied sequentially in key order
            113 => "auto-changelog" => transform_value(value, sort_object_alphabetically),
            // Only sorts top-level keys: `plugins` in object form runs plugins in key order
            114 => "remarkConfig" => transform_value(value, sort_object_alphabetically),
            115 => "stylelint" => transform_value(value, sort_object_recursive),
            116 => "typescript" => transform_value(value, sort_object_recursive),
            117 => "typedoc" => transform_value(value, sort_object_recursive),
            // Only sorts top-level keys: `exports` values may be pass-through conditional exports
            118 => "tshy" => transform_value(value, sort_object_alphabetically),
            119 => "tsdown" => transform_value(value, sort_object_recursive),
            120 => "size-limit",
            // Testing
            121 => "ava" => transform_value(value, sort_object_recursive),
            // Only sorts top-level keys: nested config like `moduleNameMapper` is order-dependent
            122 => "jest" => transform_value(value, sort_object_alphabetically),
            123 => "jest-junit",
            124 => "jest-stare",
            125 => "mocha" => transform_value(value, sort_object_recursive),
            126 => "nyc" => transform_value(value, sort_object_recursive),
            127 => "c8" => transform_value(value, sort_object_recursive),
            128 => "tap",
            129 => "tsd" => transform_value(value, sort_object_recursive),
            130 => "typeCoverage" => transform_value(value, sort_object_recursive),
            131 => "oclif" => transform_value(value, sort_object_recursive),
            // Runtime & Package Manager
            132 => "languageName",
            133 => "preferGlobal",
            134 => "devEngines" => transform_value(value, sort_dev_engines),
            135 => "engines" => transform_value(value, sort_object_alphabetically),
            136 => "engineStrict",
            137 => "volta" => transform_value(value, sort_object_recursive),
            138 => "packageManager",
            139 => "pnpm",
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

    let mut result = Vec::with_capacity(known.len() + unknown.len());
    for (_, key, value) in known {
        result.push((key, value));
    }
    for (key, value) in unknown {
        result.push((key, value));
    }
    result
}
