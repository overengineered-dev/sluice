<p align="center">
  <img src="docs/logo.png" alt="sluice logo" width="600">
</p>

# sluice

[![CI](https://github.com/overengineered-dev/sluice/actions/workflows/ci.yml/badge.svg)](https://github.com/overengineered-dev/sluice/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/sluice-rs.svg)](https://crates.io/crates/sluice-rs)
[![docs.rs](https://img.shields.io/docsrs/sluice-rs/latest)](https://docs.rs/sluice-rs)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

Until now, reading the Maven Central index required the JVM, a custom script wiring up the Java `indexer-reader` library, and patience. Sluice is a single binary that does it 5x faster.

A fast, streaming parser for the [Maven Central Nexus binary index format](https://maven.apache.org/repository/central-index.html), plus a CLI that turns index files into JSON Lines.

For a byte-level specification of the wire format, see [`docs/binary-format.md`](docs/binary-format.md). For the incremental-update protocol, see [`docs/incremental-updates.md`](docs/incremental-updates.md).

## Layout

This is a Cargo workspace with two crates:

- **`crates/core`** — `sluice`, the library. I/O-neutral: operates on any `std::io::Read`, with no knowledge of gzip, HTTP, files, or JSON. Parses the Nexus binary header and record stream, decodes fields (including CESU-8 strings), and classifies documents into descriptors, group lists, and artifact add/remove records with parsed `UINFO` tuples.
- **`crates/cli`** — `sluice-cli`, which builds the `sluice` binary. Handles gzip decoding, argument parsing, and JSON Lines output on stdout.

## Installation

### Homebrew (macOS and Linux)

```bash
brew install overengineered-dev/tap/sluice
```

### Cargo

```bash
cargo install sluice-cli
```

### Prebuilt archives

Download the archive for your platform from the [latest release](https://github.com/overengineered-dev/sluice/releases/latest), extract, and move `sluice` onto your `PATH`.

### From source

```bash
git clone https://github.com/overengineered-dev/sluice
cd sluice
cargo install --path crates/cli
```

## Quick start

```bash
# Parse a gzipped Maven Central index chunk and print artifact adds as
# JSON Lines (with stats on stderr).
sluice --stats chunk-latest.gz
```

Or stream the full Maven Central index straight from Apache without saving it to disk (~2.8 GB compressed, several minutes to parse):

```bash
curl -sL https://repo1.maven.org/maven2/.index/nexus-maven-repository-index.gz \
  | sluice --stats > artifacts.jsonl
```

Contributors working from a clone can use the `just` recipes — see [Development](#development) below.

### CLI options

```
sluice [OPTIONS] [INPUT]
```

- `INPUT` — path to a gzipped Maven index file. Reads from stdin if omitted.
- `--include-removes` — also emit `ArtifactRemove` records (`type="remove"`) alongside adds.
- `--full` — emit all records including classified artifacts (sources, javadoc, etc.) with their classifier and extension. By default, only root-level artifacts (classifier=NA) are emitted.
- `--stats` — print summary stats to stderr at end of run.

Output is one JSON object per line, e.g.:

```json
{"type":"add","group_id":"org.example","artifact_id":"lib","version":"1.0","extension":"jar"}
```

With `--full`, classified artifacts are included and the `classifier` field appears:

```json
{"type":"add","group_id":"org.example","artifact_id":"lib","version":"1.0","extension":"jar"}
{"type":"add","group_id":"org.example","artifact_id":"lib","version":"1.0","classifier":"sources","extension":"jar"}
{"type":"add","group_id":"org.example","artifact_id":"lib","version":"1.0","classifier":"javadoc","extension":"jar"}
```

By default, records whose classifier is anything other than `NA` are filtered out. Use `--full` to include all records.

## Library usage

The core library is I/O-neutral — it reads from any `std::io::Read`. For gzipped index files, bring your own decompressor (e.g. `flate2`). The crate is published as `sluice-rs` on crates.io; the import path is `sluice`:

```toml
[dependencies]
sluice-rs = "0.1"
flate2 = "1"
```

```rust
use std::fs::File;
use std::io::BufReader;
use flate2::read::GzDecoder;
use sluice::{IndexReader, Record};

let file = File::open("fixtures/chunk-latest.gz")?;
let gz = GzDecoder::new(BufReader::new(file));
let index = IndexReader::new(BufReader::new(gz))?;

for doc in index {
    let doc = doc?;
    // `Uinfo` implements `Display` as `groupId:artifactId:version[:classifier][:extension]`.
    match Record::try_from(&doc)? {
        Record::ArtifactAdd(u) => println!("add {u}"),
        Record::ArtifactRemove(u) => println!("del {u}"),
        // `Record` is `#[non_exhaustive]`; match `_` for descriptors, group lists,
        // and any future variants.
        _ => {}
    }
}
```

### Serde support

Enable the `serde` feature to derive `Serialize` on all domain types (`Record`, `Uinfo`, `Document`, etc.):

```toml
[dependencies]
sluice-rs = { version = "0.1", features = ["serde"] }
serde_json = "1"
```

```rust
use sluice::{IndexReader, Record};

// ... set up IndexReader as above ...

for doc in index {
    let doc = doc?;
    if let Record::ArtifactAdd(ref uinfo) = Record::try_from(&doc)? {
        println!("{}", serde_json::to_string(uinfo)?);
    }
}
```

## Performance

Sluice is **~5x faster** than the Java [Apache Maven Indexer](https://github.com/apache/maven-indexer) `indexer-reader` on the full Maven Central index (2.8 GB compressed, ~97M documents):

| Tool | Mean | Relative |
|:---|---:|---:|
| sluice (Rust) | 208s | 1.00 |
| indexer-reader (Java) | 1112s | 5.35 |

Both tools produce identical output across all ~97M records. The Java tool does additional per-record work (field expansion via `RecordExpander`) that sluice skips, so the workloads are not identical. See [`docs/benchmark.md`](docs/benchmark.md) for methodology, reproduction steps, and a detailed discussion.

## Development

Recipes are run through [`just`](https://github.com/casey/just) (`cargo install just` or `brew install just`):

```bash
just fmt         # cargo fmt --all
just fmt-check   # cargo fmt --all -- --check
just lint        # cargo clippy --all-targets --all-features -- -D warnings
just test        # cargo test --all
just fetch-chunk # download the latest incremental chunk into fixtures/
just run-chunk   # parse fixtures/chunk-latest.gz with --stats
just fetch-full  # download the full Maven Central index (~2.8 GB)
just run-full    # parse the full index
```

The Rust toolchain is pinned via `rust-toolchain.toml`. MSRV is **1.75** for the library (`sluice-rs`) and **1.85** for the CLI (`sluice-cli`) — `clap` transitively requires `edition2024`. Lints are workspace-wide: `rust_2018_idioms` denied and `clippy::pedantic` at warn level.

### Test fixtures

A small sample fixture (`crates/core/tests/fixtures/chunk-sample.gz`) is committed for offline testing. To regenerate it from a full Maven Central chunk:

```bash
just fetch-chunk
just regen-fixture
```

The full fixture is not committed to keep clone sizes small.

## License

Apache-2.0.
