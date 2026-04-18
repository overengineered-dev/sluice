use cesu8::Cesu8DecodingError;

/// All failure modes of the binary index parser.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ParseError {
    /// Wrapped I/O error from the underlying reader.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The index header declared an unsupported version byte.
    #[error("unsupported index version: expected 0x01, got 0x{0:02X}")]
    UnsupportedVersion(u8),

    /// A field name or value failed Java Modified UTF-8 decoding.
    #[error("modified UTF-8 decoding failed at field {context}: {source}")]
    InvalidMutf8 {
        /// Whether the failure happened on a field name or value.
        context: &'static str,
        /// Underlying cesu8 decode failure.
        #[source]
        source: Cesu8DecodingError,
    },

    /// Document field-count prefix was negative.
    #[error("invalid field count: {0}")]
    InvalidFieldCount(i32),

    /// Field value length prefix was negative or over the safety cap.
    #[error("invalid value length: {0}")]
    InvalidValueLength(i32),

    /// UINFO string had the wrong number of pipe-delimited segments.
    #[error("malformed UINFO: {0:?}")]
    MalformedUinfo(String),

    /// EOF reached mid-way through a document's framing bytes.
    #[error("unexpected EOF in middle of document after {bytes_into_doc} bytes")]
    TruncatedDocument {
        /// Number of bytes consumed into the truncated document.
        bytes_into_doc: u64,
    },
}
