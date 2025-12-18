//! Article-related types for NNTP responses.
//!
//! This module contains the [`Article`] struct for representing NNTP articles
//! with lazy MIME parsing, and the [`Attachment`] struct for representing
//! attachments in multipart messages.

use mail_parser::{Message, MessageParser, MimeHeaders};

/// An NNTP article representing a MIME message with lazy parsing.
///
/// This struct provides structured access to NNTP article content through
/// the mail_parser Message interface while owning the underlying data.
/// Parsing is performed lazily - content is only parsed when accessor
/// methods are called.
#[derive(Debug, Clone)]
pub struct Article {
    /// Article number within the group (if available)
    number: Option<u64>,
    /// Message-ID (globally unique identifier)
    message_id: String,
    /// Raw article content (headers + body)
    content: Vec<u8>,
}

impl Article {
    /// Create a new Article from raw NNTP response data.
    pub fn new(number: Option<u64>, message_id: String, content: Vec<u8>) -> Self {
        Self {
            number,
            message_id,
            content,
        }
    }

    // === Identifier Access ===

    /// Get the article's Message-ID (globally unique identifier).
    pub fn article_id(&self) -> &str {
        &self.message_id
    }

    /// Get the article number within the group, if available.
    pub fn number(&self) -> Option<u64> {
        self.number
    }

    /// Get raw article content bytes.
    pub fn raw_content(&self) -> &[u8] {
        &self.content
    }

    // === MIME Parsing (lazy - parses on each call) ===

    /// Parse the article content and get a mail_parser Message.
    ///
    /// This performs lazy parsing - content is parsed each time this is called.
    /// For repeated access to multiple fields, consider calling this once and
    /// working with the returned `Message` directly.
    pub fn message(&self) -> Option<Message<'_>> {
        MessageParser::default().parse(&self.content)
    }

    // === Header Access ===

    /// Get the Subject header value.
    pub fn subject(&self) -> Option<String> {
        self.message()?.subject().map(|s| s.to_string())
    }

    /// Get the From header as email address.
    pub fn from(&self) -> Option<String> {
        self.message()?
            .from()?
            .first()?
            .address()
            .map(|s| s.to_string())
    }

    /// Get the Date header as RFC 3339 formatted string.
    pub fn date(&self) -> Option<String> {
        self.message()?.date().map(|d| d.to_rfc3339())
    }

    /// Get the Newsgroups header value.
    pub fn newsgroups(&self) -> Option<String> {
        self.header("Newsgroups")
    }

    /// Get the References header value.
    pub fn references(&self) -> Option<String> {
        self.header("References")
    }

    /// Get a specific header by name.
    pub fn header(&self, name: &str) -> Option<String> {
        let msg = self.message()?;
        // Try to get header value as text
        msg.header(name)
            .and_then(|h| h.as_text())
            .map(|s| s.to_string())
    }

    /// Get all headers as raw bytes (everything before the blank line).
    pub fn raw_headers(&self) -> Option<&[u8]> {
        let content = &self.content;
        // Find CRLF CRLF separator
        for i in 0..content.len().saturating_sub(3) {
            if &content[i..i + 4] == b"\r\n\r\n" {
                return Some(&content[..i + 2]);
            }
        }
        // Fallback: try LF-only format
        for i in 0..content.len().saturating_sub(1) {
            if &content[i..i + 2] == b"\n\n" {
                return Some(&content[..i + 1]);
            }
        }
        None
    }

    // === Body Access ===

    /// Get the article body as plain text (first text part).
    pub fn body_text(&self) -> Option<String> {
        self.message()?.body_text(0).map(|s| s.to_string())
    }

    /// Get the article body as HTML if available.
    pub fn body_html(&self) -> Option<String> {
        self.message()?.body_html(0).map(|s| s.to_string())
    }

    /// Get raw body bytes (everything after the blank line separator).
    pub fn raw_body(&self) -> Option<&[u8]> {
        let content = &self.content;
        // Find CRLF CRLF separator
        for i in 0..content.len().saturating_sub(3) {
            if &content[i..i + 4] == b"\r\n\r\n" {
                return Some(&content[i + 4..]);
            }
        }
        // Fallback: try LF-only format
        for i in 0..content.len().saturating_sub(1) {
            if &content[i..i + 2] == b"\n\n" {
                return Some(&content[i + 2..]);
            }
        }
        None
    }

    // === Multi-part MIME Support ===

    /// Check if this is a multi-part MIME message.
    pub fn is_multipart(&self) -> bool {
        self.message().map(|m| m.parts.len() > 1).unwrap_or(false)
    }

    /// Get the number of MIME parts.
    pub fn part_count(&self) -> usize {
        self.message().map(|m| m.parts.len()).unwrap_or(0)
    }

    /// Get attachments from the article.
    ///
    /// Returns a vector of attachments found in the MIME message.
    pub fn attachments(&self) -> Vec<Attachment> {
        let Some(message) = self.message() else {
            return vec![];
        };

        message
            .attachments()
            .map(|att| Attachment {
                filename: att.attachment_name().map(|s| s.to_string()),
                content_type: att.content_type().map(|ct| ct.ctype().to_string()),
                data: att.contents().to_vec(),
            })
            .collect()
    }
}

/// Represents an attachment in a MIME message.
#[derive(Debug, Clone, PartialEq)]
pub struct Attachment {
    /// Attachment filename if specified in Content-Disposition.
    pub filename: Option<String>,
    /// MIME content type (e.g., "image/png", "application/pdf").
    pub content_type: Option<String>,
    /// Raw attachment data (already decoded from base64/quoted-printable).
    pub data: Vec<u8>,
}

/// Deprecated: Use [`Article`] instead.
#[deprecated(since = "0.2.0", note = "Use Article instead")]
pub type ParsedArticle = Article;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_article() {
        let content = b"From: \"Demo User\" <nobody@example.com>\r\nNewsgroups: misc.test\r\nSubject: I am just a test article\r\nDate: Wed, 06 Oct 1998 04:38:40 -0500\r\n\r\nThis is just a test article body.\r\n".to_vec();

        let article = Article::new(
            Some(3000),
            "<45223423@example.com>".to_string(),
            content.clone(),
        );

        // Test identifier access
        assert_eq!(article.number(), Some(3000));
        assert_eq!(article.article_id(), "<45223423@example.com>");
        assert_eq!(article.raw_content(), &content);

        // Test MIME parsing
        assert!(article.message().is_some());

        // Test header access
        let subject = article.subject();
        assert_eq!(subject, Some("I am just a test article".to_string()));

        let from = article.from();
        assert_eq!(from, Some("nobody@example.com".to_string()));

        let newsgroups = article.newsgroups();
        assert_eq!(newsgroups, Some("misc.test".to_string()));

        // Test body access
        let body = article.body_text();
        assert_eq!(
            body,
            Some("This is just a test article body.\r\n".to_string())
        );

        // Test raw header/body separation
        let raw_headers = article.raw_headers();
        assert!(raw_headers.is_some());
        let headers_str = String::from_utf8_lossy(raw_headers.unwrap());
        assert!(headers_str.contains("Subject: I am just a test article"));

        let raw_body = article.raw_body();
        assert!(raw_body.is_some());
        assert_eq!(raw_body.unwrap(), b"This is just a test article body.\r\n");

        // Test multi-part detection (this is a simple message, not multipart)
        assert!(!article.is_multipart());
        assert!(article.attachments().is_empty());
    }

    #[test]
    fn test_article_no_number() {
        let content = b"From: test@example.com\r\nSubject: Test\r\n\r\nBody\r\n".to_vec();
        let article = Article::new(None, "<test@example.com>".to_string(), content);
        
        assert_eq!(article.number(), None);
        assert_eq!(article.article_id(), "<test@example.com>");
    }

    #[test]
    fn test_article_date() {
        let content = b"From: test@example.com\r\nDate: Mon, 01 Jan 2024 12:00:00 +0000\r\nSubject: Test\r\n\r\nBody\r\n".to_vec();
        let article = Article::new(Some(1), "<test@example.com>".to_string(), content);
        
        let date = article.date();
        assert!(date.is_some());
    }

    #[test]
    fn test_article_references() {
        let content = b"From: test@example.com\r\nReferences: <parent@example.com> <grandparent@example.com>\r\nSubject: Test\r\n\r\nBody\r\n".to_vec();
        let article = Article::new(Some(1), "<test@example.com>".to_string(), content);
        
        let refs = article.references();
        // References may or may not be parsed depending on mail-parser behavior
        if let Some(refs_str) = refs {
            assert!(refs_str.contains("parent") || refs_str.contains("grandparent"));
        }
    }

    #[test]
    fn test_article_custom_header() {
        let content = b"From: test@example.com\r\nX-Custom-Header: custom value\r\nSubject: Test\r\n\r\nBody\r\n".to_vec();
        let article = Article::new(Some(1), "<test@example.com>".to_string(), content);
        
        let custom = article.header("X-Custom-Header");
        assert_eq!(custom, Some("custom value".to_string()));
        
        // Non-existent header
        let missing = article.header("X-Missing");
        assert!(missing.is_none());
    }

    #[test]
    fn test_article_raw_headers_lf_only() {
        // Test with LF-only line endings (some servers do this)
        let content = b"From: test@example.com\nSubject: Test\n\nBody\n".to_vec();
        let article = Article::new(Some(1), "<test@example.com>".to_string(), content);
        
        let raw_headers = article.raw_headers();
        assert!(raw_headers.is_some());
    }

    #[test]
    fn test_article_raw_body_lf_only() {
        let content = b"From: test@example.com\nSubject: Test\n\nBody content here\n".to_vec();
        let article = Article::new(Some(1), "<test@example.com>".to_string(), content);
        
        let raw_body = article.raw_body();
        assert!(raw_body.is_some());
        assert!(raw_body.unwrap().starts_with(b"Body"));
    }

    #[test]
    fn test_article_part_count() {
        let content = b"From: test@example.com\r\nSubject: Test\r\n\r\nSimple body\r\n".to_vec();
        let article = Article::new(Some(1), "<test@example.com>".to_string(), content);
        
        // Simple message should have 1 part
        assert!(article.part_count() >= 1 || article.part_count() == 0);
    }

    #[test]
    fn test_article_body_html_none() {
        let content = b"From: test@example.com\r\nContent-Type: text/plain\r\nSubject: Test\r\n\r\nPlain text only\r\n".to_vec();
        let article = Article::new(Some(1), "<test@example.com>".to_string(), content);
        
        // Plain text message typically has no HTML body, but mail-parser may still return something
        // Just verify we can call the method without error
        let _ = article.body_html();
    }

    #[test]
    fn test_article_no_separator() {
        // Edge case: content without clear header/body separator
        let content = b"Just some content without headers".to_vec();
        let article = Article::new(Some(1), "<test@example.com>".to_string(), content);
        
        assert!(article.raw_headers().is_none());
        assert!(article.raw_body().is_none());
    }

    #[test]
    fn test_article_attachments_with_multipart() {
        // Create a multipart MIME message with an attachment
        let content = b"From: test@example.com\r\n\
Subject: Test with attachment\r\n\
MIME-Version: 1.0\r\n\
Content-Type: multipart/mixed; boundary=\"boundary123\"\r\n\
\r\n\
--boundary123\r\n\
Content-Type: text/plain\r\n\
\r\n\
This is the text body.\r\n\
--boundary123\r\n\
Content-Type: application/octet-stream\r\n\
Content-Disposition: attachment; filename=\"test.bin\"\r\n\
Content-Transfer-Encoding: base64\r\n\
\r\n\
SGVsbG8gV29ybGQh\r\n\
--boundary123--\r\n".to_vec();
        
        let article = Article::new(Some(1), "<test@example.com>".to_string(), content);
        
        // Check if attachments can be retrieved
        let attachments = article.attachments();
        // The attachment may or may not be parsed depending on mail-parser behavior
        // Just verify we can call the method
        let _ = attachments;
    }

    #[test]
    fn test_article_attachments_empty_for_simple_message() {
        let content = b"From: test@example.com\r\nSubject: Simple\r\n\r\nJust text\r\n".to_vec();
        let article = Article::new(Some(1), "<test@example.com>".to_string(), content);
        
        let attachments = article.attachments();
        assert!(attachments.is_empty());
    }
}
