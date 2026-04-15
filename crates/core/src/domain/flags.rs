/// Bitfield describing how a field is stored/indexed in the index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct FieldFlags(u8);

impl FieldFlags {
    pub const INDEXED: u8 = 0x01;
    pub const TOKENIZED: u8 = 0x02;
    pub const STORED: u8 = 0x04;

    #[must_use]
    pub fn new(bits: u8) -> Self {
        Self(bits)
    }

    #[must_use]
    pub fn bits(self) -> u8 {
        self.0
    }

    #[must_use]
    pub fn is_indexed(self) -> bool {
        self.0 & Self::INDEXED != 0
    }

    #[must_use]
    pub fn is_tokenized(self) -> bool {
        self.0 & Self::TOKENIZED != 0
    }

    #[must_use]
    pub fn is_stored(self) -> bool {
        self.0 & Self::STORED != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_flags_have_no_bits() {
        let f = FieldFlags::new(0);
        assert!(!f.is_indexed());
        assert!(!f.is_tokenized());
        assert!(!f.is_stored());
    }

    #[test]
    fn indexed_only() {
        let f = FieldFlags::new(FieldFlags::INDEXED);
        assert!(f.is_indexed());
        assert!(!f.is_tokenized());
        assert!(!f.is_stored());
    }

    #[test]
    fn indexed_tokenized_stored() {
        let f = FieldFlags::new(FieldFlags::INDEXED | FieldFlags::TOKENIZED | FieldFlags::STORED);
        assert!(f.is_indexed());
        assert!(f.is_tokenized());
        assert!(f.is_stored());
        assert_eq!(f.bits(), 0x07);
    }

    #[test]
    fn stored_only() {
        let f = FieldFlags::new(FieldFlags::STORED);
        assert!(!f.is_indexed());
        assert!(!f.is_tokenized());
        assert!(f.is_stored());
    }
}
