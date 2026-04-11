#!/usr/bin/env bash
# Download the latest Maven Central incremental index chunk into
# fixtures/chunk-latest.gz. Safe to re-run.
set -euo pipefail

BASE_URL="https://repo1.maven.org/maven2/.index"
PROPS_URL="${BASE_URL}/nexus-maven-repository-index.properties"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT_DIR="${ROOT}/fixtures"
OUT_FILE="${OUT_DIR}/chunk-latest.gz"

mkdir -p "${OUT_DIR}"

TMP_PROPS="$(mktemp)"
trap 'rm -f "${TMP_PROPS}"' EXIT

echo "Fetching ${PROPS_URL}" >&2
curl -fsSL "${PROPS_URL}" -o "${TMP_PROPS}"

LAST=$(grep '^nexus.index.last-incremental=' "${TMP_PROPS}" | cut -d= -f2 | tr -d '\r')
if [[ -z "${LAST}" ]]; then
    echo "error: could not read nexus.index.last-incremental from properties" >&2
    exit 1
fi

CHUNK_URL="${BASE_URL}/nexus-maven-repository-index.${LAST}.gz"
echo "Downloading chunk ${LAST} → ${OUT_FILE}" >&2
curl -fsSL "${CHUNK_URL}" -o "${OUT_FILE}"

SIZE=$(stat -c %s "${OUT_FILE}")
echo "Wrote ${OUT_FILE} (${SIZE} bytes)" >&2
