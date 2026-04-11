use crate::error::ParseError;

/// Maven artifact coordinates decoded from a UINFO string.
///
/// UINFO is a pipe-delimited composite:
/// `groupId | artifactId | version | classifier | extension`.
/// `classifier` is the literal string `"NA"` on the wire when absent and is
/// mapped to `None`. The `extension` segment was absent in pre-5.x indexes and
/// is therefore also `Option`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Uinfo {
    pub group_id: String,
    pub artifact_id: String,
    pub version: String,
    pub classifier: Option<String>,
    pub extension: Option<String>,
}

/// Parse a UINFO string. Accepts either 4- or 5-segment forms.
pub fn parse_uinfo(raw: &str) -> Result<Uinfo, ParseError> {
    let mut parts = raw.split('|');
    let group_id = parts.next();
    let artifact_id = parts.next();
    let version = parts.next();
    let classifier = parts.next();
    let extension = parts.next();
    let extra = parts.next();

    let (Some(group_id), Some(artifact_id), Some(version), Some(classifier)) =
        (group_id, artifact_id, version, classifier)
    else {
        return Err(ParseError::MalformedUinfo(raw.to_owned()));
    };

    if extra.is_some() {
        return Err(ParseError::MalformedUinfo(raw.to_owned()));
    }

    if group_id.is_empty() || artifact_id.is_empty() || version.is_empty() {
        return Err(ParseError::MalformedUinfo(raw.to_owned()));
    }

    let classifier = if classifier == "NA" {
        None
    } else {
        Some(classifier.to_owned())
    };

    let extension = extension.map(ToOwned::to_owned);

    Ok(Uinfo {
        group_id: group_id.to_owned(),
        artifact_id: artifact_id.to_owned(),
        version: version.to_owned(),
        classifier,
        extension,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_segments_with_na_classifier() {
        let u = parse_uinfo("org.apache.maven|maven-core|3.9.6|NA|jar").unwrap();
        assert_eq!(u.group_id, "org.apache.maven");
        assert_eq!(u.artifact_id, "maven-core");
        assert_eq!(u.version, "3.9.6");
        assert_eq!(u.classifier, None);
        assert_eq!(u.extension.as_deref(), Some("jar"));
    }

    #[test]
    fn five_segments_with_real_classifier() {
        let u = parse_uinfo("com.example|lib|1.0|sources|jar").unwrap();
        assert_eq!(u.classifier.as_deref(), Some("sources"));
        assert_eq!(u.extension.as_deref(), Some("jar"));
    }

    #[test]
    fn four_segments_pre_5x() {
        let u = parse_uinfo("com.example|lib|1.0|NA").unwrap();
        assert_eq!(u.classifier, None);
        assert_eq!(u.extension, None);
    }

    #[test]
    fn too_many_segments_is_malformed() {
        assert!(matches!(
            parse_uinfo("a|b|c|NA|jar|extra"),
            Err(ParseError::MalformedUinfo(_))
        ));
    }

    #[test]
    fn too_few_segments_is_malformed() {
        assert!(matches!(
            parse_uinfo("a|b|c"),
            Err(ParseError::MalformedUinfo(_))
        ));
    }

    #[test]
    fn empty_string_is_malformed() {
        assert!(matches!(
            parse_uinfo(""),
            Err(ParseError::MalformedUinfo(_))
        ));
    }

    #[test]
    fn empty_group_id_is_malformed() {
        assert!(matches!(
            parse_uinfo("|b|c|NA|jar"),
            Err(ParseError::MalformedUinfo(_))
        ));
    }
}
