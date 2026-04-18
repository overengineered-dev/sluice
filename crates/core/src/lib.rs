//! Pure-Rust streaming parser for the
//! [Maven Central Nexus binary index format](https://maven.apache.org/repository/central-index.html).
//!
//! The crate is I/O-neutral: it operates on any [`std::io::Read`] and has no
//! knowledge of gzip, files, HTTP, or JSON. Gzip wrapping and output belong
//! to the CLI crate. For a byte-level specification see
//! [`docs/binary-format.md`](https://github.com/overengineered-dev/sluice/blob/main/docs/binary-format.md).
//!
//! # Quick start
//!
//! ```
//! use std::io::Cursor;
//! use sluice::{IndexReader, Record};
//!
//! // A minimal binary stream: version 0x01, timestamp, then one artifact-add document.
//! let mut stream = Vec::new();
//! stream.push(0x01u8);                                       // version
//! stream.extend_from_slice(&1_700_000_000_000i64.to_be_bytes()); // timestamp
//! stream.extend_from_slice(&3i32.to_be_bytes());             // field count
//! // field: flags=0x05, name="u", value="org.example|mylib|1.0|NA|jar"
//! stream.push(0x05);
//! stream.extend_from_slice(&1u16.to_be_bytes());
//! stream.extend_from_slice(b"u");
//! let uinfo = b"org.example|mylib|1.0|NA|jar";
//! stream.extend_from_slice(&(uinfo.len() as i32).to_be_bytes());
//! stream.extend_from_slice(uinfo);
//! // field: flags=0x04, name="i", value="jar|1700000000000|123|0|0|0|jar"
//! stream.push(0x04);
//! stream.extend_from_slice(&1u16.to_be_bytes());
//! stream.extend_from_slice(b"i");
//! let info = b"jar|1700000000000|123|0|0|0|jar";
//! stream.extend_from_slice(&(info.len() as i32).to_be_bytes());
//! stream.extend_from_slice(info);
//! // field: flags=0x04, name="m", value="1700000000000"
//! stream.push(0x04);
//! stream.extend_from_slice(&1u16.to_be_bytes());
//! stream.extend_from_slice(b"m");
//! let modified = b"1700000000000";
//! stream.extend_from_slice(&(modified.len() as i32).to_be_bytes());
//! stream.extend_from_slice(modified);
//!
//! let reader = IndexReader::new(Cursor::new(stream))?;
//! assert_eq!(reader.header().version, 0x01);
//!
//! for doc in reader {
//!     let doc = doc?;
//!     match Record::try_from(&doc)? {
//!         Record::ArtifactAdd(u) => {
//!             assert_eq!(u.group_id, "org.example");
//!             assert_eq!(u.artifact_id, "mylib");
//!         }
//!         _ => {}
//!     }
//! }
//! # Ok::<(), sluice::ParseError>(())
//! ```

#![warn(missing_docs)]

pub mod domain;
/// Error type for parsing operations.
pub mod error;
/// Streaming parser over a binary index stream.
pub mod parser;

pub use domain::{
    document::Document, field::Field, flags::FieldFlags, header::IndexHeader, record::Record,
    uinfo::Uinfo,
};
pub use error::ParseError;
pub use parser::IndexReader;
