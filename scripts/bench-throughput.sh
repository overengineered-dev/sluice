#!/usr/bin/env bash
# Throughput comparison: sluice vs Java indexer-reader using hyperfine.
#
# Usage: ./scripts/bench-throughput.sh [path-to-index.gz]
# Default: fixtures/full/nexus-maven-repository-index.gz
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
INPUT="${1:-${ROOT}/fixtures/full/nexus-maven-repository-index.gz}"
JAVA_DIR="${ROOT}/scripts/java"

# --- Preflight checks ---
if [[ ! -f "${INPUT}" ]]; then
    echo "error: input file not found: ${INPUT}" >&2
    echo "  Run 'just fetch-full' to download the full Maven Central index." >&2
    exit 1
fi

if [[ ! -f "${JAVA_DIR}/DumpIndex.class" ]]; then
    echo "error: DumpIndex.class not found. Run 'just setup-bench' first." >&2
    exit 1
fi

if ! command -v hyperfine &>/dev/null; then
    echo "error: hyperfine not found." >&2
    echo "  Install: cargo install hyperfine / sudo apt install hyperfine / brew install hyperfine" >&2
    exit 1
fi

# --- Build sluice ---
echo "Building sluice (release)..." >&2
cargo build --release -p sluice-cli --manifest-path "${ROOT}/Cargo.toml" 2>&1 | tail -1

SLUICE="${ROOT}/target/release/sluice"

# --- Run hyperfine ---
echo "" >&2
echo "Benchmarking on: $(basename "${INPUT}")" >&2
echo "File size: $(du -h "${INPUT}" | cut -f1)" >&2
echo "" >&2

hyperfine \
    --warmup 1 \
    --min-runs 3 \
    --export-markdown /tmp/bench-results.md \
    --command-name "sluice (Rust)" \
    "${SLUICE} ${INPUT} > /dev/null" \
    --command-name "indexer-reader (Java)" \
    "java -Xmx4g -cp ${JAVA_DIR}:${JAVA_DIR}/lib/* DumpIndex ${INPUT} > /dev/null"

echo "" >&2
echo "Markdown results saved to /tmp/bench-results.md" >&2
cat /tmp/bench-results.md
