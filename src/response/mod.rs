//! NNTP response types and parsing.
//!
//! This module provides types and parsing for NNTP server responses.
//!
//! # Module Structure
//!
//! - [`article`] - Article-related types ([`Article`], [`Attachment`])
//! - [`metadata`] - Newsgroup metadata types ([`NewsGroup`], [`OverviewEntry`], [`HeaderEntry`])
//! - [`wrappers`] - Newtype wrappers for type-safe response extraction

mod article;
mod metadata;
pub mod wrappers;

pub use article::{Article, Attachment};
#[allow(deprecated)]
pub use article::ParsedArticle;
pub use metadata::{HeaderEntry, NewsGroup, OverviewEntry};
pub use wrappers::*;

use crate::error::{Error, Result};
use mail_parser::{Message, MessageParser};

/// NNTP server responses
#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    /// Server capabilities response (101)
    Capabilities(Vec<String>),

    /// Mode reader response (200/201)
    ModeReader {
        /// Whether posting is allowed
        posting_allowed: bool,
    },

    /// Authentication successful (281)
    AuthSuccess,

    /// Authentication required (381)
    AuthRequired,

    /// Group selected successfully (211)
    GroupSelected {
        /// Estimated number of articles
        count: u64,
        /// First article number
        first: u64,
        /// Last article number
        last: u64,
        /// Group name
        name: String,
    },

    /// Article listing (211)
    ArticleListing(Vec<u64>),

    /// Article retrieved (220/221/222)
    Article {
        /// Article number
        number: Option<u64>,
        /// Message-ID
        message_id: String,
        /// Article content (headers and/or body)
        content: Vec<u8>,
    },

    /// Article status (223)
    ArticleStatus {
        /// Article number
        number: u64,
        /// Message-ID
        message_id: String,
    },

    /// Newsgroup list (215)
    NewsgroupList(Vec<NewsGroup>),

    /// New newsgroups (231)
    NewNewsgroups(Vec<NewsGroup>),

    /// New articles (230)
    NewArticles(Vec<String>),

    /// Post accepted (340)
    PostAccepted,

    /// Article wanted for IHAVE (335)
    ArticleWanted,

    /// Article not wanted for IHAVE (435/436)
    ArticleNotWanted,

    /// Article transferred successfully (235)
    ArticleTransferred,

    /// Article posted successfully (240)
    PostSuccess,

    /// Connection closing (205)
    Quit,

    /// Help information (100)
    Help(Vec<String>),

    /// Server date and time (111)
    Date(String),

    /// Header field data (225)
    HeaderData(Vec<HeaderEntry>),

    /// Overview data (224)
    OverviewData(Vec<OverviewEntry>),

    /// Overview format data (215)
    OverviewFormat(Vec<String>),

    /// Generic successful response
    Success {
        /// Response code
        code: u16,
        /// Response message
        message: String,
    },

    /// Protocol error response (4xx/5xx codes).
    ///
    /// Use helper methods like `is_no_such_newsgroup()`, `is_auth_required()` etc.
    /// to check for specific error conditions.
    Error {
        /// Error code
        code: u16,
        /// Error message
        message: String,
    },

    // RFC 4642 (STARTTLS) responses

    /// TLS negotiation may begin (382 response).
    ///
    /// After receiving this response, the client should immediately begin
    /// TLS negotiation on the underlying connection. This library provides
    /// protocol-level support but does not perform the actual TLS handshake.
    ///
    /// # Usage
    ///
    /// When using the sans-IO layer:
    ///
    /// 1. Send [`Command::StartTls`]
    /// 2. Receive this `TlsReady` response
    /// 3. Wrap the underlying stream with a TLS implementation (e.g., `rustls`, `native-tls`)
    /// 4. Continue NNTP protocol communication over the encrypted connection
    ///
    /// [`Command::StartTls`]: crate::command::Command::StartTls
    TlsReady,

    /// TLS temporarily unavailable (483 response).
    ///
    /// The server supports STARTTLS but cannot currently perform TLS negotiation.
    /// The client may retry later or continue with an unencrypted connection
    /// if appropriate for the use case.
    TlsNotAvailable {
        /// Server-provided message explaining why TLS is unavailable
        message: String,
    },
}

/// Convert bytes with various text encodings to UTF-8 string
///
/// This function attempts to detect the encoding of the input bytes and convert
/// them to a UTF-8 string. It tries several common encodings used in NNTP:
/// 1. UTF-8 (try first since it's most common now)
/// 2. Windows-1252 (covers ISO-8859-1 range plus extra characters)
/// 3. ISO-8859-15 (Latin-9, common in Europe)
/// 4. ISO-8859-2 (Central European)
///
/// If all fail, it falls back to lossy UTF-8 conversion.
fn decode_text_with_encoding(data: &[u8]) -> String {
    // First try UTF-8 since it's the most common nowadays
    if let Ok(text) = std::str::from_utf8(data) {
        return text.to_string();
    }

    // Common encodings to try in order of likelihood for NNTP
    let encodings_to_try = [
        encoding_rs::WINDOWS_1252, // Covers ISO-8859-1 plus extras, very common
        encoding_rs::ISO_8859_15,  // Latin-9, common in Europe
        encoding_rs::ISO_8859_2,   // Central European
        encoding_rs::UTF_16LE,     // Little-endian UTF-16
        encoding_rs::UTF_16BE,     // Big-endian UTF-16
    ];

    for encoding in &encodings_to_try {
        let (decoded, _, had_errors) = encoding.decode(data);
        if !had_errors {
            return decoded.into_owned();
        }
    }

    // If all else fails, use lossy UTF-8 conversion
    String::from_utf8_lossy(data).into_owned()
}

impl Response {
    /// Parse response from server bytes with automatic encoding detection
    ///
    /// This method automatically detects and converts various text encodings
    /// to UTF-8, including UTF-8, Windows-1252, ISO-8859-15, and others.
    /// This ensures compatibility with NNTP servers that send responses in
    /// different character encodings.
    pub fn parse(data: &[u8]) -> Result<Self> {
        let response_text = decode_text_with_encoding(data);
        Self::parse_str(&response_text)
    }

    /// Parse response from string
    pub fn parse_str(response: &str) -> Result<Self> {
        let lines: Vec<&str> = response.lines().collect();
        if lines.is_empty() {
            return Err(Error::Parse("Empty response".to_string()));
        }

        let status_line = lines[0];
        let (code, message) = parse_status_line(status_line)?;

        match code {
            100 => {
                // Help information
                let help_lines = lines[1..]
                    .iter()
                    .take_while(|line| **line != ".")
                    .map(|line| line.to_string())
                    .collect();
                Ok(Response::Help(help_lines))
            }
            101 => {
                // Capabilities list
                let capabilities = lines[1..]
                    .iter()
                    .take_while(|line| **line != ".")
                    .map(|line| line.to_string())
                    .collect();
                Ok(Response::Capabilities(capabilities))
            }
            111 => {
                // Server date
                Ok(Response::Date(message))
            }
            200 => Ok(Response::ModeReader {
                posting_allowed: true,
            }),
            201 => Ok(Response::ModeReader {
                posting_allowed: false,
            }),
            205 => Ok(Response::Quit),
            211 => {
                // Could be group selection or article listing
                if message.contains("list follows") {
                    // Article listing
                    let articles = lines[1..]
                        .iter()
                        .take_while(|line| **line != ".")
                        .filter_map(|line| line.parse::<u64>().ok())
                        .collect();
                    Ok(Response::ArticleListing(articles))
                } else {
                    // Group selection
                    parse_group_response(&message)
                }
            }
            215 => {
                // Could be newsgroup list or overview format
                if message.to_lowercase().contains("overview") {
                    // Overview format list
                    let format_fields = lines[1..]
                        .iter()
                        .take_while(|line| **line != ".")
                        .map(|line| line.to_string())
                        .collect();
                    Ok(Response::OverviewFormat(format_fields))
                } else {
                    // Newsgroup list
                    let groups = lines[1..]
                        .iter()
                        .take_while(|line| **line != ".")
                        .filter_map(|line| parse_newsgroup_line(line))
                        .collect();
                    Ok(Response::NewsgroupList(groups))
                }
            }
            220..=222 => {
                // Article content
                parse_article_response(code, &message, &lines[1..])
            }
            223 => {
                // Article status
                parse_article_status(&message)
            }
            224 => {
                // Overview data
                let overview = lines[1..]
                    .iter()
                    .take_while(|line| **line != ".")
                    .filter_map(|line| parse_overview_entry(line))
                    .collect();
                Ok(Response::OverviewData(overview))
            }
            225 => {
                // Header data
                let headers = lines[1..]
                    .iter()
                    .take_while(|line| **line != ".")
                    .filter_map(|line| parse_header_entry(line))
                    .collect();
                Ok(Response::HeaderData(headers))
            }
            230 => {
                // New articles
                let articles = lines[1..]
                    .iter()
                    .take_while(|line| **line != ".")
                    .map(|line| line.to_string())
                    .collect();
                Ok(Response::NewArticles(articles))
            }
            231 => {
                // New newsgroups
                let groups = lines[1..]
                    .iter()
                    .take_while(|line| **line != ".")
                    .filter_map(|line| parse_newsgroup_line(line))
                    .collect();
                Ok(Response::NewNewsgroups(groups))
            }
            235 => Ok(Response::ArticleTransferred),
            240 => Ok(Response::PostSuccess),
            281 => Ok(Response::AuthSuccess),
            335 => Ok(Response::ArticleWanted),
            340 => Ok(Response::PostAccepted),
            381 => Ok(Response::AuthRequired),
            382 => Ok(Response::TlsReady),
            435 | 436 => Ok(Response::ArticleNotWanted),
            483 => Ok(Response::TlsNotAvailable { message }),
            // All 4xx and 5xx error codes use the unified Error variant
            400..=599 => Ok(Response::Error { code, message }),
            _ => {
                if (200..400).contains(&code) {
                    Ok(Response::Success { code, message })
                } else {
                    Ok(Response::Error { code, message })
                }
            }
        }
    }

    /// Parse article content as an email message (only applicable to Article responses)
    ///
    /// Returns a parsed message if this is an Article response and the content
    /// can be successfully parsed as an email message.
    pub fn parsed_message(&self) -> Option<Message<'_>> {
        match self {
            Response::Article { content, .. } => MessageParser::default().parse(content),
            _ => None,
        }
    }

    /// Get subject from article content (only applicable to Article responses)
    ///
    /// This is a convenience method that parses the article content and extracts
    /// the subject field.
    pub fn article_subject(&self) -> Option<String> {
        self.parsed_message()?.subject().map(|s| s.to_string())
    }

    /// Get sender from article content (only applicable to Article responses)  
    ///
    /// This is a convenience method that parses the article content and extracts
    /// the from field.
    pub fn article_from(&self) -> Option<String> {
        self.parsed_message()?
            .from()?
            .first()?
            .address()
            .map(|s| s.to_string())
    }

    /// Get body text from article content (only applicable to Article responses)
    ///
    /// This is a convenience method that parses the article content and extracts
    /// the body text.
    pub fn article_body(&self) -> Option<String> {
        self.parsed_message()?.body_text(0).map(|s| s.to_string())
    }

    // ===== Error checking helper methods =====

    /// Check if this is an error response.
    pub fn is_error(&self) -> bool {
        matches!(self, Response::Error { .. })
    }

    /// Get the error code if this is an error response.
    pub fn error_code(&self) -> Option<u16> {
        match self {
            Response::Error { code, .. } => Some(*code),
            _ => None,
        }
    }

    /// Get the error message if this is an error response.
    pub fn error_message(&self) -> Option<&str> {
        match self {
            Response::Error { message, .. } => Some(message),
            _ => None,
        }
    }

    /// Check if this is a "service discontinued" error (400).
    pub fn is_service_discontinued(&self) -> bool {
        matches!(self, Response::Error { code: 400, .. })
    }

    /// Check if this is a "no such newsgroup" error (411).
    pub fn is_no_such_newsgroup(&self) -> bool {
        matches!(self, Response::Error { code: 411, .. })
    }

    /// Check if this is a "no newsgroup selected" error (412).
    pub fn is_no_newsgroup_selected(&self) -> bool {
        matches!(self, Response::Error { code: 412, .. })
    }

    /// Check if this is a "no current article" error (420).
    pub fn is_no_current_article(&self) -> bool {
        matches!(self, Response::Error { code: 420, .. })
    }

    /// Check if this is a "no next article" error (421).
    pub fn is_no_next_article(&self) -> bool {
        matches!(self, Response::Error { code: 421, .. })
    }

    /// Check if this is a "no previous article" error (422).
    pub fn is_no_previous_article(&self) -> bool {
        matches!(self, Response::Error { code: 422, .. })
    }

    /// Check if this is a "no such article" error (430).
    pub fn is_no_such_article(&self) -> bool {
        matches!(self, Response::Error { code: 430, .. })
    }

    /// Check if this is an "authentication required" error (480).
    pub fn is_auth_required(&self) -> bool {
        matches!(self, Response::Error { code: 480, .. })
    }

    /// Check if this is a "command not recognized" error (500).
    pub fn is_command_not_recognized(&self) -> bool {
        matches!(self, Response::Error { code: 500, .. })
    }

    /// Check if this is a "command syntax error" (501).
    pub fn is_command_syntax_error(&self) -> bool {
        matches!(self, Response::Error { code: 501, .. })
    }

    /// Check if this is an "access denied" error (502).
    pub fn is_access_denied(&self) -> bool {
        matches!(self, Response::Error { code: 502, .. })
    }

    /// Check if this is a "program fault" error (503).
    pub fn is_program_fault(&self) -> bool {
        matches!(self, Response::Error { code: 503, .. })
    }
}

fn parse_status_line(line: &str) -> Result<(u16, String)> {
    let parts: Vec<&str> = line.splitn(2, ' ').collect();
    if parts.len() < 2 {
        return Err(Error::Parse(format!("Invalid status line: {line}")));
    }

    let code = parts[0]
        .parse::<u16>()
        .map_err(|_| Error::Parse(format!("Invalid response code: {}", parts[0])))?;
    let message = parts[1].to_string();

    Ok((code, message))
}

fn parse_group_response(message: &str) -> Result<Response> {
    let parts: Vec<&str> = message.split_whitespace().collect();
    if parts.len() < 4 {
        return Err(Error::Parse(format!("Invalid group response: {message}")));
    }

    let count = parts[0]
        .parse::<u64>()
        .map_err(|_| Error::Parse("Invalid article count".to_string()))?;
    let first = parts[1]
        .parse::<u64>()
        .map_err(|_| Error::Parse("Invalid first article number".to_string()))?;
    let last = parts[2]
        .parse::<u64>()
        .map_err(|_| Error::Parse("Invalid last article number".to_string()))?;
    let name = parts[3].to_string();

    Ok(Response::GroupSelected {
        count,
        first,
        last,
        name,
    })
}

fn parse_article_response(_code: u16, message: &str, content_lines: &[&str]) -> Result<Response> {
    let parts: Vec<&str> = message.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(Error::Parse(format!("Invalid article response: {message}")));
    }

    let number = if parts[0] == "0" {
        None
    } else {
        Some(
            parts[0]
                .parse::<u64>()
                .map_err(|_| Error::Parse("Invalid article number".to_string()))?,
        )
    };
    let message_id = parts[1].to_string();

    // Collect content until terminator dot
    let mut content = Vec::new();
    for line in content_lines {
        if *line == "." {
            break;
        }
        content.extend_from_slice(line.as_bytes());
        content.extend_from_slice(b"\r\n");
    }

    Ok(Response::Article {
        number,
        message_id,
        content,
    })
}

fn parse_article_status(message: &str) -> Result<Response> {
    let parts: Vec<&str> = message.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(Error::Parse(format!("Invalid status response: {message}")));
    }

    let number = parts[0]
        .parse::<u64>()
        .map_err(|_| Error::Parse("Invalid article number".to_string()))?;
    let message_id = parts[1].to_string();

    Ok(Response::ArticleStatus { number, message_id })
}

fn parse_newsgroup_line(line: &str) -> Option<NewsGroup> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 4 {
        return None;
    }

    let name = parts[0].to_string();
    let last = parts[1].parse::<u64>().ok()?;
    let first = parts[2].parse::<u64>().ok()?;
    let posting_status = parts[3].chars().next()?;

    Some(NewsGroup {
        name,
        last,
        first,
        posting_status,
    })
}

fn parse_header_entry(line: &str) -> Option<HeaderEntry> {
    let parts: Vec<&str> = line.splitn(2, ' ').collect();
    if parts.len() < 2 {
        return None;
    }

    Some(HeaderEntry {
        article: parts[0].to_string(),
        value: parts[1].to_string(),
    })
}

fn parse_overview_entry(line: &str) -> Option<OverviewEntry> {
    let parts: Vec<String> = line.split('\t').map(|s| s.to_string()).collect();
    if parts.is_empty() {
        return None;
    }

    Some(OverviewEntry { fields: parts })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_capabilities() {
        let response = "101 Capability list:\r\nVERSION 2\r\nREADER\r\nIHAVE\r\n.\r\n";
        let parsed = Response::parse_str(response).unwrap();

        if let Response::Capabilities(caps) = parsed {
            assert_eq!(caps.len(), 3);
            assert_eq!(caps[0], "VERSION 2");
            assert_eq!(caps[1], "READER");
            assert_eq!(caps[2], "IHAVE");
        } else {
            panic!("Expected Capabilities response");
        }
    }

    #[test]
    fn test_parse_group_selected() {
        let response = "211 1234 3000 4234 misc.test";
        let parsed = Response::parse_str(response).unwrap();

        if let Response::GroupSelected {
            count,
            first,
            last,
            name,
        } = parsed
        {
            assert_eq!(count, 1234);
            assert_eq!(first, 3000);
            assert_eq!(last, 4234);
            assert_eq!(name, "misc.test");
        } else {
            panic!("Expected GroupSelected response");
        }
    }

    #[test]
    fn test_parse_error_response() {
        let response = "500 Command not recognized";
        let parsed = Response::parse_str(response).unwrap();

        if let Response::Error { code, message } = parsed {
            assert_eq!(code, 500);
            assert_eq!(message, "Command not recognized");
        } else {
            panic!("Expected Error response");
        }
    }

    #[test]
    fn test_parse_help_response() {
        let response = "100 Help text follows\r\nCAPABILITIES\r\nMODE READER\r\nGROUP\r\n.\r\n";
        let parsed = Response::parse_str(response).unwrap();

        if let Response::Help(help_lines) = parsed {
            assert_eq!(help_lines.len(), 3);
            assert_eq!(help_lines[0], "CAPABILITIES");
            assert_eq!(help_lines[1], "MODE READER");
            assert_eq!(help_lines[2], "GROUP");
        } else {
            panic!("Expected Help response");
        }
    }

    #[test]
    fn test_parse_date_response() {
        let response = "111 20231106123456";
        let parsed = Response::parse_str(response).unwrap();

        if let Response::Date(date) = parsed {
            assert_eq!(date, "20231106123456");
        } else {
            panic!("Expected Date response");
        }
    }

    #[test]
    fn test_parse_header_data_response() {
        let response = "225 Header follows\r\n3000 I am just a test article\r\n3001 Another test article\r\n.\r\n";
        let parsed = Response::parse_str(response).unwrap();

        if let Response::HeaderData(headers) = parsed {
            assert_eq!(headers.len(), 2);
            assert_eq!(headers[0].article, "3000");
            assert_eq!(headers[0].value, "I am just a test article");
            assert_eq!(headers[1].article, "3001");
            assert_eq!(headers[1].value, "Another test article");
        } else {
            panic!("Expected HeaderData response");
        }
    }

    #[test]
    fn test_parse_overview_data_response() {
        let response = "224 Overview information follows\r\n3000\tI am just a test article\tdemo@example.com\t6 Oct 1998 04:38:40 -0500\t<45223423@example.com>\t\t1234\t42\r\n.\r\n";
        let parsed = Response::parse_str(response).unwrap();

        if let Response::OverviewData(overview) = parsed {
            assert_eq!(overview.len(), 1);
            assert_eq!(overview[0].number(), Some(3000));
            assert_eq!(overview[0].subject(), Some("I am just a test article"));
            assert_eq!(overview[0].from(), Some("demo@example.com"));
            assert_eq!(overview[0].message_id(), Some("<45223423@example.com>"));
            assert_eq!(overview[0].byte_count(), Some(1234));
            assert_eq!(overview[0].line_count(), Some(42));
        } else {
            panic!("Expected OverviewData response");
        }
    }

    #[test]
    fn test_parse_overview_format_response() {
        let response = "215 Order of fields in overview database.\r\nSubject:\r\nFrom:\r\nDate:\r\nMessage-ID:\r\nReferences:\r\nBytes:\r\nLines:\r\n.\r\n";
        let parsed = Response::parse_str(response).unwrap();

        if let Response::OverviewFormat(format_fields) = parsed {
            assert_eq!(format_fields.len(), 7);
            assert_eq!(format_fields[0], "Subject:");
            assert_eq!(format_fields[1], "From:");
            assert_eq!(format_fields[2], "Date:");
            assert_eq!(format_fields[3], "Message-ID:");
            assert_eq!(format_fields[4], "References:");
            assert_eq!(format_fields[5], "Bytes:");
            assert_eq!(format_fields[6], "Lines:");
        } else {
            panic!("Expected OverviewFormat response");
        }
    }

    #[test]
    fn test_article_response_parsing_methods() {
        // Create an Article response with realistic email content
        let article_response = Response::Article {
            number: Some(3000),
            message_id: "<45223423@example.com>".to_string(),
            content: b"From: \"Demo User\" <nobody@example.com>\r\nNewsgroups: misc.test\r\nSubject: I am just a test article\r\nDate: Wed, 06 Oct 1998 04:38:40 -0500\r\n\r\nThis is just a test article body.\r\n".to_vec(),
        };

        // Test parsing methods
        assert!(article_response.parsed_message().is_some());

        let subject = article_response.article_subject();
        assert_eq!(subject, Some("I am just a test article".to_string()));

        let from = article_response.article_from();
        assert_eq!(from, Some("nobody@example.com".to_string()));

        let body = article_response.article_body();
        assert_eq!(
            body,
            Some("This is just a test article body.\r\n".to_string())
        );

        // Test with non-Article response
        let other_response = Response::Quit;
        assert!(other_response.parsed_message().is_none());
        assert!(other_response.article_subject().is_none());
        assert!(other_response.article_from().is_none());
        assert!(other_response.article_body().is_none());
    }

    #[test]
    fn test_encoding_detection_utf8() {
        // Test UTF-8 encoding (should work as before)
        let utf8_data = "101 Capability list:\r\nVERSION 2\r\nREADER\r\n.\r\n".as_bytes();
        let response = Response::parse(utf8_data).unwrap();

        if let Response::Capabilities(caps) = response {
            assert_eq!(caps.len(), 2);
            assert_eq!(caps[0], "VERSION 2");
            assert_eq!(caps[1], "READER");
        } else {
            panic!("Expected Capabilities response");
        }
    }

    #[test]
    fn test_encoding_detection_windows1252() {
        // Test Windows-1252 encoding with special characters
        // Windows-1252 byte 0x80 = Euro sign (€), 0x85 = ellipsis (…)
        let mut win1252_data = Vec::new();
        win1252_data.extend_from_slice(b"200 Welcome to the news server ");
        win1252_data.push(0x80); // Euro sign in Windows-1252
        win1252_data.extend_from_slice(b" ");
        win1252_data.push(0x85); // Ellipsis in Windows-1252
        win1252_data.extend_from_slice(b"\r\n");

        let response = Response::parse(&win1252_data).unwrap();

        if let Response::ModeReader { posting_allowed } = response {
            assert!(posting_allowed);
        } else {
            panic!("Expected ModeReader response");
        }
    }

    #[test]
    fn test_encoding_detection_iso_8859_15() {
        // Test ISO-8859-15 encoding with special characters
        // ISO-8859-15 byte 0xA4 = Euro sign (€)
        let mut iso_data = Vec::new();
        iso_data.extend_from_slice(b"211 100 1 100 test.group ");
        iso_data.push(0xA4); // Euro sign in ISO-8859-15
        iso_data.extend_from_slice(b"\r\n");

        let response = Response::parse(&iso_data).unwrap();

        if let Response::GroupSelected {
            count,
            first,
            last,
            name,
        } = response
        {
            assert_eq!(count, 100);
            assert_eq!(first, 1);
            assert_eq!(last, 100);
            assert!(name.contains("test.group"));
        } else {
            panic!("Expected GroupSelected response");
        }
    }

    #[test]
    fn test_encoding_detection_invalid_utf8() {
        // Test data that is not valid UTF-8 but can be decoded with fallback
        let invalid_utf8 = vec![
            b'5', b'0', b'0', b' ', 0xFF, 0xFE, // Invalid UTF-8 byte sequence
            b' ', b'E', b'r', b'r', b'o', b'r', b'\r', b'\n',
        ];

        let response = Response::parse(&invalid_utf8).unwrap();

        // The encoding system should handle the invalid UTF-8 bytes gracefully
        if let Response::Error { code, message } = response {
            assert_eq!(code, 500);
            // The message should contain some representation of the invalid bytes
            // which got converted to valid UTF-8 characters (replacement chars or similar)
            assert!(message.contains("Error"));
        } else {
            panic!(
                "Expected Error response, got: {:?}",
                response
            );
        }
    }

    #[test]
    fn test_encoding_detection_mixed_content() {
        // Test multiline response with potential encoding issues
        let mut mixed_data = Vec::new();
        mixed_data.extend_from_slice(b"215 Newsgroups follow:\r\n");
        mixed_data.extend_from_slice(b"comp.lang.rust 1000 1 y\r\n");
        // Add some ISO-8859-15 specific characters
        mixed_data.extend_from_slice(b"de.test.");
        mixed_data.push(0xA4); // Euro sign in ISO-8859-15
        mixed_data.extend_from_slice(b" 50 1 n\r\n");
        mixed_data.extend_from_slice(b".\r\n");

        let response = Response::parse(&mixed_data).unwrap();

        if let Response::NewsgroupList(groups) = response {
            assert_eq!(groups.len(), 2);
            assert_eq!(groups[0].name, "comp.lang.rust");
            assert!(groups[1].name.starts_with("de.test."));
        } else {
            panic!("Expected NewsgroupList response");
        }
    }

    #[test]
    fn test_decode_text_with_encoding_direct() {
        // Test the decode_text_with_encoding function directly

        // UTF-8 text should work fine
        let utf8_text = "Hello, 世界!";
        let utf8_bytes = utf8_text.as_bytes();
        assert_eq!(decode_text_with_encoding(utf8_bytes), utf8_text);

        // Windows-1252 with special characters
        let win1252_bytes = vec![0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x20, 0x80]; // "Hello €"
        let decoded = decode_text_with_encoding(&win1252_bytes);
        assert_eq!(decoded, "Hello €");

        // Test fallback with completely invalid data
        let invalid_bytes = vec![0xFF, 0xFE, 0xFD];
        let decoded = decode_text_with_encoding(&invalid_bytes);
        // Should not panic and should return some string
        assert!(!decoded.is_empty());
    }

    #[test]
    fn test_parse_tls_ready_response() {
        let response = "382 Continue with TLS negotiation";
        let parsed = Response::parse_str(response).unwrap();

        assert_eq!(parsed, Response::TlsReady);
    }

    #[test]
    fn test_parse_tls_not_available_response() {
        let response = "483 TLS temporarily not available";
        let parsed = Response::parse_str(response).unwrap();

        if let Response::TlsNotAvailable { message } = parsed {
            assert_eq!(message, "TLS temporarily not available");
        } else {
            panic!("Expected TlsNotAvailable response");
        }
    }

    #[test]
    fn test_error_helper_methods() {
        // Test is_error
        let error = Response::Error { code: 500, message: "test".to_string() };
        assert!(error.is_error());
        assert!(!Response::Quit.is_error());
        
        // Test error_code
        assert_eq!(error.error_code(), Some(500));
        assert_eq!(Response::Quit.error_code(), None);
        
        // Test error_message
        assert_eq!(error.error_message(), Some("test"));
        assert_eq!(Response::Quit.error_message(), None);
    }

    #[test]
    fn test_is_service_discontinued() {
        let error = Response::Error { code: 400, message: "Service discontinued".to_string() };
        assert!(error.is_service_discontinued());
        
        let other = Response::Error { code: 500, message: "Other".to_string() };
        assert!(!other.is_service_discontinued());
    }

    #[test]
    fn test_is_no_such_newsgroup() {
        let error = Response::Error { code: 411, message: "No such newsgroup".to_string() };
        assert!(error.is_no_such_newsgroup());
        
        let other = Response::Error { code: 412, message: "Other".to_string() };
        assert!(!other.is_no_such_newsgroup());
    }

    #[test]
    fn test_is_no_newsgroup_selected() {
        let error = Response::Error { code: 412, message: "No newsgroup selected".to_string() };
        assert!(error.is_no_newsgroup_selected());
        
        let other = Response::Error { code: 411, message: "Other".to_string() };
        assert!(!other.is_no_newsgroup_selected());
    }

    #[test]
    fn test_is_no_current_article() {
        let error = Response::Error { code: 420, message: "No current article".to_string() };
        assert!(error.is_no_current_article());
    }

    #[test]
    fn test_is_no_next_article() {
        let error = Response::Error { code: 421, message: "No next article".to_string() };
        assert!(error.is_no_next_article());
    }

    #[test]
    fn test_is_no_previous_article() {
        let error = Response::Error { code: 422, message: "No previous article".to_string() };
        assert!(error.is_no_previous_article());
    }

    #[test]
    fn test_is_no_such_article() {
        let error = Response::Error { code: 430, message: "No such article".to_string() };
        assert!(error.is_no_such_article());
    }

    #[test]
    fn test_is_auth_required() {
        let error = Response::Error { code: 480, message: "Authentication required".to_string() };
        assert!(error.is_auth_required());
    }

    #[test]
    fn test_is_command_not_recognized() {
        let error = Response::Error { code: 500, message: "Command not recognized".to_string() };
        assert!(error.is_command_not_recognized());
    }

    #[test]
    fn test_is_command_syntax_error() {
        let error = Response::Error { code: 501, message: "Syntax error".to_string() };
        assert!(error.is_command_syntax_error());
    }

    #[test]
    fn test_is_access_denied() {
        let error = Response::Error { code: 502, message: "Access denied".to_string() };
        assert!(error.is_access_denied());
    }

    #[test]
    fn test_is_program_fault() {
        let error = Response::Error { code: 503, message: "Program fault".to_string() };
        assert!(error.is_program_fault());
    }

    #[test]
    fn test_parse_empty_response() {
        let result = Response::parse_str("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_status_line_no_space() {
        let result = Response::parse_str("200");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_response_code() {
        let result = Response::parse_str("ABC Invalid code");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_mode_reader_posting_allowed() {
        let response = "200 Reader mode, posting allowed";
        let parsed = Response::parse_str(response).unwrap();
        assert_eq!(parsed, Response::ModeReader { posting_allowed: true });
    }

    #[test]
    fn test_parse_mode_reader_posting_prohibited() {
        let response = "201 Reader mode, posting prohibited";
        let parsed = Response::parse_str(response).unwrap();
        assert_eq!(parsed, Response::ModeReader { posting_allowed: false });
    }

    #[test]
    fn test_parse_quit_response() {
        let response = "205 Goodbye";
        let parsed = Response::parse_str(response).unwrap();
        assert_eq!(parsed, Response::Quit);
    }

    #[test]
    fn test_parse_article_listing() {
        let response = "211 list follows\r\n3000\r\n3001\r\n3002\r\n.\r\n";
        let parsed = Response::parse_str(response).unwrap();

        if let Response::ArticleListing(articles) = parsed {
            assert_eq!(articles, vec![3000, 3001, 3002]);
        } else {
            panic!("Expected ArticleListing response");
        }
    }

    #[test]
    fn test_parse_article_with_number_zero() {
        let response = "220 0 <test@example.com>\r\nSubject: Test\r\n\r\nBody\r\n.\r\n";
        let parsed = Response::parse_str(response).unwrap();

        if let Response::Article { number, message_id, .. } = parsed {
            assert_eq!(number, None); // 0 means no article number
            assert_eq!(message_id, "<test@example.com>");
        } else {
            panic!("Expected Article response");
        }
    }

    #[test]
    fn test_parse_article_head_response() {
        let response = "221 123 <test@example.com>\r\nSubject: Test\r\nFrom: user@example.com\r\n.\r\n";
        let parsed = Response::parse_str(response).unwrap();

        if let Response::Article { number, message_id, content } = parsed {
            assert_eq!(number, Some(123));
            assert_eq!(message_id, "<test@example.com>");
            assert!(content.len() > 0);
        } else {
            panic!("Expected Article response");
        }
    }

    #[test]
    fn test_parse_article_body_response() {
        let response = "222 123 <test@example.com>\r\nThis is the body.\r\n.\r\n";
        let parsed = Response::parse_str(response).unwrap();

        if let Response::Article { number, message_id, content } = parsed {
            assert_eq!(number, Some(123));
            assert_eq!(message_id, "<test@example.com>");
            assert!(!content.is_empty());
        } else {
            panic!("Expected Article response");
        }
    }

    #[test]
    fn test_parse_article_status_response() {
        let response = "223 3000 <45223423@example.com>";
        let parsed = Response::parse_str(response).unwrap();

        if let Response::ArticleStatus { number, message_id } = parsed {
            assert_eq!(number, 3000);
            assert_eq!(message_id, "<45223423@example.com>");
        } else {
            panic!("Expected ArticleStatus response");
        }
    }

    #[test]
    fn test_parse_new_articles_response() {
        let response = "230 New articles follow\r\n<msg1@example.com>\r\n<msg2@example.com>\r\n.\r\n";
        let parsed = Response::parse_str(response).unwrap();

        if let Response::NewArticles(articles) = parsed {
            assert_eq!(articles.len(), 2);
            assert_eq!(articles[0], "<msg1@example.com>");
            assert_eq!(articles[1], "<msg2@example.com>");
        } else {
            panic!("Expected NewArticles response");
        }
    }

    #[test]
    fn test_parse_new_newsgroups_response() {
        let response = "231 New newsgroups follow\r\nalt.test 100 1 y\r\ncomp.test 50 1 n\r\n.\r\n";
        let parsed = Response::parse_str(response).unwrap();

        if let Response::NewNewsgroups(groups) = parsed {
            assert_eq!(groups.len(), 2);
            assert_eq!(groups[0].name, "alt.test");
            assert_eq!(groups[1].name, "comp.test");
        } else {
            panic!("Expected NewNewsgroups response");
        }
    }

    #[test]
    fn test_parse_article_transferred_response() {
        let response = "235 Article transferred OK";
        let parsed = Response::parse_str(response).unwrap();
        assert_eq!(parsed, Response::ArticleTransferred);
    }

    #[test]
    fn test_parse_post_success_response() {
        let response = "240 Article posted OK";
        let parsed = Response::parse_str(response).unwrap();
        assert_eq!(parsed, Response::PostSuccess);
    }

    #[test]
    fn test_parse_auth_success_response() {
        let response = "281 Authentication accepted";
        let parsed = Response::parse_str(response).unwrap();
        assert_eq!(parsed, Response::AuthSuccess);
    }

    #[test]
    fn test_parse_article_wanted_response() {
        let response = "335 Send article";
        let parsed = Response::parse_str(response).unwrap();
        assert_eq!(parsed, Response::ArticleWanted);
    }

    #[test]
    fn test_parse_post_accepted_response() {
        let response = "340 Send article";
        let parsed = Response::parse_str(response).unwrap();
        assert_eq!(parsed, Response::PostAccepted);
    }

    #[test]
    fn test_parse_auth_required_response() {
        let response = "381 Password required";
        let parsed = Response::parse_str(response).unwrap();
        assert_eq!(parsed, Response::AuthRequired);
    }

    #[test]
    fn test_parse_article_not_wanted_435() {
        let response = "435 Article not wanted";
        let parsed = Response::parse_str(response).unwrap();
        assert_eq!(parsed, Response::ArticleNotWanted);
    }

    #[test]
    fn test_parse_article_not_wanted_436() {
        let response = "436 Transfer denied";
        let parsed = Response::parse_str(response).unwrap();
        assert_eq!(parsed, Response::ArticleNotWanted);
    }

    #[test]
    fn test_parse_success_generic() {
        let response = "282 Some custom success";
        let parsed = Response::parse_str(response).unwrap();

        if let Response::Success { code, message } = parsed {
            assert_eq!(code, 282);
            assert_eq!(message, "Some custom success");
        } else {
            panic!("Expected Success response");
        }
    }

    #[test]
    fn test_parse_error_4xx() {
        let response = "440 Posting not permitted";
        let parsed = Response::parse_str(response).unwrap();

        if let Response::Error { code, message } = parsed {
            assert_eq!(code, 440);
            assert_eq!(message, "Posting not permitted");
        } else {
            panic!("Expected Error response");
        }
    }

    #[test]
    fn test_parse_error_5xx() {
        let response = "502 Access denied";
        let parsed = Response::parse_str(response).unwrap();

        if let Response::Error { code, message } = parsed {
            assert_eq!(code, 502);
            assert_eq!(message, "Access denied");
        } else {
            panic!("Expected Error response");
        }
    }

    #[test]
    fn test_parse_invalid_group_response() {
        let response = "211 incomplete";
        let result = Response::parse_str(response);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_article_response() {
        let response = "220 incomplete";
        let result = Response::parse_str(response);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_article_status_response() {
        let response = "223 incomplete";
        let result = Response::parse_str(response);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_newsgroup_line_incomplete() {
        // Internal function test
        let result = parse_newsgroup_line("incomplete");
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_header_entry_incomplete() {
        // Internal function test
        let result = parse_header_entry("incomplete");
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_overview_entry_empty() {
        // Internal function test - empty string still creates an entry with empty fields
        let result = parse_overview_entry("");
        // An empty line creates an OverviewEntry with one empty field
        assert!(result.is_some());
        let entry = result.unwrap();
        assert!(entry.fields.is_empty() || entry.fields[0].is_empty());
    }

    #[test]
    fn test_response_error_code_range() {
        // Test error code at the edge of range
        let response = "599 Maximum error code";
        let parsed = Response::parse_str(response).unwrap();
        assert!(matches!(parsed, Response::Error { code: 599, .. }));
    }
}
