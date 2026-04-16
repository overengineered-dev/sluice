use std::fmt;

use super::document::Document;
use super::uinfo::{parse_info_extension, parse_uinfo, Uinfo};
use crate::error::ParseError;

/// A classified Maven index document: either a structural record
/// (descriptor, group lists) or an artifact add/remove with parsed coordinates.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub enum Record {
    /// Index descriptor document (header metadata).
    Descriptor,
    /// `allGroups` index — the full list of group IDs.
    AllGroups,
    /// `rootGroups` index — top-level group name prefixes.
    RootGroups,
    /// Artifact add record with parsed coordinates.
    ArtifactAdd(Uinfo),
    /// Artifact removal record with parsed coordinates.
    ArtifactRemove(Uinfo),
    /// Document that did not match any known record shape.
    Unknown,
}

impl fmt::Display for Record {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Record::Descriptor => f.write_str("descriptor"),
            Record::AllGroups => f.write_str("allGroups"),
            Record::RootGroups => f.write_str("rootGroups"),
            Record::ArtifactAdd(u) => write!(f, "add {u}"),
            Record::ArtifactRemove(u) => write!(f, "remove {u}"),
            Record::Unknown => f.write_str("unknown"),
        }
    }
}

impl TryFrom<&Document> for Record {
    type Error = ParseError;

    /// Classify a document into a [`Record`].
    ///
    /// Rules, checked in priority order:
    /// 1. `DESCRIPTOR` field → `Descriptor`
    /// 2. `allGroups` field → `AllGroups`
    /// 3. `rootGroups` field → `RootGroups`
    /// 4. `u` field → `ArtifactAdd(parse_uinfo(u))`
    /// 5. `del` field → `ArtifactRemove(parse_uinfo(del))`
    /// 6. otherwise → `Unknown`
    ///
    /// # Errors
    ///
    /// Returns [`ParseError::MalformedUinfo`] when an add or remove record
    /// contains a UINFO string that cannot be parsed. Structural documents
    /// (descriptor, group lists) never fail.
    fn try_from(doc: &Document) -> Result<Self, Self::Error> {
        if doc.has("DESCRIPTOR") {
            return Ok(Record::Descriptor);
        }
        if doc.has("allGroups") {
            return Ok(Record::AllGroups);
        }
        if doc.has("rootGroups") {
            return Ok(Record::RootGroups);
        }
        if let Some(raw) = doc.find("u") {
            let mut uinfo = parse_uinfo(raw)?;
            // MINDEXER-41: backfill extension from INFO field when UINFO has
            // only 4 segments (pre-5.x indexes omit the extension segment).
            if uinfo.extension.is_none() {
                if let Some(info_raw) = doc.find("i") {
                    uinfo.extension = parse_info_extension(info_raw);
                }
            }
            return Ok(Record::ArtifactAdd(uinfo));
        }
        if let Some(raw) = doc.find("del") {
            return Ok(Record::ArtifactRemove(parse_uinfo(raw)?));
        }
        Ok(Record::Unknown)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::field::Field;
    use crate::domain::flags::FieldFlags;

    fn field(name: &str, value: &str) -> Field {
        Field {
            flags: FieldFlags::new(0x07),
            name: name.to_owned(),
            value: value.to_owned(),
        }
    }

    fn doc(fields: Vec<Field>) -> Document {
        Document::new(fields)
    }

    #[test]
    fn classifies_descriptor() {
        let d = doc(vec![
            field("DESCRIPTOR", "NexusIndex"),
            field("IDXINFO", "..."),
        ]);
        assert_eq!(Record::try_from(&d).unwrap(), Record::Descriptor);
    }

    #[test]
    fn classifies_all_groups() {
        let d = doc(vec![
            field("allGroups", "ignored"),
            field("allGroupsList", "a|b|c"),
        ]);
        assert_eq!(Record::try_from(&d).unwrap(), Record::AllGroups);
    }

    #[test]
    fn classifies_root_groups() {
        let d = doc(vec![
            field("rootGroups", "x"),
            field("rootGroupsList", "a|b"),
        ]);
        assert_eq!(Record::try_from(&d).unwrap(), Record::RootGroups);
    }

    #[test]
    fn classifies_add() {
        let d = doc(vec![field("u", "org.example|lib|1.0|NA|jar")]);
        let Record::ArtifactAdd(u) = Record::try_from(&d).unwrap() else {
            panic!("expected Add");
        };
        assert_eq!(u.group_id, "org.example");
    }

    #[test]
    fn classifies_remove() {
        let d = doc(vec![field("del", "org.example|lib|1.0|NA|jar")]);
        let Record::ArtifactRemove(u) = Record::try_from(&d).unwrap() else {
            panic!("expected Remove");
        };
        assert_eq!(u.artifact_id, "lib");
    }

    #[test]
    fn unknown_when_no_recognisable_field() {
        let d = doc(vec![field("foo", "bar")]);
        assert_eq!(Record::try_from(&d).unwrap(), Record::Unknown);
    }

    #[test]
    fn descriptor_beats_u_field_priority() {
        let d = doc(vec![field("DESCRIPTOR", "x"), field("u", "a|b|c|NA|jar")]);
        assert_eq!(Record::try_from(&d).unwrap(), Record::Descriptor);
    }

    #[test]
    fn all_groups_beats_u_field_priority() {
        let d = doc(vec![field("allGroups", "x"), field("u", "a|b|c|NA|jar")]);
        assert_eq!(Record::try_from(&d).unwrap(), Record::AllGroups);
    }

    #[test]
    fn malformed_uinfo_on_add_bubbles_up() {
        let d = doc(vec![field("u", "not-enough-pipes")]);
        assert!(matches!(
            Record::try_from(&d),
            Err(ParseError::MalformedUinfo(_))
        ));
    }

    #[test]
    fn four_segment_uinfo_backfills_extension_from_info() {
        let d = doc(vec![
            field("u", "org.example|lib|1.0|NA"),
            field("i", "jar|1700000000000|123|0|0|0|jar"),
        ]);
        let Record::ArtifactAdd(u) = Record::try_from(&d).unwrap() else {
            panic!("expected ArtifactAdd");
        };
        assert_eq!(u.extension.as_deref(), Some("jar"));
    }

    #[test]
    fn five_segment_uinfo_ignores_info_extension() {
        let d = doc(vec![
            field("u", "org.example|lib|1.0|NA|war"),
            field("i", "jar|1700000000000|123|0|0|0|jar"),
        ]);
        let Record::ArtifactAdd(u) = Record::try_from(&d).unwrap() else {
            panic!("expected ArtifactAdd");
        };
        assert_eq!(u.extension.as_deref(), Some("war"));
    }

    #[test]
    fn display_descriptor() {
        assert_eq!(Record::Descriptor.to_string(), "descriptor");
    }

    #[test]
    fn display_unknown() {
        assert_eq!(Record::Unknown.to_string(), "unknown");
    }

    #[test]
    fn display_artifact_add_uses_uinfo_display() {
        let d = doc(vec![field("u", "org.example|lib|1.0|NA|jar")]);
        let r = Record::try_from(&d).unwrap();
        assert_eq!(r.to_string(), "add org.example:lib:1.0:jar");
    }

    #[test]
    fn display_artifact_remove_uses_uinfo_display() {
        let d = doc(vec![field("del", "org.example|lib|1.0|sources|jar")]);
        let r = Record::try_from(&d).unwrap();
        assert_eq!(r.to_string(), "remove org.example:lib:1.0:sources:jar");
    }

    #[test]
    fn four_segment_uinfo_without_info_stays_none() {
        let d = doc(vec![field("u", "org.example|lib|1.0|NA")]);
        let Record::ArtifactAdd(u) = Record::try_from(&d).unwrap() else {
            panic!("expected ArtifactAdd");
        };
        assert_eq!(u.extension, None);
    }
}
