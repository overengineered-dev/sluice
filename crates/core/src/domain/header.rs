/// The 9-byte header that prefixes every Maven Central index stream.
///
/// The version byte is currently always `0x01`. The timestamp is milliseconds
/// since the Unix epoch; the on-wire sentinel `-1` is mapped to `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub struct IndexHeader {
    /// Format version byte; currently always `0x01`.
    pub version: u8,
    /// Index-build timestamp in milliseconds since the Unix epoch; `None` when
    /// the on-wire sentinel `-1` is present.
    pub timestamp_millis: Option<i64>,
}
