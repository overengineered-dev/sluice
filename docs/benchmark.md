# Comparative benchmark: sluice vs Java indexer-reader

Until now, reading the Maven Central index required the JVM, a custom script wiring up the Java `indexer-reader` library, and patience. Sluice is a single binary that does it 7x faster.

## What is compared

Sluice is compared against the **Apache Maven `indexer-reader`** library
(version 7.1.6), *not* the `indexer-cli` JAR. The CLI only packs/unpacks
Lucene indexes and has no text-dump mode, so it cannot be used for output
comparison.

A small Java wrapper (`scripts/java/DumpIndex.java`) reads the same `.gz` file
using `ChunkReader` + `RecordExpander` and prints pipe-delimited artifact
records to stdout — the same information sluice emits as JSON Lines.

## What the correctness comparison validates

- **GAV coordinates** (groupId|artifactId|version) for every `ARTIFACT_ADD`
  record where classifier is `NA` (the default sluice output).
- Both outputs are sorted and diffed line-by-line.
- Total record counts (adds, removes, descriptor, groups) are printed to stderr
  by both tools for manual cross-check.

## What it does NOT validate

- **Extension field** — sluice reads extension from the raw UINFO (5th segment,
  often absent), while Java's `RecordExpander` derives it from the INFO field.
  This is by design: sluice intentionally skips field expansion.
- Field expansion beyond UINFO (e.g. `sha1`, `name`, `classnames`) — the Java
  `RecordExpander` populates many extra fields that sluice skips.
- Records with a real classifier (e.g. `sources`, `javadoc`) — sluice filters
  these out by default.

## Prerequisites

- Java 11+
- `jq` (for normalising sluice JSON Lines output)
- `hyperfine` (for throughput benchmark only)
- The full Maven Central index: `just fetch-full` (~2.8 GB download)

## Quick start

```bash
# 1. Download the indexer-reader JAR and compile the Java wrapper
just setup-bench

# 2. Correctness: diff sluice vs Java on the full index
just compare

# 3. Throughput: hyperfine comparison on the full index
just bench-java
```

You can also run on a smaller fixture for a quick smoke test:

```bash
just compare input=fixtures/chunk-sample.gz
just bench-java input=fixtures/chunk-sample.gz
```

## How it works

### Correctness (`scripts/compare-correctness.sh`)

1. Runs `sluice` on the input → pipes through `jq` to extract
   `group_id|artifact_id|version` → sorts.
2. Runs `DumpIndex` on the same input → filters to classifier=NA adds →
   extracts `g|a|v` with `awk` → sorts.
3. Diffs the two sorted files. Exit 0 = identical, exit 1 = differences found.

### Throughput (`scripts/bench-throughput.sh`)

Runs `hyperfine` with `--warmup 1 --min-runs 3` comparing:

- `target/release/sluice <input> > /dev/null`
- `java -Xmx4g -cp ... DumpIndex <input> > /dev/null`

Both tools' stdout is discarded so the benchmark measures parsing + gzip
decompression, not terminal I/O. JVM startup time is included — this reflects
real-world user experience.

## Results

Full Maven Central index (2.8 GB compressed, ~19.7M artifact records):

| Tool | Mean | Min | Max | Relative |
|:---|---:|---:|---:|---:|
| sluice (Rust) | 151.2s ± 7.9s | 142.1s | 156.3s | 1.00 |
| indexer-reader (Java) | 1112.1s ± 109.7s | 1033.7s | 1237.5s | 7.35 |

Sluice processes the full index in ~2.5 minutes vs ~18.5 minutes for Java.

## Interpreting the results

### The workloads are not identical

The 7.4x figure is a real-world comparison, not a controlled micro-benchmark.
The two tools do different amounts of work per record:

- **Sluice**: decompresses gzip → parses binary format → classifies records →
  parses UINFO (`u` field) into GAV coordinates → serializes to JSON → writes
  to stdout.
- **Java DumpIndex**: decompresses gzip → parses binary format → applies
  `RecordExpander` (parses the INFO field to derive ~20 expanded fields
  including `FILE_EXTENSION`, `SHA1`, `CLASSNAMES`, etc.) → formats
  pipe-delimited string → writes to stdout.

The Java `RecordExpander` step is nontrivial string processing that sluice
deliberately skips. This means part of Java's runtime is spent on work sluice
doesn't do. An apples-to-apples comparison where both tools did identical work
would show a **smaller** gap than 7.4x.

The comparison is still meaningful — it measures what each tool actually delivers
to the user end-to-end. But it should not be read as a pure "Rust vs Java
parsing speed" benchmark.

### Where the performance difference comes from

For streaming binary parsing workloads like this, a 3-7x difference between Rust
and Java is common. The main factors:

- **No GC pressure** — Java allocates a `Map<String, String>` and a `Record`
  object per document across ~47M documents (including records with classifiers).
  This creates significant garbage collection overhead. Sluice uses stack
  allocations and has minimal heap churn.
- **No JIT warmup** — Rust code is optimised at compile time. Java's C2 JIT
  compiler optimises hot loops at runtime, running on a separate thread. In
  `htop`, the Java process shows two threads at 100% CPU (application + JIT)
  while sluice uses a single thread. Per-CPU-core, sluice is roughly twice as
  efficient as the headline wall-clock ratio suggests.
- **Tighter data representation** — Rust structs are stack-allocated with no
  object headers or pointer indirection. Each Java object carries 12-16 bytes of
  header overhead, and fields are accessed through references rather than inline.

If the workload were more I/O-bound (network calls, database access), the gap
would be much smaller since both languages would spend most time waiting.
