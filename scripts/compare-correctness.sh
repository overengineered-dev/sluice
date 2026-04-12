#!/usr/bin/env bash
# Compare sluice output against the Java indexer-reader for correctness.
#
# Usage: ./scripts/compare-correctness.sh [path-to-index.gz]
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

if ! command -v jq &>/dev/null; then
    echo "error: jq not found. Install jq to continue." >&2
    echo "  e.g.: sudo apt install jq / brew install jq" >&2
    exit 1
fi

# --- Build sluice ---
echo "Building sluice (release)..." >&2
cargo build --release -p sluice-cli --manifest-path "${ROOT}/Cargo.toml" 2>&1 | tail -1

SLUICE="${ROOT}/target/release/sluice"

TMP_SLUICE="$(mktemp)"
TMP_JAVA="$(mktemp)"
trap 'rm -f "${TMP_SLUICE}" "${TMP_JAVA}"' EXIT

# Comparison is on GAV (groupId|artifactId|version) only.
# Extension differs by design: sluice reads the raw UINFO (no expansion),
# while Java RecordExpander derives extension from the INFO field.

# --- Run sluice ---
echo "Running sluice on $(basename "${INPUT}")..." >&2
"${SLUICE}" "${INPUT}" 2>/dev/null \
    | jq -r '[.group_id, .artifact_id, .version] | join("|")' \
    | sort > "${TMP_SLUICE}"

SLUICE_COUNT=$(wc -l < "${TMP_SLUICE}")
echo "  sluice: ${SLUICE_COUNT} records (classifier=NA adds)" >&2

# --- Run Java ---
echo "Running Java DumpIndex on $(basename "${INPUT}")..." >&2
java -Xmx4g -cp "${JAVA_DIR}:${JAVA_DIR}/lib/*" DumpIndex "${INPUT}" 2>/dev/null \
    | grep -v '^DEL|' \
    | awk -F'|' '$4 == "NA" { print $1 "|" $2 "|" $3 }' \
    | sort > "${TMP_JAVA}"

JAVA_COUNT=$(wc -l < "${TMP_JAVA}")
echo "  java:   ${JAVA_COUNT} records (classifier=NA adds)" >&2

# --- Diff ---
echo "" >&2
if diff "${TMP_SLUICE}" "${TMP_JAVA}" > /dev/null 2>&1; then
    echo "PASS: outputs are identical (${SLUICE_COUNT} records)" >&2
    exit 0
else
    DIFF_COUNT=$(diff "${TMP_SLUICE}" "${TMP_JAVA}" | grep -c '^[<>]' || true)
    echo "FAIL: ${DIFF_COUNT} differing lines" >&2
    echo "" >&2
    echo "First 20 differences:" >&2
    diff "${TMP_SLUICE}" "${TMP_JAVA}" | head -40 >&2
    echo "" >&2
    echo "Full diff files:" >&2
    echo "  sluice: ${TMP_SLUICE}" >&2
    echo "  java:   ${TMP_JAVA}" >&2
    # Don't clean up temp files on failure so the user can inspect them
    trap '' EXIT
    exit 1
fi
