# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- `--no-color` and `NO_COLOR` now correctly suppress all ANSI codes. Plain
  `.red()` / `.green()` etc. emit codes unconditionally; switched everything to
  `if_supports_color()` which respects the owo-colors override system.

## [0.1.1] - 2026-03-15

### Added
- Interactive fix mode: resolver control loop with IO abstraction and mock-driven tests
- Filesystem operation layer with `--dry-run` support
- `--batch-confirm` flag to collect all decisions then confirm before applying
- Safety confirmation layer for destructive actions (remove)
- Fuzzy candidate search with Levenshtein distance, path similarity, and depth penalty scoring
- Configurable ignore patterns (`-I`) propagated to scanner and fuzzy walker
- Cross-filesystem and loop-detection warnings
- CI lint workflow
- Release profile with LTO, single codegen unit, and stripped binaries

### Changed
- Replaced subprocess-based resolution with native Rust implementation

## [0.1.0] - 2026-03-13

### Added
- Initial release
- Recursive broken symlink scanner
- Reports dead target path for each broken link
- Basic `--list` and `--path` flags
- Fix for correctly identifying broken symlinks (not all symlinks)
