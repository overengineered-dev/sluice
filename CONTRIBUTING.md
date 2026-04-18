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