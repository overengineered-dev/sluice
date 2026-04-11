use std::io::{ErrorKind, Read};

use crate::error::ParseError;

/// Hard ceiling for a single field value length — guards against corrupt
/// 4-byte length prefixes blowing up the allocator (MINDEXER-28 class bugs).
const MAX_VALUE_LEN: usize = 256 * 1024 * 1024;

pub fn read_u8<R: Read>(r: &mut R) -> Result<u8, ParseError> {
    let mut buf = [0u8; 1];
    r.read_exact(&mut buf)?;
    Ok(buf[0])
}

pub fn read_u16<R: Read>(r: &mut R) -> Result<u16, ParseError> {
    let mut buf = [0u8; 2];
    r.read_exact(&mut buf)?;
    Ok(u16::from_be_bytes(buf))
}

pub fn read_i32<R: Read>(r: &mut R) -> Result<i32, ParseError> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(i32::from_be_bytes(buf))
}

pub fn read_i64<R: Read>(r: &mut R) -> Result<i64, ParseError> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    Ok(i64::from_be_bytes(buf))
}

/// Read a Java-`DataInput::readUTF` style string: `u16` length prefix then
/// `N` bytes of Java Modified UTF-8.
pub fn read_mutf8_name<R: Read>(r: &mut R) -> Result<String, ParseError> {
    let len = read_u16(r)? as usize;
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf)?;
    decode_mutf8(&buf, "name")
}

/// Read the custom Maven-indexer value string: `i32` length prefix then
/// `N` bytes of Java Modified UTF-8. The writer (MINDEXER-28) widened the
/// prefix from 2 to 4 bytes so class-name lists can exceed 64 KB.
pub fn read_mutf8_value<R: Read>(r: &mut R) -> Result<String, ParseError> {
    let len_i32 = read_i32(r)?;
    let Ok(len) = usize::try_from(len_i32) else {
        return Err(ParseError::InvalidValueLength(len_i32));
    };
    if len > MAX_VALUE_LEN {
        return Err(ParseError::InvalidValueLength(len_i32));
    }
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf)?;
    decode_mutf8(&buf, "value")
}

/// Attempt to read the 4-byte field count for the next document.
///
/// Returns:
/// - `Ok(None)` on clean EOF *before any byte* of the length is consumed.
/// - `Err(TruncatedDocument)` if 1–3 bytes were consumed then EOF.
/// - `Err(InvalidFieldCount)` if the decoded value is negative.
/// - `Ok(Some(n))` otherwise.
pub fn try_read_field_count<R: Read>(r: &mut R) -> Result<Option<i32>, ParseError> {
    let mut buf = [0u8; 4];
    let mut filled = 0usize;
    while filled < 4 {
        match r.read(&mut buf[filled..]) {
            Ok(0) => {
                if filled == 0 {
                    return Ok(None);
                }
                return Err(ParseError::TruncatedDocument {
                    bytes_into_doc: filled as u64,
                });
            }
            Ok(n) => filled += n,
            Err(e) if e.kind() == ErrorKind::Interrupted => {}
            Err(e) => return Err(ParseError::Io(e)),
        }
    }
    let n = i32::from_be_bytes(buf);
    if n < 0 {
        return Err(ParseError::InvalidFieldCount(n));
    }
    Ok(Some(n))
}

fn decode_mutf8(bytes: &[u8], context: &'static str) -> Result<String, ParseError> {
    match cesu8::from_java_cesu8(bytes) {
        Ok(cow) => Ok(cow.into_owned()),
        Err(source) => Err(ParseError::InvalidMutf8 { context, source }),
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn read_u8_reads_one_byte() {
        let mut c = Cursor::new(vec![0xAB]);
        assert_eq!(read_u8(&mut c).unwrap(), 0xAB);
    }

    #[test]
    fn read_u16_big_endian() {
        let mut c = Cursor::new(vec![0x12, 0x34]);
        assert_eq!(read_u16(&mut c).unwrap(), 0x1234);
    }

    #[test]
    fn read_i32_big_endian() {
        let mut c = Cursor::new(vec![0x00, 0x00, 0x00, 0x2A]);
        assert_eq!(read_i32(&mut c).unwrap(), 42);
    }

    #[test]
    fn read_i32_negative() {
        let mut c = Cursor::new(vec![0xFF, 0xFF, 0xFF, 0xFF]);
        assert_eq!(read_i32(&mut c).unwrap(), -1);
    }

    #[test]
    fn read_i64_big_endian() {
        let mut c = Cursor::new(vec![0, 0, 0, 0, 0, 0, 0x01, 0x00]);
        assert_eq!(read_i64(&mut c).unwrap(), 256);
    }

    fn mutf8_name_bytes(body: &[u8]) -> Vec<u8> {
        let mut v = Vec::new();
        let len = u16::try_from(body.len()).unwrap();
        v.extend_from_slice(&len.to_be_bytes());
        v.extend_from_slice(body);
        v
    }

    fn mutf8_value_bytes(body: &[u8]) -> Vec<u8> {
        let mut v = Vec::new();
        let len = i32::try_from(body.len()).unwrap();
        v.extend_from_slice(&len.to_be_bytes());
        v.extend_from_slice(body);
        v
    }

    #[test]
    fn mutf8_name_plain_ascii() {
        let raw = mutf8_name_bytes(b"hello");
        let mut c = Cursor::new(raw);
        assert_eq!(read_mutf8_name(&mut c).unwrap(), "hello");
    }

    #[test]
    fn mutf8_value_plain_ascii() {
        let raw = mutf8_value_bytes(b"world");
        let mut c = Cursor::new(raw);
        assert_eq!(read_mutf8_value(&mut c).unwrap(), "world");
    }

    #[test]
    fn mutf8_null_char_as_c0_80() {
        // Java encodes U+0000 as 0xC0 0x80, never as 0x00.
        let raw = mutf8_name_bytes(&[0xC0, 0x80]);
        let mut c = Cursor::new(raw);
        let s = read_mutf8_name(&mut c).unwrap();
        assert_eq!(s, "\u{0000}");
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn mutf8_two_byte_latin1() {
        // 'é' U+00E9 encoded as 0xC3 0xA9 (same as standard UTF-8 here).
        let raw = mutf8_name_bytes(&[0xC3, 0xA9]);
        let mut c = Cursor::new(raw);
        assert_eq!(read_mutf8_name(&mut c).unwrap(), "é");
    }

    #[test]
    fn mutf8_three_byte_cjk() {
        // '漢' U+6F22 encoded as 0xE6 0xBC 0xA2 (same as standard UTF-8 here).
        let raw = mutf8_name_bytes(&[0xE6, 0xBC, 0xA2]);
        let mut c = Cursor::new(raw);
        assert_eq!(read_mutf8_name(&mut c).unwrap(), "漢");
    }

    #[test]
    fn mutf8_supplementary_code_point_via_surrogate_pair() {
        // '🎉' U+1F389. In Modified UTF-8 this is encoded as a UTF-16
        // surrogate pair, each half written as 3 bytes → 6 bytes total.
        // High surrogate U+D83C → 1110_1101 1010_0000 1011_1100 = ED A0 BC
        // Low  surrogate U+DF89 → 1110_1101 1011_1110 1000_1001 = ED BE 89
        let raw = mutf8_name_bytes(&[0xED, 0xA0, 0xBC, 0xED, 0xBE, 0x89]);
        let mut c = Cursor::new(raw);
        assert_eq!(read_mutf8_name(&mut c).unwrap(), "🎉");
    }

    #[test]
    fn mutf8_value_rejects_negative_length() {
        let mut v = Vec::new();
        v.extend_from_slice(&(-5i32).to_be_bytes());
        let mut c = Cursor::new(v);
        assert!(matches!(
            read_mutf8_value(&mut c),
            Err(ParseError::InvalidValueLength(-5))
        ));
    }

    #[test]
    fn mutf8_value_rejects_length_over_cap() {
        let mut v = Vec::new();
        // i32::MAX is comfortably above the 256 MB cap, so this will trip
        // the size guard without risking platform-specific cast issues.
        let bogus: i32 = i32::MAX;
        v.extend_from_slice(&bogus.to_be_bytes());
        let mut c = Cursor::new(v);
        assert!(matches!(
            read_mutf8_value(&mut c),
            Err(ParseError::InvalidValueLength(_))
        ));
    }

    #[test]
    fn mutf8_value_supports_length_beyond_u16_max() {
        // Proves the 4-byte prefix actually enables payloads larger than a
        // u16-limited name prefix would allow.
        let body = vec![b'x'; 70_000];
        let raw = mutf8_value_bytes(&body);
        let mut c = Cursor::new(raw);
        let s = read_mutf8_value(&mut c).unwrap();
        assert_eq!(s.len(), 70_000);
    }

    #[test]
    fn try_read_field_count_eof_before_any_byte_is_clean_end() {
        let mut c = Cursor::new(Vec::<u8>::new());
        assert!(matches!(try_read_field_count(&mut c), Ok(None)));
    }

    #[test]
    fn try_read_field_count_partial_is_truncation() {
        let mut c = Cursor::new(vec![0x00, 0x00]);
        let err = try_read_field_count(&mut c).unwrap_err();
        assert!(matches!(
            err,
            ParseError::TruncatedDocument { bytes_into_doc: 2 }
        ));
    }

    #[test]
    fn try_read_field_count_happy_path() {
        let mut c = Cursor::new(vec![0x00, 0x00, 0x00, 0x05]);
        assert_eq!(try_read_field_count(&mut c).unwrap(), Some(5));
    }

    #[test]
    fn try_read_field_count_rejects_negative() {
        let mut c = Cursor::new(vec![0xFF, 0xFF, 0xFF, 0xFF]);
        assert!(matches!(
            try_read_field_count(&mut c),
            Err(ParseError::InvalidFieldCount(-1))
        ));
    }
}
