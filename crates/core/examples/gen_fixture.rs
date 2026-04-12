//! Reads a full Maven Central index chunk and writes a trimmed version
//! containing only the first N documents, with oversized fields (like
//! classnames) truncated. Output is gzipped.
//!
//! Usage:
//!
//! ```sh
//! cargo run --example gen_fixture -- fixtures/chunk-latest.gz fixtures/chunk-sample.gz [max_docs]
//! ```

use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use sluice::{Document, Field, IndexReader};

/// Max byte length for any single field value in the output.
const MAX_VALUE_LEN: usize = 4096;

fn write_header(w: &mut impl Write, version: u8, timestamp: Option<i64>) -> std::io::Result<()> {
    w.write_all(&[version])?;
    let ts = timestamp.unwrap_or(-1);
    w.write_all(&ts.to_be_bytes())
}

fn write_field(w: &mut impl Write, field: &Field) -> std::io::Result<()> {
    w.write_all(&[field.flags.bits()])?;
    // Name: 2-byte length prefix (MUTF-8, but for ASCII names this is fine)
    let name_bytes = field.name.as_bytes();
    let name_len = u16::try_from(name_bytes.len()).expect("field name too long");
    w.write_all(&name_len.to_be_bytes())?;
    w.write_all(name_bytes)?;
    // Value: 4-byte length prefix
    let value_bytes = field.value.as_bytes();
    let value_len = i32::try_from(value_bytes.len()).expect("field value too long for i32");
    w.write_all(&value_len.to_be_bytes())?;
    w.write_all(value_bytes)?;
    Ok(())
}

fn write_document(w: &mut impl Write, doc: &Document) -> std::io::Result<()> {
    let field_count = i32::try_from(doc.fields.len()).expect("too many fields");
    w.write_all(&field_count.to_be_bytes())?;
    for field in &doc.fields {
        write_field(w, field)?;
    }
    Ok(())
}

fn truncate_large_fields(doc: &mut Document) {
    for field in &mut doc.fields {
        if field.value.len() > MAX_VALUE_LEN {
            // Truncate at a char boundary
            let mut end = MAX_VALUE_LEN;
            while end > 0 && !field.value.is_char_boundary(end) {
                end -= 1;
            }
            field.value.truncate(end);
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: gen_fixture <input.gz> <output.gz> [max_docs]");
        std::process::exit(1);
    }
    let input_path = &args[1];
    let output_path = &args[2];
    let max_docs: usize = args
        .get(3)
        .map_or(500, |s| s.parse().expect("max_docs must be a number"));

    let input = File::open(input_path).expect("cannot open input");
    let gz_in = GzDecoder::new(BufReader::new(input));
    let reader = IndexReader::new(gz_in).expect("invalid header");

    let header = *reader.header();

    let output = File::create(output_path).expect("cannot create output");
    let gz_out = GzEncoder::new(BufWriter::new(output), Compression::best());
    let mut writer = BufWriter::new(gz_out);

    write_header(&mut writer, header.version, header.timestamp_millis).expect("write header");

    let mut count = 0usize;
    for doc_result in reader {
        if count >= max_docs {
            break;
        }
        let mut doc = doc_result.expect("parse error in input");
        truncate_large_fields(&mut doc);
        write_document(&mut writer, &doc).expect("write document");
        count += 1;
    }

    let gz_out = writer.into_inner().expect("flush");
    gz_out.finish().expect("finalize gzip");

    eprintln!("Wrote {count} documents to {output_path}");
}
