# Maven Central index distribution and incremental updates

This document describes **how** the Maven Central index is distributed, how incremental updates work, and the protocol a well-behaved client should follow when keeping a local mirror in sync. For the **byte-level** format of the `.gz` index files themselves, see [`binary-format.md`](binary-format.md).

All facts here were verified against the live `https://repo1.maven.org/maven2/.index/` endpoint on 2026-04-11. Values like chunk numbers and timestamps will be stale by the time you read this; the *shape* of the data is what matters.

## 1. Distribution layout

Central publishes the index under a single directory on the repo HTTP server:

```
https://repo1.maven.org/maven2/.index/
  ├── nexus-maven-repository-index.gz              # full dump (~2.8 GB)
  ├── nexus-maven-repository-index.properties      # manifest / sidecar metadata
  ├── nexus-maven-repository-index.924.gz          # newest incremental chunk
  ├── nexus-maven-repository-index.923.gz
  ├── nexus-maven-repository-index.922.gz
  ├── ...
  └── nexus-maven-repository-index.895.gz          # oldest retained chunk
```

| File | Role |
|---|---|
| `nexus-maven-repository-index.gz` | **Full dump.** Complete state of every artifact on Central at publish time. |
| `nexus-maven-repository-index.<N>.gz` | **Incremental chunk** number `N`. Contains only the adds (`u`) and removes (`del`) since chunk `N-1`. |
| `nexus-maven-repository-index.properties` | **Manifest.** Plain Java `.properties` file listing the current chunk number, retained older chunks, and the chain-id. The authoritative source of the *current* state. |

Only the chunks listed in the properties file are guaranteed to exist on the server. Older numbered files have been deleted.

## 2. The properties file

Fetch the properties file *first*. It is small (<1 KB), unauthenticated, and tells you everything you need to decide what to do next. A verbatim snapshot:

```properties
#Wed Apr 08 12:52:23 UTC 2026
nexus.index.id=central
nexus.index.chain-id=1318453614498
nexus.index.timestamp=20260408120526.011 +0000
nexus.index.time=20120615133728.952 +0000
nexus.index.last-incremental=924
nexus.index.incremental-0=924    # newest retained
nexus.index.incremental-1=923
nexus.index.incremental-2=922
...
nexus.index.incremental-29=895   # oldest retained
```

| Key | Type | Meaning |
|---|---|---|
| `nexus.index.id` | string | Repository id. Always `central` for Maven Central. |
| `nexus.index.chain-id` | opaque numeric | **Lineage identifier.** See §3. |
| `nexus.index.timestamp` | datetime | When the *current* full + chunks were generated. Format: `yyyyMMddHHmmss.SSS ±ZZZZ`. |
| `nexus.index.time` | datetime | When the chain was *first* created (original lineage start). |
| `nexus.index.last-incremental` | int | Current head chunk number. |
| `nexus.index.incremental-<k>` | int | Chunk number retained at slot `k`, where `k=0` is newest and increases with age. Gives you the exact rolling window. |

**Retention window.** Central keeps 30 chunks (`incremental-0` … `incremental-29`). Anything older than `incremental-29` is gone. If your last-applied chunk number is lower than `incremental-29`, you cannot catch up incrementally and must redownload the full dump.

## 3. The chain-id

The `chain-id` identifies the **lineage** that the current full dump and chunks belong to.

- As long as the chain-id is stable, incremental chunks chain continuously off one another and off the full dump: chunk `N` is a valid delta on top of chunk `N-1`, which is valid on top of chunk `N-2`, all the way down to the full dump.
- If Central ever **rotates** the chain-id, the chain restarts: every incremental chunk that existed before the rotation becomes meaningless, and every consumer must redownload the full dump and restart their local chain from the new `last-incremental`.

**Stability in practice.** The chain-id observed on 2026-04-11 is `1318453614498`, and `nexus.index.time` shows the chain started on `2012-06-15`. So in ~14 years of Central operation the chain has **not** been rotated. Rotation is rare, but clients still need to handle it.

### The chain-id lives in the properties file, not in the `.gz`

Verified by decoding the `DESCRIPTOR` document inside the full dump byte-by-byte. The descriptor document contains exactly two fields:

```
DESCRIPTOR = "NexusIndex"   # literal marker string
IDXINFO    = "1.0|central"  # format version '1.0' and repository id 'central'
```

That's it. There is **no chain-id anywhere in the binary file**. The only in-band self-identification the binary carries is:

- The 9-byte file header: a format-version byte (`0x01`) and an `i64` millisecond timestamp.
- The `IDXINFO` field value: `1.0|central`.

Practical consequences:

1. **A `.gz` file alone does not tell you which chain it belongs to.** Chain membership is only derivable from the properties file fetched alongside it.
2. **Fetch the properties file alongside the binary and bind them together at download time.** Store both, or store the binary plus the chain-id that was current at fetch time.
3. **Chain rotation is visible only through `.properties`.** The binary itself does not change shape across a rotation.

## 4. The full dump

- Compressed size: ~2.8 GB (observed: 2,935,310,279 bytes).
- Decompressed size: ~30 GB (observed: 30,288,487,862 bytes).

Structural layout of the decompressed stream:

```
[9-byte header: version byte 0x01 + i64 millis timestamp]
[artifact add documents, ~hundreds of millions]
[DESCRIPTOR document (2 fields: DESCRIPTOR, IDXINFO)]
[allGroups document]
[rootGroups document]
[EOF — no sentinel, just stream end]
```

Two non-obvious points:

- **The DESCRIPTOR sits at roughly offset 30.3 GB**, ~99% of the way through the stream. A parser must read through the whole file to reach it.
- **Structural summary documents (`allGroups`, `rootGroups`) come *after* the descriptor**, at the tail. The `rootGroups` document's value is a flat pipe-separated list of every top-level groupId published on Central, which can be useful for coarse enumeration without parsing the whole binary.

## 5. Incremental chunks

Chunk files use the same format as the full dump's document stream: a 9-byte header followed by a sequence of documents. A chunk document is either:

- An **add** (`u` field present) → apply as "artifact `g:a:v[:c[:e]]` now exists".
- A **remove** (`del` field present) → apply as "artifact `g:a:v[:c[:e]]` no longer exists".
- Structural records (`DESCRIPTOR`, `allGroups`, `rootGroups`) were absent in every chunk sampled; observed chunks contained only adds and removes.

Chunks are numbered monotonically from chain start. Lower numbers are older. To replay, apply chunks in **ascending** order: `(stored+1), (stored+2), …, last-incremental`.

**Observed chunk sizes**: 5–18 MB compressed per chunk (sample of chunks 895–924). The specific chunk 924 I analyzed held 661,617 add records, of which 87,947 had `classifier=NA` (the main-GAV records — one per published version) and 573,670 were sibling file records (sources jars, javadocs, checksums, signatures).

## 6. Update cadence

**Full dump and chunks are all republished together in a single atomic operation.** Every file in `.index/` gets a fresh `Last-Modified` timestamp at each publish, even files whose *content* hasn't changed since the previous publish.

Observed HTTP headers (sampled 2026-04-11):

| File | Last-Modified | Size |
|---|---|---|
| `nexus-maven-repository-index.gz` | 2026-04-08 12:59:21 | 2,935 MB |
| `nexus-maven-repository-index.924.gz` | 2026-04-08 12:59:14 | 15 MB |
| `nexus-maven-repository-index.923.gz` | 2026-04-08 12:59:14 | 18 MB |
| `nexus-maven-repository-index.920.gz` | 2026-04-08 12:59:13 | 8 MB |
| `nexus-maven-repository-index.895.gz` | 2026-04-08 12:58:44 | 7 MB |

All republished within seconds of each other. **Do not use `Last-Modified` as a signal for "when was this chunk originally generated."** It only tells you "when was the most recent publish." To detect a new publish, compare `nexus.index.last-incremental` (or `nexus.index.timestamp`) in `.properties` against your stored value.

**Publish rate.** The observed chain started on 2012-06-15 and had reached chunk 924 by 2026-04-08 — about 924 chunks in ~13.8 years, or **~66 chunks per year, roughly one every 5–6 days**. With a 30-chunk retention window that gives you a safe catch-up window of roughly **5–6 months**.

> This rate is a multi-year average. To measure the current publish cadence, poll `.properties` daily for a few weeks and count how often `last-incremental` advances.

## 7. Recommended consumer protocol

A client that wants to maintain a fresh local mirror should run this loop on its desired polling cadence (daily is fine).

**Local state you need to persist**:

- `stored_chain_id`
- `stored_last_incremental`
- your accumulated artifact inventory

**Algorithm**:

```
1. Fetch nexus-maven-repository-index.properties.

2. If you have no local state:
      a. Download the full dump.
      b. Stream-parse it. Build your inventory from the add records.
      c. Store (chain-id, last-incremental) from the properties file
         fetched in step 1 as your local state.
      d. Exit.

3. If stored_chain_id != properties.chain-id:
      CHAIN ROTATED — discard everything. Download the full dump. Go to 2.

4. If properties.last-incremental == stored_last_incremental:
      No new data. Exit cleanly.

5. Compute missing = (stored_last_incremental + 1 .. properties.last-incremental).

6. If the lowest missing chunk < properties.incremental-29:
      GAP TOO LARGE — we fell off the retention window.
      Discard local state. Download the full dump. Go to 2.

7. For each chunk N in missing (ASCENDING order):
      a. Download nexus-maven-repository-index.<N>.gz.
      b. Stream-parse it.
      c. For each add  record: upsert into inventory.
         For each del  record: remove from inventory.
      d. Commit inventory changes + set stored_last_incremental = N.

8. Done.
```

Implementation notes:

- **Apply order matters.** Always ascending. A later chunk may remove an artifact that an earlier chunk added, or vice versa. Treating the chunks as a set rather than a sequence will give you the wrong final state.
- **Commit at chunk boundaries, not at the end.** If your consumer crashes halfway through chunk-replay, the next run picks up cleanly from `stored_last_incremental`.
- **Pair the chain-id with the binary.** At step 2c, persist `chain-id` *before* you claim the full dump was processed successfully. Pairing is what lets step 3 detect rotation.
- **`If-Modified-Since` and `If-None-Match` are safe shortcuts.** The server honors both. Your periodic poll of `.properties` can use them to skip the body when nothing changed — but still check `last-incremental` against stored, since a 304 on the properties file means nothing *at all* changed, not just "you are up to date".
- **Rate limits.** `repo1.maven.org` is fronted by a CDN with no documented client rate limit. Poll daily; the publish cadence is ~5 days, so anything more frequent buys you nothing.

## 8. Filtering guidance for your consumer

What you care about depends on what you are building. Two common shapes:

### "List every GAV ever published on Central"

Filter records to `classifier == NA` and keep only `group_id`, `artifact_id`, `version`. This drops the ~6–7× multiplier from sibling file records (sources jars, javadocs, checksums, signatures) and gives you exactly one row per published GAV. On chunk 924 this reduced 661,617 adds to 87,947 real GAVs.

Note: the `extension` field will always be null for these records. This is because the main-GAV record uses a 4-segment UINFO form (`groupId|artifactId|version|NA`) that has no extension slot, while sibling file records use the 5-segment form that does. See [`binary-format.md`](binary-format.md) for the byte-level details. Drop `extension` from your output — it carries no information.

### "Track packaging type per GAV"

The main-GAV record's UINFO does not carry the extension, but the record still has an `i` (INFO) field, which is a pipe-separated 7-tuple:

```
packaging | lastModified | size | sourcesExists | javadocExists | signatureExists | extension
```

`packaging` is the authoritative source for `jar` vs `pom` vs `war` vs `aar` etc., and does not require joining with sibling records. The current `sluice` CLI only exposes the `u` field; wiring `i` through is a small enhancement if you need packaging type.

### Removes

Even if you only want adds to build an initial inventory, you must process `del` (remove) records when replaying chunks, or your local state will diverge from Central's. Individual versions do get yanked occasionally (bad uploads, compromised keys, license issues).

## 9. Failure modes

| Symptom | Diagnosis | Response |
|---|---|---|
| `.properties` has different `chain-id` than stored | Chain rotated (rare — unprecedented as of 2026-04) | Re-fetch full dump |
| Your `stored_last_incremental` is below `incremental-29` in `.properties` | You fell off the retention window | Re-fetch full dump |
| A chunk returns 404 after being listed in `.properties` | Race with a republish in progress (very rare, sub-minute window) | Retry after a few seconds |
| Parser fails mid-chunk with `TruncatedDocument` | Corrupted download or server truncation | Retry the chunk |
| Parser sees a `del` record for an artifact you do not have | Normal — chunks may remove artifacts that existed before your local state began | Ignore silently |
| Parser sees a duplicate `add` for an artifact you already have | Normal — republishes and duplicate index entries occur | Upsert, do not error |
| WebFetch or curl with default user-agent returns 403 on `.index/` URLs | The CDN rejects some user-agent strings | Set an explicit `User-Agent` header |

## 10. What `sluice` already supports

As of the commit this document was written against:

- **Streaming parse** of full dumps and chunks via `IndexReader` — yes.
- **Classification** into `Descriptor`, `AllGroups`, `RootGroups`, `ArtifactAdd`, `ArtifactRemove`, `Unknown` — yes.
- **UINFO parsing** for both 4- and 5-segment forms — yes.
- **MUTF-8 decoding** including supplementary code points via surrogate pairs — yes.
- **State tracking / incremental replay loop** — no. The library is format-level only. A consumer implementing §7's protocol has to handle properties fetching, chain-id compare, chunk sequencing, and local state persistence itself.
- **`.properties` parser** — no. Trivial to add; the format is standard Java `.properties`, handleable with any KV parser.
- **`i` (INFO) field extraction into a structured type** — no. The raw `Field` and `Document` types already carry the bytes; the classification layer would need to expose a parsed form if you want typed packaging info.

## 11. References

- [`binary-format.md`](binary-format.md) — byte-level format of the `.gz` files.
- Apache Maven Indexer source — `IndexDataReader.java`, `IndexDataWriter.java`, `indexer-reader` module. These are the upstream authorities on the format.
- `crates/core/src/domain/record.rs` — this project's `Record::try_from(&Document)` impl, mirroring the upstream `Record.Type` enum.
- `scripts/fetch-chunk.sh` — downloads the latest incremental chunk by parsing `last-incremental` from `.properties`.
