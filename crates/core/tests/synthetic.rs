//! End-to-end synthetic test: hand-built byte stream → `IndexReader` →
//! classified records. Runs without network or gzip.

use std::io::Cursor;

use sluice::{classify, IndexReader, ParseError, Record};

fn push_name(buf: &mut Vec<u8>, name: &str) {
    let bytes = name.as_bytes();
    buf.extend_from_slice(&u16::try_from(bytes.len()).unwrap().to_be_bytes());
    buf.extend_from_slice(bytes);
}

fn push_value(buf: &mut Vec<u8>, value: &str) {
    let bytes = value.as_bytes();
    buf.extend_from_slice(&i32::try_from(bytes.len()).unwrap().to_be_bytes());
    buf.extend_from_slice(bytes);
}

fn push_field(buf: &mut Vec<u8>, flags: u8, name: &str, value: &str) {
    buf.push(flags);
    push_name(buf, name);
    push_value(buf, value);
}

fn push_document(buf: &mut Vec<u8>, fields: &[(u8, &str, &str)]) {
    buf.extend_from_slice(&i32::try_from(fields.len()).unwrap().to_be_bytes());
    for (flags, name, value) in fields {
        push_field(buf, *flags, name, value);
    }
}

fn build_stream(version: u8, ts: i64) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.push(version);
    buf.extend_from_slice(&ts.to_be_bytes());
    buf
}

#[test]
fn parses_header_and_five_document_stream() {
    let mut stream = build_stream(0x01, 1_700_000_000_000);

    // 1. descriptor
    push_document(
        &mut stream,
        &[
            (0x05, "DESCRIPTOR", "NexusIndex"),
            (0x05, "IDXINFO", "1.0|central"),
        ],
    );

    // 2. allGroups
    push_document(
        &mut stream,
        &[
            (0x01, "allGroups", "NA"),
            (0x04, "allGroupsList", "org.example|com.foo"),
        ],
    );

    // 3. rootGroups
    push_document(
        &mut stream,
        &[
            (0x01, "rootGroups", "NA"),
            (0x04, "rootGroupsList", "org|com"),
        ],
    );

    // 4. artifact add (NA classifier)
    push_document(
        &mut stream,
        &[
            (0x05, "u", "org.apache.maven|maven-core|3.9.6|NA|jar"),
            (0x04, "i", "jar|1700000000000|123|0|0|0|jar"),
            (0x04, "m", "1700000000000"),
        ],
    );

    // 5. artifact remove
    push_document(
        &mut stream,
        &[(0x05, "del", "org.example|legacy|1.0|NA|jar")],
    );

    let reader = IndexReader::new(Cursor::new(stream)).unwrap();
    assert_eq!(reader.header().version, 0x01);
    assert_eq!(reader.header().timestamp_millis, Some(1_700_000_000_000));

    let docs: Vec<_> = reader.collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(docs.len(), 5);

    let records: Vec<Record> = docs.iter().map(|d| classify(d).unwrap()).collect();
    assert!(matches!(records[0], Record::Descriptor));
    assert!(matches!(records[1], Record::AllGroups));
    assert!(matches!(records[2], Record::RootGroups));

    let Record::ArtifactAdd(ref uinfo) = records[3] else {
        panic!("expected ArtifactAdd");
    };
    assert_eq!(uinfo.group_id, "org.apache.maven");
    assert_eq!(uinfo.artifact_id, "maven-core");
    assert_eq!(uinfo.version, "3.9.6");
    assert_eq!(uinfo.classifier, None);
    assert_eq!(uinfo.extension.as_deref(), Some("jar"));

    let Record::ArtifactRemove(ref uinfo) = records[4] else {
        panic!("expected ArtifactRemove");
    };
    assert_eq!(uinfo.group_id, "org.example");
    assert_eq!(uinfo.artifact_id, "legacy");
}

#[test]
fn four_segment_uinfo_backfills_extension_from_info() {
    let mut stream = build_stream(0x01, 1_700_000_000_000);

    // artifact add with 4-segment UINFO (no extension) + INFO field
    push_document(
        &mut stream,
        &[
            (0x05, "u", "org.example|lib|2.0|NA"),
            (0x04, "i", "jar|1700000000000|456|1|1|0|jar"),
            (0x04, "m", "1700000000000"),
        ],
    );

    let reader = IndexReader::new(Cursor::new(stream)).unwrap();
    let docs: Vec<_> = reader.collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(docs.len(), 1);

    let Record::ArtifactAdd(ref uinfo) = classify(&docs[0]).unwrap() else {
        panic!("expected ArtifactAdd");
    };
    assert_eq!(uinfo.group_id, "org.example");
    assert_eq!(uinfo.version, "2.0");
    assert_eq!(uinfo.classifier, None);
    assert_eq!(uinfo.extension.as_deref(), Some("jar"));
}

#[test]
fn five_segment_uinfo_ignores_info_extension() {
    let mut stream = build_stream(0x01, 1_700_000_000_000);

    // artifact add with 5-segment UINFO (extension=war) + INFO field (extension=jar)
    push_document(
        &mut stream,
        &[
            (0x05, "u", "org.example|webapp|1.0|NA|war"),
            (0x04, "i", "war|1700000000000|789|0|0|0|jar"),
        ],
    );

    let reader = IndexReader::new(Cursor::new(stream)).unwrap();
    let docs: Vec<_> = reader.collect::<Result<Vec<_>, _>>().unwrap();

    let Record::ArtifactAdd(ref uinfo) = classify(&docs[0]).unwrap() else {
        panic!("expected ArtifactAdd");
    };
    // UINFO extension takes precedence over INFO
    assert_eq!(uinfo.extension.as_deref(), Some("war"));
}

#[test]
fn timestamp_negative_one_maps_to_none() {
    let stream = build_stream(0x01, -1);
    let reader = IndexReader::new(Cursor::new(stream)).unwrap();
    assert_eq!(reader.header().timestamp_millis, None);
}

#[test]
fn rejects_unsupported_version() {
    let stream = build_stream(0x02, 0);
    match IndexReader::new(Cursor::new(stream)) {
        Err(ParseError::UnsupportedVersion(0x02)) => {}
        Err(other) => panic!("wrong error: {other:?}"),
        Ok(_) => panic!("should have rejected version 0x02"),
    }
}

#[test]
fn clean_eof_after_header_terminates_iterator() {
    let stream = build_stream(0x01, 0);
    let mut reader = IndexReader::new(Cursor::new(stream)).unwrap();
    assert!(reader.next().is_none());
}

#[test]
fn truncated_field_count_is_reported() {
    let mut stream = build_stream(0x01, 0);
    stream.extend_from_slice(&[0x00, 0x00]); // only 2 bytes of field count
    let mut reader = IndexReader::new(Cursor::new(stream)).unwrap();
    let item = reader.next().unwrap();
    assert!(matches!(
        item,
        Err(ParseError::TruncatedDocument { bytes_into_doc: 2 })
    ));
}

#[test]
fn truncated_mid_field_is_io_error() {
    let mut stream = build_stream(0x01, 0);
    // Field count = 1, then an incomplete field (just the flags byte).
    stream.extend_from_slice(&1i32.to_be_bytes());
    stream.push(0x07);
    let mut reader = IndexReader::new(Cursor::new(stream)).unwrap();
    let err = reader.next().unwrap().unwrap_err();
    assert!(matches!(err, ParseError::Io(_)));
}

#[test]
fn document_with_zero_fields_is_legal() {
    let mut stream = build_stream(0x01, 0);
    stream.extend_from_slice(&0i32.to_be_bytes());
    let reader = IndexReader::new(Cursor::new(stream)).unwrap();
    let docs: Vec<_> = reader.collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(docs.len(), 1);
    assert!(docs[0].fields.is_empty());
}
