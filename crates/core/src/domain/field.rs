use super::flags::FieldFlags;

/// A single decoded field inside an index document.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Field {
    pub flags: FieldFlags,
    pub name: String,
    pub value: String,
}
