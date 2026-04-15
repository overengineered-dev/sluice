use super::flags::FieldFlags;

/// A single decoded field inside an index document.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub struct Field {
    pub flags: FieldFlags,
    pub name: String,
    pub value: String,
}

impl Field {
    /// Construct a new `Field`. Required because the struct is
    /// `#[non_exhaustive]` and cannot be built with a struct literal from
    /// outside this crate.
    #[must_use]
    pub fn new(flags: FieldFlags, name: String, value: String) -> Self {
        Self { flags, name, value }
    }
}
