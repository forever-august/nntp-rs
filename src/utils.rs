//! Utility functions for NNTP article processing.

/// Parse a References header into a list of Message-IDs (oldest first).
///
/// The References header contains space-separated Message-IDs representing
/// the ancestry of an article, from oldest ancestor to immediate parent.
///
/// # Example
///
/// ```
/// use nntp_rs::utils::parse_references;
///
/// let refs = parse_references(Some("<a@x.com> <b@x.com> <c@x.com>"));
/// assert_eq!(refs, vec!["<a@x.com>", "<b@x.com>", "<c@x.com>"]);
///
/// // Empty or None returns empty vec
/// assert!(parse_references(None).is_empty());
/// assert!(parse_references(Some("")).is_empty());
///
/// // Invalid entries (not wrapped in <>) are filtered out
/// let refs = parse_references(Some("<valid@x.com> invalid <also-valid@y.com>"));
/// assert_eq!(refs, vec!["<valid@x.com>", "<also-valid@y.com>"]);
/// ```
pub fn parse_references(references: Option<&str>) -> Vec<String> {
    references
        .map(|refs| {
            refs.split_whitespace()
                .filter(|s| s.starts_with('<') && s.ends_with('>'))
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default()
}

/// Normalize a subject line by removing Re:, Fwd:, etc. prefixes.
///
/// This is useful for grouping articles by their base subject when
/// implementing threading algorithms.
///
/// # Example
///
/// ```
/// use nntp_rs::utils::normalize_subject;
///
/// assert_eq!(normalize_subject("Hello World"), "Hello World");
/// assert_eq!(normalize_subject("Re: Hello World"), "Hello World");
/// assert_eq!(normalize_subject("RE: Hello World"), "Hello World");
/// assert_eq!(normalize_subject("Re: Re: Hello World"), "Hello World");
/// assert_eq!(normalize_subject("Fwd: Hello World"), "Hello World");
/// assert_eq!(normalize_subject("Re: Fwd: Hello World"), "Hello World");
/// ```
pub fn normalize_subject(subject: &str) -> String {
    let mut normalized = subject.trim().to_string();

    // Common prefixes to remove (case-insensitive)
    let prefixes = ["re:", "fwd:", "fw:", "aw:", "sv:", "antw:"];

    loop {
        let lower = normalized.to_lowercase();
        let mut found = false;

        for prefix in &prefixes {
            if lower.starts_with(prefix) {
                normalized = normalized[prefix.len()..].trim_start().to_string();
                found = true;
                break;
            }
        }

        // Also handle [Fwd: ...] style
        if normalized.starts_with('[') {
            if let Some(end) = normalized.find(']') {
                let bracket_content = &normalized[1..end].to_lowercase();
                if prefixes
                    .iter()
                    .any(|p| bracket_content.starts_with(p.trim_end_matches(':')))
                {
                    normalized = normalized[end + 1..].trim_start().to_string();
                    found = true;
                }
            }
        }

        if !found {
            break;
        }
    }

    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_references_empty() {
        assert!(parse_references(None).is_empty());
        assert!(parse_references(Some("")).is_empty());
    }

    #[test]
    fn test_parse_references_single() {
        let refs = parse_references(Some("<abc@example.com>"));
        assert_eq!(refs, vec!["<abc@example.com>"]);
    }

    #[test]
    fn test_parse_references_multiple() {
        let refs = parse_references(Some("<a@x.com> <b@x.com> <c@x.com>"));
        assert_eq!(refs, vec!["<a@x.com>", "<b@x.com>", "<c@x.com>"]);
    }

    #[test]
    fn test_parse_references_filters_invalid() {
        let refs = parse_references(Some("<valid@x.com> invalid <also-valid@y.com>"));
        assert_eq!(refs, vec!["<valid@x.com>", "<also-valid@y.com>"]);
    }

    #[test]
    fn test_normalize_subject_no_prefix() {
        assert_eq!(normalize_subject("Hello World"), "Hello World");
    }

    #[test]
    fn test_normalize_subject_re_prefix() {
        assert_eq!(normalize_subject("Re: Hello World"), "Hello World");
        assert_eq!(normalize_subject("RE: Hello World"), "Hello World");
        assert_eq!(normalize_subject("re: Hello World"), "Hello World");
    }

    #[test]
    fn test_normalize_subject_multiple_re() {
        assert_eq!(normalize_subject("Re: Re: Hello World"), "Hello World");
    }

    #[test]
    fn test_normalize_subject_fwd_prefix() {
        assert_eq!(normalize_subject("Fwd: Hello World"), "Hello World");
        assert_eq!(normalize_subject("FWD: Hello World"), "Hello World");
        assert_eq!(normalize_subject("Fw: Hello World"), "Hello World");
    }

    #[test]
    fn test_normalize_subject_mixed_prefixes() {
        assert_eq!(normalize_subject("Re: Fwd: Hello World"), "Hello World");
    }

    #[test]
    fn test_normalize_subject_international_prefixes() {
        assert_eq!(normalize_subject("Aw: Hello World"), "Hello World"); // German
        assert_eq!(normalize_subject("Sv: Hello World"), "Hello World"); // Swedish
        assert_eq!(normalize_subject("Antw: Hello World"), "Hello World"); // German alternate
    }

    #[test]
    fn test_normalize_subject_bracket_style() {
        assert_eq!(
            normalize_subject("[Fwd: Something] Hello World"),
            "Hello World"
        );
    }
}
