# Contributing to sluice

Thanks for your interest in contributing! This document covers the basics.

## Getting started

You need a Rust toolchain (the version is pinned in `rust-toolchain.toml`) and [`just`](https://github.com/casey/just) as a task runner (`cargo install just` or `brew install just`).

```bash
just fmt         # format all code
just lint        # cargo clippy with pedantic warnings
just test        # run all tests
```

## Test fixtures

A small sample fixture (`crates/core/tests/fixtures/chunk-sample.gz`) is committed so tests run offline. To test against a real Maven Central chunk:

```bash
just fetch-chunk          # download latest incremental chunk
just regen-fixture        # regenerate the committed sample from it
```

## Pull request guidelines

- Run `just fmt` and `just lint` before opening a PR. CI enforces both.
- Add tests for new functionality.
- Keep commits focused — one logical change per commit.
- Write a clear PR description explaining *why*, not just *what*.

## Code style

- Workspace lints: `rust_2018_idioms` is denied, `clippy::pedantic` is at warn level.
- No comments unless the *why* is non-obvious.
- Prefer returning `Result` over panicking.

## Reporting bugs

Open a [GitHub issue](https://github.com/overengineered-dev/sluice/issues) with:

- What you expected to happen
- What actually happened
- Steps to reproduce
- Your Rust version (`rustc --version`)

## Releases

Releases are automated via [release-plz](https://release-plz.dev/). The flow:

1. Commits land on `main` following the [Conventional Commits](https://www.conventionalcommits.org/) format.
2. A bot opens (or updates) a "release-plz" PR with the next version bumps and changelog entries.
3. A maintainer reviews and merges the release PR.
4. release-plz publishes the affected crates to crates.io, creates per-crate git tags (e.g. `sluice-rs-v0.2.0`, `sluice-cli-v0.2.0`), and creates per-crate GitHub Releases.
5. For each `sluice-cli-v*` tag, the `release-binaries` workflow builds prebuilt CLI binaries for Linux (x86_64, aarch64), macOS (x86_64, aarch64), and Windows (x86_64), and attaches them to the GitHub Release.

### Commit message format

Use Conventional Commits so version bumps and changelog entries are derived correctly:

- `feat:` → minor version bump (or patch pre-1.0).
- `fix:` / `perf:` → patch bump.
- `feat!:` or any commit with a `BREAKING CHANGE:` footer → major bump.
- `chore:`, `ci:`, `docs:`, `test:`, `refactor:` → no version bump (no changelog entry).

Use the **scope** to indicate which crate is affected:

- `feat(core): ...` for changes to `crates/core` (`sluice-rs`).
- `feat(cli): ...` for changes to `crates/cli` (`sluice-cli`).
- Unscoped commits affect both crates.

Examples:

```
feat(core): support 6-segment UINFO tuples
fix(cli): handle stdin EOF without panic
docs: clarify benchmark methodology
```