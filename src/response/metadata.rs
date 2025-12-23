//! Newsgroup metadata types for NNTP responses.
//!
//! This module contains types representing metadata about newsgroups and articles:
//! - [`NewsGroup`] - Information about a newsgroup
//! - [`OverviewEntry`] - Article metadata from OVER command
//! - [`HeaderEntry`] - Header field data from HDR command

/// Newsgroup information
#[derive(Debug, Clone, PartialEq)]
pub struct NewsGroup {
    /// Group name
    pub name: String,
    /// Last article number
    pub last: u64,
    /// First article number
    pub first: u64,
    /// Posting status (y/n/m)
    pub posting_status: char,
}

/// Overview entry for OVER command response
#[derive(Debug, Clone, PartialEq)]
pub struct OverviewEntry {
    /// Raw tab-separated fields from the OVER response
    pub fields: Vec<String>,
}

impl OverviewEntry {
    /// Get article number (always the first field)
    pub fn number(&self) -> Option<u64> {
        self.fields.first()?.parse().ok()
    }

    /// Get field at specific index
    pub fn get_field(&self, index: usize) -> Option<&str> {
        self.fields.get(index).map(|s| s.as_str())
    }

    /// Get field by name (requires field format knowledge)
    /// This is a helper that assumes the default RFC 3977 format
    pub fn get_default_field(&self, field_name: &str) -> Option<&str> {
        let index = match field_name.to_lowercase().as_str() {
            "subject" => 1,
            "from" => 2,
            "date" => 3,
            "message-id" => 4,
            "references" => 5,
            "byte_count" | "bytes" => 6,
            "line_count" | "lines" => 7,
            _ => return None,
        };
        self.get_field(index)
    }

    /// Get subject field (index 1 in default format)
    pub fn subject(&self) -> Option<&str> {
        self.get_field(1)
    }

    /// Get from field (index 2 in default format)
    pub fn from(&self) -> Option<&str> {
        self.get_field(2)
    }

    /// Get date field (index 3 in default format)
    pub fn date(&self) -> Option<&str> {
        self.get_field(3)
    }

    /// Get message-id field (index 4 in default format)
    pub fn message_id(&self) -> Option<&str> {
        self.get_field(4)
    }

    /// Get references field (index 5 in default format)
    pub fn references(&self) -> Option<&str> {
        self.get_field(5)
    }

    /// Get byte count field (index 6 in default format)
    pub fn byte_count(&self) -> Option<u64> {
        self.get_field(6)?.parse().ok()
    }

    /// Get line count field (index 7 in default format)
    pub fn line_count(&self) -> Option<u64> {
        self.get_field(7)?.parse().ok()
    }
}

/// Header entry for HDR command response
#[derive(Debug, Clone, PartialEq)]
pub struct HeaderEntry {
    /// Article number or message ID
    pub article: String,
    /// Header field value
    pub value: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_newsgroup() {
        let group = NewsGroup {
            name: "misc.test".to_string(),
            last: 100,
            first: 1,
            posting_status: 'y',
        };
        assert_eq!(group.name, "misc.test");
        assert_eq!(group.first, 1);
        assert_eq!(group.last, 100);
        assert_eq!(group.posting_status, 'y');
    }

    #[test]
    fn test_overview_entry() {
        let entry = OverviewEntry {
            fields: vec![
                "3000".to_string(),
                "I am just a test article".to_string(),
                "demo@example.com".to_string(),
                "6 Oct 1998 04:38:40 -0500".to_string(),
                "<45223423@example.com>".to_string(),
                "".to_string(),
                "1234".to_string(),
                "42".to_string(),
            ],
        };

        assert_eq!(entry.number(), Some(3000));
        assert_eq!(entry.subject(), Some("I am just a test article"));
        assert_eq!(entry.from(), Some("demo@example.com"));
        assert_eq!(entry.date(), Some("6 Oct 1998 04:38:40 -0500"));
        assert_eq!(entry.message_id(), Some("<45223423@example.com>"));
        assert_eq!(entry.references(), Some(""));
        assert_eq!(entry.byte_count(), Some(1234));
        assert_eq!(entry.line_count(), Some(42));

        // Test get_default_field
        assert_eq!(
            entry.get_default_field("subject"),
            Some("I am just a test article")
        );
        assert_eq!(entry.get_default_field("from"), Some("demo@example.com"));
        assert_eq!(entry.get_default_field("bytes"), Some("1234"));
        assert_eq!(entry.get_default_field("lines"), Some("42"));
        assert_eq!(entry.get_default_field("unknown"), None);
    }

    #[test]
    fn test_header_entry() {
        let entry = HeaderEntry {
            article: "3000".to_string(),
            value: "Test Subject".to_string(),
        };
        assert_eq!(entry.article, "3000");
        assert_eq!(entry.value, "Test Subject");
    }
}
