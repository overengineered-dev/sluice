//! Parse the committed chunk fixture end-to-end and assert structural
//! invariants. Snapshots the first 10 adds via `insta`.

use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use flate2::read::GzDecoder;
use sluice::{IndexReader, Record};

fn fixture_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("../../fixtures/chunk-sample.gz");
    p
}

#[test]
fn parses_committed_chunk() {
    let file =
        File::open(fixture_path()).expect("fixture missing; run `cargo run --example gen_fixture`");
    let gz = GzDecoder::new(BufReader::new(file));
    let reader = IndexReader::new(gz).expect("valid header");

    assert_eq!(reader.header().version, 0x01);
    assert!(
        reader.header().timestamp_millis.is_some(),
        "chunks from Maven Central always carry a timestamp"
    );

    let mut total = 0usize;
    let mut adds = 0usize;
    let mut removes = 0usize;
    let mut descriptor = 0usize;
    let mut all_groups = 0usize;
    let mut root_groups = 0usize;
    let mut unknown = 0usize;
    let mut na_adds = 0usize;
    let mut first_na_adds: Vec<String> = Vec::new();

    for doc in reader {
        let doc = doc.expect("no mid-stream parse errors");
        total += 1;

        match Record::try_from(&doc).expect("classification is infallible for well-formed fixtures")
        {
            Record::Descriptor => descriptor += 1,
            Record::AllGroups => all_groups += 1,
            Record::RootGroups => root_groups += 1,
            Record::ArtifactAdd(u) => {
                adds += 1;
                assert!(!u.group_id.is_empty());
                assert!(!u.artifact_id.is_empty());
                assert!(!u.version.is_empty());
                if u.classifier.is_none() {
                    na_adds += 1;
                    if first_na_adds.len() < 10 {
                        first_na_adds.push(format!(
                            "{} | {} | {} | NA | {}",
                            u.group_id,
                            u.artifact_id,
                            u.version,
                            u.extension.as_deref().unwrap_or("-")
                        ));
                    }
                }
            }
            Record::ArtifactRemove(u) => {
                removes += 1;
                assert!(!u.group_id.is_empty());
                assert!(!u.artifact_id.is_empty());
                assert!(!u.version.is_empty());
            }
            // `Record::Unknown` plus any future non-exhaustive variants.
            _ => unknown += 1,
        }
    }

    assert!(total > 0, "chunk should contain documents");
    assert!(adds > 0, "chunk should contain at least one add");
    assert!(
        na_adds > 0,
        "chunk should contain at least one NA-classifier add"
    );
    assert_eq!(
        total,
        adds + removes + descriptor + all_groups + root_groups + unknown,
        "record buckets should sum to total"
    );

    insta::assert_snapshot!("first_10_na_adds", first_na_adds.join("\n"));
}
