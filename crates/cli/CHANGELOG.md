# Changelog

All notable changes to `sluice-cli` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.4](https://github.com/overengineered-dev/sluice/compare/sluice-cli-v0.1.3...sluice-cli-v0.1.4) - 2026-04-18

### Other

- Cross-compile `x86_64-apple-darwin` from the arm64 (`macos-latest`) runner instead of using the `macos-13` Intel image, which had multi-hour queue times. No user-visible CLI changes vs 0.1.3.

## [0.1.3](https://github.com/overengineered-dev/sluice/compare/sluice-cli-v0.1.2...sluice-cli-v0.1.3) - 2026-04-18

### Other

- Fix the release-binaries workflow so it installs the `aarch64-unknown-linux-gnu` rustup target on the 1.85.0 toolchain pinned by `rust-toolchain.toml` (previously added to `stable`, which `cargo` wasn't actually using). 0.1.3 is the first release where Homebrew `Formula/sluice.rb` is auto-published to `overengineered-dev/homebrew-tap`. No user-visible CLI changes vs 0.1.2.

## [0.1.2](https://github.com/overengineered-dev/sluice/compare/sluice-cli-v0.1.1...sluice-cli-v0.1.2) - 2026-04-18

### Other

- First release with Homebrew formula auto-published to `overengineered-dev/homebrew-tap` and prebuilt binaries attached to the GitHub Release. No user-visible CLI changes versus 0.1.1.

## [0.1.1](https://github.com/overengineered-dev/sluice/compare/sluice-cli-v0.1.0...sluice-cli-v0.1.1) - 2026-04-18

### Added

- publish sluice via Homebrew tap on every release

### Other

- switch to per-crate CHANGELOG.md files
- update badges

## [0.1.0] - 2026-04-18

### Added

- Initial release.
- `sluice` binary that emits artifact records as JSON Lines.
