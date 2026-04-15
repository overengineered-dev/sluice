use cesu8::Cesu8DecodingError;

/// All failure modes of the binary index parser.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ParseError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("unsupported index version: expected 0x01, got 0x{0:02X}")]
    UnsupportedVersion(u8),

    #[error("modified UTF-8 decoding failed at field {context}: {source}")]
    InvalidMutf8 {
        context: &'static str,
        #[source]
        source: Cesu8DecodingError,
    },

    #[error("invalid field count: {0}")]
    InvalidFieldCount(i32),

    #[error("invalid value length: {0}")]
    InvalidValueLength(i32),

    #[error("malformed UINFO: {0:?}")]
    MalformedUinfo(String),

    #[error("unexpected EOF in middle of document after {bytes_into_doc} bytes")]
    TruncatedDocument { bytes_into_doc: u64 },
}
