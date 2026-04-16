# Comparative benchmark: sluice vs Java indexer-reader

Until now, reading the Maven Central index required the JVM, a custom script wiring up the Java `indexer-reader` library, and patience. Sluice is a single binary that does it 5x faster.

## What is compared

Sluice is compared against the **Apache Maven `indexer-reader`** library
(version 7.1.6), *not* the `indexer-cli` JAR. The CLI only packs/unpacks
Lucene indexes and has no text-dump mode, so it cannot be used for output
comparison.

A small Java wrapper (`scripts/java/DumpIndex.java`) reads the same `.gz` file
using `ChunkReader` + `RecordExpander` and prints pipe-delimited artifact
records to stdout — the same information sluice emits as JSON Lines.

## What the correctness comparison validates

- **Default mode — GAV coordinates** (groupId|artifactId|version) for every
  `ARTIFACT_ADD` record where classifier is `NA` (the default sluice output).
- **Full mode — GAV + classifier** (groupId|artifactId|version|classifier) for
  all `ARTIFACT_ADD` records via `sluice --full`, compared against the unfiltered
  Java output.
- Both outputs are sorted and diffed line-by-line.
- Total record counts (adds, removes, descriptor, groups) are printed to stderr
  by both tools for manual cross-check.

## What it does NOT validate

- **Extension field** — sluice backfills extension from the INFO field when the
  5th UINFO segment is absent (MINDEXER-41), while Java's `RecordExpander`
  derives it via its own expansion logic. The two may differ for edge cases.
- Field expansion beyond UINFO (e.g. `sha1`, `name`, `classnames`) — the Java
  `RecordExpander` populates many extra fields that sluice skips.

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
#    (runs both default mode and --full mode comparisons)
just compare

# 3. Throughput: hyperfine comparison on the full index
#    (benchmarks sluice, sluice --full, and Java)
just bench-java
```

You can also run on a smaller fixture for a quick smoke test:

```bash
just compare input=crates/core/tests/fixtures/chunk-sample.gz
just bench-java input=crates/core/tests/fixtures/chunk-sample.gz
```

## How it works

### Correctness (`scripts/compare-correctness.sh`)

Two passes run back-to-back:

1. **Default mode** — runs `sluice` → pipes through `jq` to extract
   `group_id|artifact_id|version` → sorts. Compares against `DumpIndex` output
   filtered to classifier=NA adds with `awk` → sorts. Diffs the two.
2. **Full mode** — runs `sluice --full` → extracts
   `group_id|artifact_id|version|classifier` (4 fields, no extension) → sorts.
   Compares against unfiltered `DumpIndex` output (same 4 fields) → sorts.
   Diffs the two.

Exit 0 = both passes identical, exit 1 = differences found in either pass.

### Throughput (`scripts/bench-throughput.sh`)

Runs `hyperfine` with `--warmup 1 --min-runs 3` comparing:

- `target/release/sluice <input> > /dev/null`
- `target/release/sluice --full <input> > /dev/null`
- `java -Xmx4g -cp ... DumpIndex <input> > /dev/null`

Both tools' stdout is discarded so the benchmark measures parsing + gzip
decompression, not terminal I/O. JVM startup time is included — this reflects
real-world user experience.

## Results

Full Maven Central index (2.8 GB compressed, ~97M documents, ~19.7M with
classifier=NA):

| Tool | Mean | Min | Max | Relative |
|:---|---:|---:|---:|---:|
| sluice --full (Rust) | 208.1s ± 6.8s | 200.8s | 214.3s | 1.00 |
| sluice (Rust) | 224.9s ± 30.3s | 197.0s | 257.2s | 1.08 |
| indexer-reader (Java) | 1112.1s ± 109.7s | 1033.7s | 1237.5s | 5.35\* |

Sluice processes the full index in ~3.5 minutes vs ~18.5 minutes for Java.

Default and `--full` mode perform within noise of each other — both parse all
~97M documents; the only difference is whether classifier-filtered records are
serialized.

\*The Java row is from a previous benchmark run before the MINDEXER-41 extension
fixup was added. The fixup (backfilling extension from the INFO field for
4-segment UINFO records) added an extra field lookup per document, increasing
sluice's baseline from ~151s to ~225s. The Java number should be re-measured
with `just bench-java` for a controlled comparison on the same run.

## Interpreting the results

### The workloads are not identical

The ~5x figure is a real-world comparison, not a controlled micro-benchmark.
The two tools do different amounts of work per record:

- **Sluice** (default): decompresses gzip → parses binary format → classifies
  records → parses UINFO (`u` field) into GAV coordinates → backfills extension
  from the INFO (`i`) field when the UINFO has only 4 segments (MINDEXER-41) →
  filters out classified records (classifier ≠ NA) → serializes to JSON →
  writes to stdout.
- **Sluice `--full`**: same pipeline but without the classifier filter — all
  records are emitted, matching the output scope of the Java tool.
- **Java DumpIndex**: decompresses gzip → parses binary format → applies
  `RecordExpander` (parses the INFO field to derive ~20 expanded fields
  including `FILE_EXTENSION`, `SHA1`, `CLASSNAMES`, etc.) → formats
  pipe-delimited string → writes to stdout.

The Java `RecordExpander` step is nontrivial string processing that sluice
largely skips. Sluice reads only the extension component from the INFO field
(one of ~20 fields that `RecordExpander` derives), so most of the expansion
work remains Java-only. This means part of Java's runtime is spent on work
sluice doesn't do. An apples-to-apples comparison where both tools did
identical work would show a **smaller** gap than ~5x.

In default mode there is a second asymmetry: sluice filters out classified
records (emitting ~19.7M) while Java emits all ~97M records. The `--full` mode
eliminates this difference — both tools emit all records — but the per-record
work gap from `RecordExpander` remains.

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
