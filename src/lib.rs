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

  let sorted_value = if let Value::Object(obj) = value {
    Value::Object(sort_object_keys(obj))
  } else {
    value
  };

  serde_json::to_string_pretty(&sorted_value)
}

fn sort_object_keys(obj: Map<String, Value>) -> Map<String, Value> {
  let mut result = Map::new();
  let mut remaining_keys: Vec<String> = obj.keys().cloned().collect();

  // First, add fields in the order specified by FIELDS_ORDER
  for &field in FIELDS_ORDER {
    if let Some(value) = obj.get(field) {
      result.insert(field.to_string(), value.clone());
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
