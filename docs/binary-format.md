# Maven Central index binary format

The Maven Central index (`nexus-maven-repository-index.gz` and its incremental chunks) uses a custom binary transport format, not a raw Lucene index. It is documented only in the Apache Maven Indexer source — `IndexDataReader.java`, `IndexDataWriter.java`, and the `indexer-reader` module. This document is a byte-level specification sufficient to implement a parser, distilled from those sources for the benefit of `sluice`.

## 1. Framing

After GZIP decompression, the stream is a big-endian binary sequence following Java `DataInputStream` conventions. The entire format is:

```
[GZIP wrapper]
  ┌─ Header (9 bytes) ───────────────────────────────────┐
  │ [1 byte]  version = 0x01                             │
  │ [8 bytes] timestamp (i64 BE, millis since epoch)     │
  └──────────────────────────────────────────────────────┘
  ┌─ Document 0 ─────────────────────────────────────────┐
  │ [4 bytes] field_count (i32 BE)                       │
  │ ┌─ Field 0 ───────────────────────────────────────┐  │
  │ │ [1 byte]  flags (bit0=IDX, bit1=TOK, bit2=STR)  │  │
  │ │ [2 bytes] name_len (u16 BE, MUTF-8 length)       │  │
  │ │ [N bytes] name_bytes (Java Modified UTF-8)       │  │
  │ │ [4 bytes] value_len (i32 BE, MUTF-8 length)      │  │
  │ │ [M bytes] value_bytes (Java Modified UTF-8)      │  │
  │ └─────────────────────────────────────────────────┘  │
  │ ... (repeat for field_count fields)                  │
  └──────────────────────────────────────────────────────┘
  ... (repeat documents until EOF)
  [EOF — no sentinel; the stream simply ends]
```

The reader auto-detects GZIP by probing the first two bytes for the magic `0x1F 0x8B` before decompression (MINDEXER-13), so both wrapped and raw streams are handled.

## 2. Header (9 bytes)

| Offset | Size | Type | Description |
|--------|------|------|-------------|
| `0` | 1 byte | `u8` | Version byte. **Always `0x01`.** Any other value is rejected. |
| `1` | 8 bytes | `i64` BE | Timestamp in milliseconds since the Unix epoch. `-1` means "no timestamp". |

## 3. Document stream

Documents are written back-to-back with no separators and no outer length prefix. Each document is self-framing:

| Component | Size | Type | Description |
|-----------|------|------|-------------|
| `field_count` | 4 bytes | `i32` BE | Number of fields in this document |
| `fields` | variable | repeated | Exactly `field_count` entries |

**End-of-stream detection.** There is no sentinel. The parser reads the next 4-byte `field_count`; EOF on that read is clean termination. EOF anywhere else is a protocol error.

## 4. Field encoding

Each field is:

| Component | Size | Encoding | Description |
|-----------|------|----------|-------------|
| `flags` | 1 byte | `u8` bitfield | `0x01` = INDEXED, `0x02` = TOKENIZED, `0x04` = STORED |
| `name` | 2 + N bytes | Java `DataInput.readUTF()` | **2-byte** unsigned big-endian length prefix, then N bytes of Java Modified UTF-8 |
| `value` | 4 + M bytes | Custom `readUTF()` | **4-byte** signed big-endian length prefix, then M bytes of Java Modified UTF-8 |

**Asymmetric length prefixes.** Field *names* use Java's standard `readUTF()` with a 2-byte length prefix (max 65,535 bytes). Field *values* use a custom reimplementation with a 4-byte length prefix (max ~2 GB). The wider prefix (MINDEXER-28) was added for class-name lists in JAR content indexes, which routinely exceed 64 KB.

### Flag combinations

The flag byte uses the three bits defined as `F_INDEXED`, `F_TOKENIZED`, and `F_STORED` in `IndexDataWriter`:

- `0x01` — indexed only (keyword/searchable, not retrievable)
- `0x04` — stored only
- `0x05` — indexed keyword + stored
- `0x07` — indexed + tokenized + stored (full-text searchable and retrievable)

Higher bits are unused.

### Java Modified UTF-8 (MUTF-8)

Both names and values use **Java Modified UTF-8**, which diverges from standard UTF-8 in two ways:

1. **`U+0000` is encoded as `0xC0 0x80`**, never as `0x00`. The byte `0x00` therefore never appears in the encoded data.
2. **Supplementary characters (≥ `U+10000`)** are encoded as a UTF-16 *surrogate pair*, with each surrogate half independently encoded as a 3-byte sequence — **6 bytes total**, instead of standard UTF-8's 4-byte form. A decoder must combine the surrogates back into the scalar value.

Byte-level table:

| Code point range | Byte pattern | Notes |
|------------------|--------------|-------|
| `U+0001`–`U+007F` | `0xxxxxxx` (1 byte) | ASCII; `U+0000` is excluded |
| `U+0000`, `U+0080`–`U+07FF` | `110xxxxx 10xxxxxx` (2 bytes) | Null char maps to `0xC0 0x80` |
| `U+0800`–`U+FFFF` | `1110xxxx 10xxxxxx 10xxxxxx` (3 bytes) | Surrogate halves `D800`–`DFFF` included |

`sluice` delegates MUTF-8 decoding to the [`cesu8`](https://crates.io/crates/cesu8) crate.

## 5. Field catalogue

The indexer uses a pluggable `IndexCreator` architecture. Four creators ship with Maven Indexer; between them they define every field key a parser may see.

### Core artifact fields — `MinimalArtifactInfoIndexCreator` (id: `min`)

| Key | Name | Flags | Content |
|-----|------|-------|---------|
| `u` | UINFO | indexed + stored | **Primary key.** Pipe-delimited: `groupId\|artifactId\|version\|classifier\|extension`. Classifier is `NA` when absent. |
| `i` | INFO | stored | Pipe-delimited: `packaging\|lastModified\|size\|sourcesExists\|javadocExists\|signatureExists\|extension`. Availability values: `0` = not available, `1` = present, `2` = not present. `size` is in bytes (`-1` if unknown). |
| `m` | LAST_MODIFIED | stored | File modification timestamp in UTC milliseconds (decimal string). |
| `1` | SHA1 | indexed + stored | 40-char hex SHA-1 digest. Optional (only when `.sha1` exists). |
| `n` | NAME | indexed + tokenized + stored | POM `<name>`. Optional. |
| `d` | DESCRIPTION | indexed + tokenized + stored | POM `<description>`. Optional. |
| `g` | GROUP_ID | indexed keyword | GroupId as untokenized keyword. Not stored. |
| `groupId` | GROUP_ID (v3) | indexed + tokenized | Same groupId, tokenized. Not stored. |
| `a` | ARTIFACT_ID | indexed keyword | ArtifactId as keyword. Not stored. |
| `artifactId` | ARTIFACT_ID (v3) | indexed + tokenized | Same, tokenized. Not stored. |
| `v` | VERSION | indexed keyword | Version string as keyword. Not stored. |
| `version` | VERSION (v3) | indexed + tokenized | Same, tokenized. Not stored. |
| `p` | PACKAGING | indexed keyword | `jar`, `pom`, `war`, `maven-plugin`, … Optional, not stored. |
| `e` | EXTENSION | indexed keyword | File extension. Not stored. |
| `l` | CLASSIFIER | indexed keyword | `sources`, `javadoc`, `tests`, … Optional, not stored. |
| `del` | DELETED | indexed + stored | **Only in ARTIFACT_REMOVE records.** Contains the UINFO value of the deleted artifact. Mutually exclusive with `u`. |

#### The `u` (UINFO) field

```
groupId | artifactId | version | nvl(classifier) | extension
```

`nvl()` converts a null classifier to the literal string `NA`. In pre-5.x indexes the 5th segment was sometimes absent; see the fixup note in §8.

#### The `i` (INFO) field

Split on `|` into exactly 7 components: `packaging`, `lastModified`, `size`, `sourcesExists`, `javadocExists`, `signatureExists`, `extension`. `packaging` may be `NA` when null. In older indexes the 7th component is missing and defaults to `jar`.

### JAR content — `JarFileContentsIndexCreator` (id: `jarContent`)

| Key | Flags | Content |
|-----|-------|---------|
| `c` | indexed + tokenized + stored | Newline-separated (`\n`) list of fully-qualified class names with `/` as the package separator (e.g. `/org/apache/maven/Main`). JARs/WARs only. |
| `classnames` | indexed keyword | Same content as `c`, kept for backward compatibility. |

**Caveat (MINDEXER-225).** The `indexer-reader` module's `RecordExpander` uses `|` as the classnames separator instead of `\n`. If you ever port that path, be aware of the discrepancy.

### Maven plugin — `MavenPluginArtifactInfoIndexCreator` (id: `maven-plugin`)

Present only for `packaging=maven-plugin`:

| Key | Flags | Content |
|-----|-------|---------|
| `px` | indexed + stored | Plugin goal prefix (e.g. `compiler`, `surefire`). |
| `gx` | indexed + tokenized + stored | Pipe-delimited list of plugin goals (e.g. `compile\|testCompile`). |

### OSGi bundle — `OsgiArtifactIndexCreator` (id: `osgi-metadatas`)

Extracted from `META-INF/MANIFEST.MF`. All optional, all indexed + tokenized + stored. Field keys are the **literal** manifest header names:

`Bundle-SymbolicName`, `Bundle-Version`, `Export-Package`, `Export-Service` (deprecated), `Bundle-Description`, `Bundle-Name`, `Bundle-License`, `Bundle-DocURL`, `Import-Package`, `Require-Bundle`, `Provide-Capability`, `Require-Capability`, `Fragment-Host`, `Bundle-RequiredExecutionEnvironment` (deprecated), and `sha256` (SHA-256 hex, computed only for OSGi bundles).

## 6. Infrastructure records

Three non-artifact document types are interleaved with artifact records:

- **Descriptor document** — contains `DESCRIPTOR` and `IDXINFO` (repository id, etc.). One per index. Detect by the presence of the `DESCRIPTOR` field.
- **All-groups document** — contains `allGroups` and `allGroupsList` (pipe-delimited list of every `groupId` in the repository).
- **Root-groups document** — contains `rootGroups` and `rootGroupsList` (pipe-delimited list of top-level group segments).

The `indexer-reader` module's `Record.Type` enum classifies documents as `DESCRIPTOR`, `ALL_GROUPS`, `ROOT_GROUPS`, `ARTIFACT_ADD`, or `ARTIFACT_REMOVE`. `sluice` mirrors this in [`Record`](../crates/core/src/domain/record.rs).

## 7. What is NOT in the index

Dependencies, parent POM references, SCM URLs, POM `<licenses>`, developers, organization, build config, properties, distribution management, modules, issue tracking, and CI info are absent. The only license-adjacent data is the OSGi `Bundle-License` manifest header, which differs from POM `<licenses>`. For dependency trees, parent relationships, or license metadata, fetch and parse the individual POM files.

## 8. Implementation notes for Rust parsers

- **GZIP.** Use `flate2::read::GzDecoder`. Maven Central always serves gzipped files.
- **Big-endian reads.** `u8::from_be_bytes`, `u16::from_be_bytes` (name length), `i32::from_be_bytes` (field count, value length), `i64::from_be_bytes` (header timestamp).
- **MUTF-8.** `std::str::from_utf8` will not work: it rejects both `0xC0 0x80` and surrogate halves. Use the [`cesu8`](https://crates.io/crates/cesu8) or [`mutf8`](https://crates.io/crates/mutf8) crate, or implement the ~40-line decoder from `IndexDataReader.readUTF()`.
- **EOF.** A clean `UnexpectedEof` on the 4-byte field-count read means the stream has ended normally. EOF anywhere else is a protocol error.
- **UINFO fixup (MINDEXER-41).** If the `u` field has fewer than 5 pipe-separated segments, append the extension taken from the `i` field. This preserves compatibility with older indexes that omitted the 5th segment.
- **Memory.** The `c` (classnames) field can be megabytes per document. The 4-byte value length prefix permits up to ~2 GB. Allocate value buffers on the heap and bound them defensively against corrupt length values (MINDEXER-28).

## 9. Incremental update protocol

Maven Central publishes numbered incremental chunks so clients don't have to re-download the full index (~1.5–2 GB compressed) on every sync. The protocol is driven by a `.properties` file plus numbered `.gz` chunks — each chunk uses the same binary format described above.

### The properties file

At `https://repo1.maven.org/maven2/.index/nexus-maven-repository-index.properties`. It is a standard Java properties file; constants come from `IndexingContext.java`:

| Property key | Example | Meaning |
|--------------|---------|---------|
| `nexus.index.id` | `central` | Repository identifier |
| `nexus.index.chain-id` | `1243533418968` | UUID or timestamp identifying the current incremental chain. **If this value differs from the locally stored one, a full reindex is required.** |
| `nexus.index.timestamp` | `20090528175658.015 +0000` | When the index was last published (`yyyyMMddHHmmss.SSS Z`) |
| `nexus.index.time` | (same format) | Legacy timestamp for the deprecated `.zip` format |
| `nexus.index.last-incremental` | `168` | The highest chunk number currently available |
| `nexus.index.incremental-0` | `161` | The *oldest* retained chunk number |
| `nexus.index.incremental-N` | … | All retained chunks in ascending order |

As of mid-2025, Maven Central retains roughly 8 chunks on a weekly cadence. Individual chunks are typically tens of kilobytes compared to the multi-gigabyte full index.

### Update algorithm

From `DefaultIndexUpdater.java`:

1. Download the remote `.properties` file.
2. Compare `chain-id` with the locally stored value. If different → full download required.
3. Compare `last-incremental` with the local value:
   - equal → already current, nothing to do;
   - local `last-incremental` < remote `incremental-0` (oldest available) → client has fallen too far behind, the needed chunks have been purged → full download required;
   - otherwise → download every `nexus-maven-repository-index.{X}.gz` where `X` is greater than the local `last-incremental` and appears in the remote `incremental-N` entries.
4. Apply chunks **in order, lowest number first.** Chunks are sequential, not cumulative.
5. After a successful apply, update the local properties with the remote values.

A first-time client with no local state always performs a full download of `nexus-maven-repository-index.gz`.

### Add vs. delete records in a chunk

Chunks use the same binary format as the full index. Addition/update records carry a `u` field plus the standard artifact metadata fields; an existing document with the same UINFO is replaced on merge. Deletion records carry a `del` field whose value is the UINFO of the artifact to remove, with no other payload.

Parsers distinguish the two by presence of `u` versus `del`. This mirrors the `RecordExpander` in `indexer-reader`, which assigns `Record.Type.ARTIFACT_ADD` vs `Record.Type.ARTIFACT_REMOVE` the same way.

### Apply semantics

- **Full index** (`nexus-maven-repository-index.gz`): applied via `context.replace()` — *replaces* the entire local index.
- **Incremental chunks** (`.N.gz`): applied via `context.merge()` — merges additions and applies deletions against the existing index.

### File locations on Maven Central

```
https://repo1.maven.org/maven2/.index/nexus-maven-repository-index.properties
https://repo1.maven.org/maven2/.index/nexus-maven-repository-index.gz
https://repo1.maven.org/maven2/.index/nexus-maven-repository-index.gz.sha1
https://repo1.maven.org/maven2/.index/nexus-maven-repository-index.{N}.gz
https://repo1.maven.org/maven2/.index/nexus-maven-repository-index.{N}.gz.sha1
https://repo1.maven.org/maven2/.index/nexus-maven-repository-index.{N}.gz.md5
```

For a comprehensive guide to the distribution layout, consumer protocol, and failure modes, see [`incremental-updates.md`](incremental-updates.md).

## 10. Summary

The transport format is a self-framing binary protocol with one version, `0x01`. Parsing loop: read the 9-byte header, then repeatedly read documents (4-byte field count + N fields of `flags + name + value`) until EOF. Two non-standard details: name lengths use a 2-byte prefix while value lengths use 4-byte, and strings are encoded as Java Modified UTF-8 rather than UTF-8. For incremental sync, fetch `.properties`, check the chain-id, download the missing numbered chunks, and merge them in ascending order. Adds carry a `u` field; deletes carry `del`.
