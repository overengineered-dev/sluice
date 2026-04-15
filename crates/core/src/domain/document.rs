use super::field::Field;

/// An index document: an ordered list of fields.
///
/// Order follows the on-wire layout. Documents average ~10 fields, so the
/// linear-scan `find` helper is fine.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Document {
    fields: Vec<Field>,
}

impl Document {
    #[must_use]
    pub fn new(fields: Vec<Field>) -> Self {
        Self { fields }
    }

    /// All fields in on-wire order.
    #[must_use]
    pub fn fields(&self) -> &[Field] {
        &self.fields
    }

    /// Return the value of the first field whose name equals `key`.
    #[must_use]
    pub fn find(&self, key: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|f| f.name == key)
            .map(|f| f.value.as_str())
    }

    /// Return `true` if any field's name equals `key`.
    #[must_use]
    pub fn has(&self, key: &str) -> bool {
        self.fields.iter().any(|f| f.name == key)
    }
}
