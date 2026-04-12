use std::io::Read;

use crate::domain::document::Document;
use crate::domain::field::Field;
use crate::domain::flags::FieldFlags;
use crate::error::ParseError;

use super::primitives::{read_mutf8_name, read_mutf8_value, read_u8};

/// Read `field_count` fields and return them as a `Document`.
///
/// # Errors
///
/// Returns [`ParseError::InvalidFieldCount`] if `field_count` is negative,
/// or [`ParseError::Io`] / [`ParseError::InvalidMutf8`] if a field cannot
/// be read or decoded.
pub fn read_document<R: Read>(r: &mut R, field_count: i32) -> Result<Document, ParseError> {
    let Ok(count) = usize::try_from(field_count) else {
        return Err(ParseError::InvalidFieldCount(field_count));
    };
    let mut fields = Vec::with_capacity(count);
    for _ in 0..count {
        let flags = FieldFlags::new(read_u8(r)?);
        let name = read_mutf8_name(r)?;
        let value = read_mutf8_value(r)?;
        fields.push(Field { flags, name, value });
    }
    Ok(Document::new(fields))
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    fn encode_name(name: &str) -> Vec<u8> {
        let bytes = name.as_bytes();
        let mut v = Vec::new();
        let len = u16::try_from(bytes.len()).unwrap();
        v.extend_from_slice(&len.to_be_bytes());
        v.extend_from_slice(bytes);
        v
    }

    fn encode_value(value: &str) -> Vec<u8> {
        let bytes = value.as_bytes();
        let mut v = Vec::new();
        let len = i32::try_from(bytes.len()).unwrap();
        v.extend_from_slice(&len.to_be_bytes());
        v.extend_from_slice(bytes);
        v
    }

    fn encode_field(flags: u8, name: &str, value: &str) -> Vec<u8> {
        let mut v = Vec::new();
        v.push(flags);
        v.extend_from_slice(&encode_name(name));
        v.extend_from_slice(&encode_value(value));
        v
    }

    #[test]
    fn zero_field_document() {
        let mut c = Cursor::new(Vec::<u8>::new());
        let d = read_document(&mut c, 0).unwrap();
        assert!(d.fields.is_empty());
    }

    #[test]
    fn single_field_document() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&encode_field(0x07, "u", "org.example|lib|1.0|NA|jar"));
        let mut c = Cursor::new(bytes);
        let d = read_document(&mut c, 1).unwrap();
        assert_eq!(d.fields.len(), 1);
        assert_eq!(d.fields[0].name, "u");
        assert_eq!(d.fields[0].value, "org.example|lib|1.0|NA|jar");
        assert_eq!(d.fields[0].flags.bits(), 0x07);
    }

    #[test]
    fn multi_field_document_preserves_order() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&encode_field(0x07, "u", "org.example|lib|1.0|NA|jar"));
        bytes.extend_from_slice(&encode_field(0x04, "i", "jar|1700000000000|123|0|0|0|jar"));
        bytes.extend_from_slice(&encode_field(0x04, "m", "1700000000000"));
        let mut c = Cursor::new(bytes);
        let d = read_document(&mut c, 3).unwrap();
        assert_eq!(d.fields.len(), 3);
        assert_eq!(d.fields[0].name, "u");
        assert_eq!(d.fields[1].name, "i");
        assert_eq!(d.fields[2].name, "m");
    }

    #[test]
    fn negative_field_count_is_rejected() {
        let mut c = Cursor::new(Vec::<u8>::new());
        assert!(matches!(
            read_document(&mut c, -1),
            Err(ParseError::InvalidFieldCount(-1))
        ));
    }

    #[test]
    fn truncation_mid_field_is_io_error() {
        // Start a field, EOF inside the name length prefix.
        let bytes = vec![0x07, 0x00];
        let mut c = Cursor::new(bytes);
        let err = read_document(&mut c, 1).unwrap_err();
        assert!(matches!(err, ParseError::Io(_)));
    }
}
