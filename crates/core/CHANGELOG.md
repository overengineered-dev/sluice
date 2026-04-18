# Changelog

All notable changes to `sluice-rs` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-04-18

### Added

- Initial release.
- Pure-Rust streaming parser for the Maven Central Nexus binary index format.
- CESU-8 / Modified UTF-8 string decoding via the `cesu8` crate.
- Document classification: descriptor, all-groups, root-groups, artifact add/remove.
- UINFO tuple parsing with 4-segment and 5-segment support.
- Optional `serde` feature for `Serialize` derives on domain types.
