<div align="center">

# sort-package-json

[![Crates.io][crates-badge]][crates-url]
[![Docs.rs][docs-badge]][docs-url]

[![MIT licensed][license-badge]][license-url]
[![Build Status][ci-badge]][ci-url]
[![Code Coverage][code-coverage-badge]][code-coverage-url]
[![CodSpeed Badge][codspeed-badge]][codspeed-url]
[![Sponsors][sponsors-badge]][sponsors-url]
[![Discord chat][discord-badge]][discord-url]

</div>

A Rust implementation of [sort-package-json](https://github.com/keithamus/sort-package-json) that sorts package.json files according to well-established npm conventions.

## Features

- **Sorts top-level fields** according to npm ecosystem conventions (138 predefined fields)
- **Preserves all data** - only reorders fields, never modifies values
- **Fast and safe** - pure Rust implementation with no unsafe code
- **Idempotent** - sorting multiple times produces the same result
- **Handles edge cases** - unknown fields sorted alphabetically, private fields (starting with `_`) sorted last

## Usage

```bash
cargo run
```

The tool will recursively find all `package.json` files in the current directory and sort them in place.

### Example

Given an unsorted package.json:

```json
{
  "version": "1.0.0",
  "dependencies": { ... },
  "name": "my-package",
  "scripts": { ... }
}
```

Running `cargo run package.json` produces:

```json
{
  "name": "my-package",
  "version": "1.0.0",
  "scripts": { ... },
  "dependencies": { ... }
}
```

## Field Ordering

Fields are sorted according to this priority:

1. **Known fields** - 138 predefined fields organized into 12 logical groups
2. **Unknown fields** - any custom fields sorted alphabetically
3. **Private fields** - fields starting with `_` sorted alphabetically at the end

The complete field order is based on both the [original sort-package-json](https://github.com/keithamus/sort-package-json/blob/main/index.js) and [prettier's package.json sorting](https://github.com/un-ts/prettier/blob/master/packages/pkg/src/rules/sort.ts) implementations.

### Known Field Groups

#### 1. Core Package Metadata
`$schema`, `name`, `displayName`, `version`, `stableVersion`, `gitHead`, `private`, `description`, `categories`, `keywords`, `homepage`, `bugs`

#### 2. License & People
`license`, `author`, `maintainers`, `contributors`

#### 3. Repository & Funding
`repository`, `funding`, `donate`, `sponsor`, `qna`, `publisher`

#### 4. Package Content & Distribution
`man`, `style`, `example`, `examplestyle`, `assets`, `bin`, `source`, `directories`, `workspaces`, `binary`, `files`, `os`, `cpu`, `libc`

#### 5. Package Entry Points
`type`, `sideEffects`, `main`, `module`, `browser`, `types`, `typings`, `typesVersions`, `typeScriptVersion`, `typesPublisherContentHash`, `react-native`, `svelte`, `unpkg`, `jsdelivr`, `jsnext:main`, `umd`, `umd:main`, `es5`, `esm5`, `fesm5`, `es2015`, `esm2015`, `fesm2015`, `es2020`, `esm2020`, `fesm2020`, `esnext`, `imports`, `exports`, `publishConfig`

#### 6. Scripts
`scripts`, `betterScripts`

#### 7. Dependencies
`dependencies`, `devDependencies`, `dependenciesMeta`, `peerDependencies`, `peerDependenciesMeta`, `optionalDependencies`, `bundledDependencies`, `bundleDependencies`, `resolutions`, `overrides`

#### 8. Git Hooks & Commit Tools
`husky`, `simple-git-hooks`, `pre-commit`, `lint-staged`, `nano-staged`, `commitlint`

#### 9. VSCode Extension Specific
`l10n`, `contributes`, `activationEvents`, `extensionPack`, `extensionDependencies`, `extensionKind`, `icon`, `badges`, `galleryBanner`, `preview`, `markdown`

#### 10. Build & Tool Configuration
`napi`, `flat`, `config`, `nodemonConfig`, `browserify`, `babel`, `browserslist`, `xo`, `prettier`, `eslintConfig`, `eslintIgnore`, `standard`, `npmpkgjsonlint`, `npmPackageJsonLintConfig`, `npmpackagejsonlint`, `release`, `auto-changelog`, `remarkConfig`, `stylelint`, `typescript`, `typedoc`, `tshy`, `tsdown`, `size-limit`

#### 11. Testing
`ava`, `jest`, `jest-junit`, `jest-stare`, `mocha`, `nyc`, `c8`, `tap`, `tsd`, `typeCoverage`, `oclif`

#### 12. Runtime & Package Manager
`languageName`, `preferGlobal`, `devEngines`, `engines`, `engineStrict`, `volta`, `packageManager`, `pnpm`

## Why Not simd-json?

We use serde_json instead of [simd-json](https://github.com/simd-lite/simd-json) because:

- **No preserve_order support** - simd-json can't maintain custom field insertion order (required for our sorting)
- **Platform issues** - simd-json doesn't work on big-endian architectures ([#437](https://github.com/simd-lite/simd-json/issues/437))

## Development

### Building

```bash
cargo build --release
```

### Running Tests

```bash
cargo test
```

Tests use snapshot testing via [insta](https://insta.rs/). To review and accept snapshot changes:

```bash
cargo insta review
```

Or to accept all changes:

```bash
cargo insta accept
```

### Test Coverage

- **Field ordering test** - verifies correct sorting of all field types
- **Idempotency test** - ensures sorting is stable (sorting twice = sorting once)

## License

MIT

## References

- [Original sort-package-json (JavaScript)](https://github.com/keithamus/sort-package-json)
- [simd-json issue #437 - Big Endian Compatibility](https://github.com/simd-lite/simd-json/issues/437)
- [Surprises in the Rust JSON Ecosystem](https://ecton.dev/rust-json-ecosystem/)

## [Sponsored By](https://github.com/sponsors/Boshen)

<p align="center">
  <a href="https://github.com/sponsors/Boshen">
    <img src="https://raw.githubusercontent.com/Boshen/sponsors/main/sponsors.svg" alt="My sponsors" />
  </a>
</p>

[discord-badge]: https://img.shields.io/discord/1079625926024900739?logo=discord&label=Discord
[discord-url]: https://discord.gg/9uXCAwqQZW
[license-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[license-url]: https://github.com/oxc-project/sort-package-json/blob/main/LICENSE
[ci-badge]: https://github.com/oxc-project/sort-package-json/actions/workflows/ci.yml/badge.svg?event=push&branch=main
[ci-url]: https://github.com/oxc-project/sort-package-json/actions/workflows/ci.yml?query=event%3Apush+branch%3Amain
[code-coverage-badge]: https://codecov.io/github/oxc-project/sort-package-json/branch/main/graph/badge.svg
[code-coverage-url]: https://codecov.io/gh/oxc-project/sort-package-json
[sponsors-badge]: https://img.shields.io/github/sponsors/Boshen
[sponsors-url]: https://github.com/sponsors/Boshen
[codspeed-badge]: https://img.shields.io/endpoint?url=https://codspeed.io/badge.json
[codspeed-url]: https://codspeed.io/oxc-project/sort-package-json
[crates-badge]: https://img.shields.io/crates/d/sort-package-json?label=crates.io
[crates-url]: https://crates.io/crates/sort-package-json
[docs-badge]: https://img.shields.io/docsrs/sort-package-json
[docs-url]: https://docs.rs/sort-package-json
