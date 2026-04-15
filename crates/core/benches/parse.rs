use std::fs;
use std::io::{BufReader, Cursor};
use std::path::PathBuf;

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use flate2::read::GzDecoder;
use sluice::{IndexReader, Record};

fn fixture_bytes() -> Vec<u8> {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("../../fixtures/chunk-sample.gz");
    fs::read(p).expect("fixture missing; run `just regen-fixture`")
}

fn decompressed_bytes(gz_bytes: &[u8]) -> Vec<u8> {
    use std::io::Read;
    let mut dec = GzDecoder::new(gz_bytes);
    let mut buf = Vec::new();
    dec.read_to_end(&mut buf).expect("decompression failed");
    buf
}

fn bench_parse_raw(c: &mut Criterion) {
    let gz_bytes = fixture_bytes();
    let raw = decompressed_bytes(&gz_bytes);

    let mut group = c.benchmark_group("parse");
    group.throughput(Throughput::Bytes(raw.len() as u64));

    group.bench_function("raw_stream", |b| {
        b.iter(|| {
            let reader = IndexReader::new(Cursor::new(&raw)).expect("valid header");
            let mut count = 0u64;
            for doc in reader {
                let _doc = doc.expect("parse ok");
                count += 1;
            }
            count
        });
    });

    group.bench_function("parse_and_classify", |b| {
        b.iter(|| {
            let reader = IndexReader::new(Cursor::new(&raw)).expect("valid header");
            let mut adds = 0u64;
            for doc in reader {
                let doc = doc.expect("parse ok");
                if let Ok(Record::ArtifactAdd(_)) = Record::try_from(&doc) {
                    adds += 1;
                }
            }
            adds
        });
    });

    group.bench_function("gzip_decompress_and_parse", |b| {
        b.iter(|| {
            let gz = GzDecoder::new(Cursor::new(&gz_bytes));
            let reader = IndexReader::new(BufReader::new(gz)).expect("valid header");
            let mut count = 0u64;
            for doc in reader {
                let _doc = doc.expect("parse ok");
                count += 1;
            }
            count
        });
    });

    group.finish();
}

criterion_group!(benches, bench_parse_raw);
criterion_main!(benches);
