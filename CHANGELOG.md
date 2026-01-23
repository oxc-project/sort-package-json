# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.10](https://github.com/oxc-project/sort-package-json/compare/v0.0.9...v0.0.10) - 2026-01-23

### Fixed

- preserve `files` array order for negation patterns

## [0.0.9](https://github.com/oxc-project/sort-package-json/compare/v0.0.8...v0.0.9) - 2026-01-20

### Other

- Add UTF-8 BOM support for package.json files ([#39](https://github.com/oxc-project/sort-package-json/pull/39))

## [0.0.8](https://github.com/oxc-project/sort-package-json/compare/v0.0.7...v0.0.8) - 2026-01-07

### Other

- Fix size-limit field being emptied during package.json sorting ([#29](https://github.com/oxc-project/sort-package-json/pull/29))

### Fixed

- Fix `size-limit` field being emptied when sorting - the field now preserves its array of configuration objects instead of filtering them out

## [0.0.7](https://github.com/oxc-project/sort-package-json/compare/v0.0.6...v0.0.7) - 2025-12-29

### Added

- add `sort_scripts` option to sort scripts field alphabetically ([#22](https://github.com/oxc-project/sort-package-json/pull/22))

## [0.0.6](https://github.com/oxc-project/sort-package-json/compare/v0.0.5...v0.0.6) - 2025-12-26

### Fixed

- do not sort contributors nor maintainers because they can prioritized

### Other

- rewrite integration tests with comprehensive fixture
- Replace field ordering list with annotated JSONC example
- Update README with library API usage and example runner
- Add explicit compatibility note to README ([#17](https://github.com/oxc-project/sort-package-json/pull/17))
- Condense 'Why Not simd-json?' section to bullet points

## [0.0.5](https://github.com/oxc-project/sort-package-json/compare/v0.0.4...v0.0.5) - 2025-12-17

### Other

- Optimize more functions with in-place mutations
- Optimize array sorting with in-place operations ([#14](https://github.com/oxc-project/sort-package-json/pull/14))
- Use unstable sort for better performance ([#13](https://github.com/oxc-project/sort-package-json/pull/13))
- Sort files field with natural path sorting ([#10](https://github.com/oxc-project/sort-package-json/pull/10))

## [0.0.4](https://github.com/oxc-project/sort-package-json/compare/v0.0.3...v0.0.4) - 2025-12-17

### Fixed

- Keep `exports` paths order ([#5](https://github.com/oxc-project/sort-package-json/pull/5))

### Other

- Add 12 commonly-used fields from npm ecosystem analysis ([#8](https://github.com/oxc-project/sort-package-json/pull/8))
- Improve field grouping with clearer logical organization ([#7](https://github.com/oxc-project/sort-package-json/pull/7))

## [0.0.3](https://github.com/oxc-project/sort-package-json/compare/v0.0.2...v0.0.3) - 2025-12-10

### Other

- Move main.rs to examples and make ignore a dev dependency
- Replace cloning with ownership and mutation
- fmt

## [0.0.2](https://github.com/oxc-project/sort-package-json/compare/v0.0.1...v0.0.2) - 2025-12-08

### Other

- Update README field count to 126
- Use unstable sort for better performance
- Move main field below type field
- Add declare_field_order! macro to simplify field ordering
- Add napi field after bundleDependencies
- Refactor value transformation with helper functions
