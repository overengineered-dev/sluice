<p align="center">
  <img src="docs/logo.png" alt="sluice logo" width="600">
</p>

# sluice

[![CI](https://github.com/nicarl/sluice/actions/workflows/ci.yml/badge.svg)](https://github.com/nicarl/sluice/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/sluice.svg)](https://crates.io/crates/sluice)
[![docs.rs](https://docs.rs/sluice/badge.svg)](https://docs.rs/sluice)
[![License: Apache-2.0](https://img.shields.io/crates/l/sluice.svg)](LICENSE)

Until now, reading the Maven Central index required the JVM, a custom script wiring up the Java `indexer-reader` library, and patience. Sluice is a single binary that does it 5x faster.

A fast, streaming parser for the [Maven Central Nexus binary index format](https://maven.apache.org/repository/central-index.html), plus a CLI that turns index files into JSON Lines.

For a byte-level specification of the wire format and incremental-update protocol, see [`docs/binary-format.md`](docs/binary-format.md).

## Layout

This is a Cargo workspace with two crates:

- **`crates/core`** — `sluice`, the library. I/O-neutral: operates on any `std::io::Read`, with no knowledge of gzip, HTTP, files, or JSON. Parses the Nexus binary header and record stream, decodes fields (including CESU-8 strings), and classifies documents into descriptors, group lists, and artifact add/remove records with parsed `UINFO` tuples.
- **`crates/cli`** — `sluice-cli`, which builds the `sluice` binary. Handles gzip decoding, argument parsing, and JSON Lines output on stdout.

## Quick start

```bash
# Fetch the latest incremental chunk into fixtures/chunk-latest.gz
just fetch-chunk

# Parse it and print artifact adds as JSON Lines (with stats on stderr)
just run-chunk

# Or parse the full Maven Central index (~700MB download, ~minutes to parse)
just fetch-full
just run-full
```

Under the hood:

```bash
cargo run --release -p sluice-cli -- --stats fixtures/chunk-latest.gz
```

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

```rust
use std::fs::File;
use std::io::BufReader;
use flate2::read::GzDecoder;
use sluice::{classify, IndexReader, Record};

let file = File::open("fixtures/chunk-latest.gz")?;
let gz = GzDecoder::new(BufReader::new(file));
let index = IndexReader::new(BufReader::new(gz))?;

for doc in index {
    let doc = doc?;
    match classify(&doc)? {
        Record::ArtifactAdd(u) => println!("add {}:{}:{}", u.group_id, u.artifact_id, u.version),
        Record::ArtifactRemove(u) => println!("del {}:{}:{}", u.group_id, u.artifact_id, u.version),
        Record::Descriptor | Record::AllGroups | Record::RootGroups | Record::Unknown => {}
    }
}
```

Enable the `serde` feature on `sluice` to derive `Serialize` for the domain types.

## Performance

Sluice is **~5x faster** than the Java [Apache Maven Indexer](https://github.com/apache/maven-indexer) `indexer-reader` on the full Maven Central index (2.8 GB compressed, ~97M documents):

| Tool | Mean | Relative |
|:---|---:|---:|
| sluice (Rust) | 225s | 1.00 |
| sluice --full (Rust) | 208s | 1.08 |
| indexer-reader (Java) | 1112s | 5.35 |

Both tools produce identical GAV (groupId, artifactId, version) output across all ~19.7M classifier=NA records, and identical GAV + classifier output across all ~97M records (`--full` mode). Note that the Java tool does additional work per record (field expansion via `RecordExpander`) that sluice largely skips, so the workloads are not identical — an apples-to-apples comparison would show a smaller gap. See [`docs/benchmark.md`](docs/benchmark.md) for a detailed discussion, methodology, and reproduction steps.

## Development

```bash
just fmt         # cargo fmt --all
just fmt-check   # cargo fmt --all -- --check
just lint        # cargo clippy --all-targets --all-features -- -D warnings
just test        # cargo test --all
```

The Rust toolchain is pinned via `rust-toolchain.toml`. Lints are workspace-wide: `rust_2018_idioms` denied and `clippy::pedantic` at warn level.

### Test fixtures

A small sample fixture (`fixtures/chunk-sample.gz`) is committed for offline testing. To regenerate it from a full Maven Central chunk:

```bash
just fetch-chunk
just regen-fixture
```

The full fixture is not committed to keep clone sizes small.

## License

Apache-2.0.
