/// The 9-byte header that prefixes every Maven Central index stream.
///
/// The version byte is currently always `0x01`. The timestamp is milliseconds
/// since the Unix epoch; the on-wire sentinel `-1` is mapped to `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct IndexHeader {
    pub version: u8,
    pub timestamp_millis: Option<i64>,
}
