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

- **Sorts top-level fields** according to npm ecosystem conventions (97 predefined fields)
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

1. **Known fields** - 97 predefined fields in standard order (name, version, description, keywords, ...)
2. **Unknown fields** - any custom fields sorted alphabetically
3. **Private fields** - fields starting with `_` sorted alphabetically at the end

The complete field order follows the [reference implementation](https://github.com/keithamus/sort-package-json/blob/main/index.js).

## Why Not simd-json?

While investigating performance optimizations, we considered using [simd-json](https://github.com/simd-lite/simd-json) instead of serde_json. However, simd-json is not suitable for this project due to several technical limitations:

### 1. No preserve_order Support

Our sorting algorithm requires maintaining custom insertion order. We insert fields in a specific sequence (known fields � unknown fields � private fields) and need the Map to preserve that exact order during serialization.

simd-json lacks the equivalent of serde_json's `preserve_order` feature, which uses IndexMap to maintain insertion order. Without this, the Map implementation would re-sort keys alphabetically, completely breaking our field ordering logic.

**Status**: Blocking issue - makes simd-json incompatible with our core functionality.

### 2. Wrong Performance Profile

simd-json is optimized for **large files** (1MB+) through SIMD acceleration, but is **slower for small files** due to SIMD overhead:

- For small objects: serde_json is **1.6x faster**
- For large objects (1.8MB): simd-json is **3x faster**

Package.json files are typically **1-5 KB**, rarely exceeding 50 KB even for large monorepos. This makes them squarely in the "small file" category where simd-json would actually **decrease performance**.

### 3. Platform Compatibility Issues

simd-json does not work correctly on **big-endian architectures** (e.g., s390x/IBM Z mainframe). Projects using simd-json must implement conditional compilation to fall back to serde_json on big-endian platforms.

See [simd-json issue #437](https://github.com/simd-lite/simd-json/issues/437) for details.

### 4. Additional Complexity

- Contains substantial **unsafe code** (C++ port)
- Requires specific allocators (mimalloc/jemalloc) for optimal performance
- More complex dependency tree

### Conclusion

For a package.json sorting tool, **serde_json is the optimal choice**:

-  Faster for small files (our use case)
-  Supports preserve_order (required feature)
-  Safe, stable, cross-platform
-  Simpler dependency tree
-  No platform-specific limitations

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
