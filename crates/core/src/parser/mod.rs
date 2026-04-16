use std::fmt;
use std::io::Read;

use crate::domain::document::Document;
use crate::domain::header::IndexHeader;
use crate::error::ParseError;

pub(crate) mod document;
pub(crate) mod primitives;

use document::read_document;
use primitives::{read_i64, read_u8, try_read_field_count};

/// Streaming parser over a Maven Central index binary stream.
///
/// The reader owns the underlying byte source and yields one `Document` per
/// call to `Iterator::next`. A clean EOF on the next document's field-count
/// read terminates the iterator; a mid-document EOF is reported as a
/// `TruncatedDocument` error.
///
/// `IndexReader` is I/O-neutral: wrap your gzip decoder outside the crate and
/// pass the resulting `Read` in.
pub struct IndexReader<R: Read> {
    inner: R,
    header: IndexHeader,
}

impl<R: Read> IndexReader<R> {
    /// Read and validate the 9-byte header, returning a parser ready to stream
    /// documents.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError::UnsupportedVersion`] if the version byte is not
    /// `0x01`, or [`ParseError::Io`] if the header cannot be read.
    pub fn new(mut inner: R) -> Result<Self, ParseError> {
        let version = read_u8(&mut inner)?;
        if version != 0x01 {
            return Err(ParseError::UnsupportedVersion(version));
        }
        let ts = read_i64(&mut inner)?;
        let timestamp_millis = if ts == -1 { None } else { Some(ts) };
        Ok(Self {
            inner,
            header: IndexHeader {
                version,
                timestamp_millis,
            },
        })
    }

    /// Return the parsed header for this stream.
    #[must_use]
    pub fn header(&self) -> &IndexHeader {
        &self.header
    }
}

impl<R: Read> fmt::Debug for IndexReader<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IndexReader")
            .field("header", &self.header)
            .finish_non_exhaustive()
    }
}

impl<R: Read> Iterator for IndexReader<R> {
    type Item = Result<Document, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        match try_read_field_count(&mut self.inner) {
            Ok(None) => None,
            Ok(Some(field_count)) => Some(read_document(&mut self.inner, field_count)),
            Err(e) => Some(Err(e)),
        }
    }
}
