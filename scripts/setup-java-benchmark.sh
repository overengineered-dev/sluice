#!/usr/bin/env bash
# Download the Maven indexer-reader JAR and compile the DumpIndex wrapper.
# Safe to re-run.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
JAVA_DIR="${ROOT}/scripts/java"
LIB_DIR="${JAVA_DIR}/lib"
JAR_VERSION="7.1.6"
JAR_NAME="indexer-reader-${JAR_VERSION}.jar"
JAR_URL="https://repo1.maven.org/maven2/org/apache/maven/indexer/indexer-reader/${JAR_VERSION}/${JAR_NAME}"

# --- Check Java ---
if ! command -v java &>/dev/null; then
    echo "error: java not found. Install Java 11+ to continue." >&2
    exit 1
fi

if ! command -v javac &>/dev/null; then
    echo "error: javac not found. Install a JDK (not just JRE) to continue." >&2
    exit 1
fi

JAVA_VER=$(java -version 2>&1 | head -1 | sed 's/.*"\([0-9]\+\)\..*/\1/')
if [[ "${JAVA_VER}" -lt 11 ]]; then
    echo "error: Java 11+ required (found ${JAVA_VER})" >&2
    exit 1
fi
echo "Found Java ${JAVA_VER}" >&2

# --- Download JAR ---
mkdir -p "${LIB_DIR}"

if [[ -f "${LIB_DIR}/${JAR_NAME}" ]]; then
    echo "JAR already present: ${LIB_DIR}/${JAR_NAME}" >&2
else
    echo "Downloading ${JAR_URL}" >&2
    curl -fsSL "${JAR_URL}" -o "${LIB_DIR}/${JAR_NAME}"
    echo "Saved ${LIB_DIR}/${JAR_NAME}" >&2
fi

# --- Compile ---
echo "Compiling DumpIndex.java" >&2
javac -cp "${LIB_DIR}/${JAR_NAME}" "${JAVA_DIR}/DumpIndex.java"

if [[ -f "${JAVA_DIR}/DumpIndex.class" ]]; then
    echo "Done. DumpIndex.class ready." >&2
else
    echo "error: compilation failed — DumpIndex.class not found" >&2
    exit 1
fi
