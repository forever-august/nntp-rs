//! NNTP response types and parsing.

use crate::error::{Error, Result};
use mail_parser::{Message, MessageParser};
use std::str;

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

    // RFC 3977 specific error responses
    /// Service discontinued (400)
    ServiceDiscontinued {
        /// Error message from server
        message: String,
    },

    /// No such newsgroup (411)
    NoSuchNewsgroup {
        /// Error message from server
        message: String,
    },

    /// No newsgroup has been selected (412)
    NoNewsgroupSelected {
        /// Error message from server
        message: String,
    },

    /// No current article has been selected (420)
    NoCurrentArticle {
        /// Error message from server
        message: String,
    },

    /// No next article in this group (421)
    NoNextArticle {
        /// Error message from server
        message: String,
    },

    /// No previous article in this group (422)
    NoPreviousArticle {
        /// Error message from server
        message: String,
    },

    /// No such article found (430)
    NoSuchArticle {
        /// Error message from server
        message: String,
    },

    /// Authentication required (480)
    AuthenticationRequired {
        /// Error message from server
        message: String,
    },

    /// Command not recognized (500)
    CommandNotRecognized {
        /// Error message from server
        message: String,
    },

    /// Command syntax error (501)
    CommandSyntaxError {
        /// Error message from server
        message: String,
    },

    /// Access restriction or permission denied (502)
    AccessDenied {
        /// Error message from server
        message: String,
    },

    /// Program fault - command not performed (503)
    ProgramFault {
        /// Error message from server
        message: String,
    },

    /// Protocol error response (4xx/5xx) - for unspecific errors
    Error {
        /// Error code
        code: u16,
        /// Error message
        message: String,
    },
}

/// Header entry for HDR command response
#[derive(Debug, Clone, PartialEq)]
pub struct HeaderEntry {
    /// Article number or message ID
    pub article: String,
    /// Header field value
    pub value: String,
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

impl Response {
    /// Parse response from server bytes
    pub fn parse(data: &[u8]) -> Result<Self> {
        let response_text =
            str::from_utf8(data).map_err(|e| Error::Parse(format!("Invalid UTF-8: {e}")))?;

        Self::parse_str(response_text)
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
            435 | 436 => Ok(Response::ArticleNotWanted),
            // RFC 3977 specific error codes
            400 => Ok(Response::ServiceDiscontinued { message }),
            411 => Ok(Response::NoSuchNewsgroup { message }),
            412 => Ok(Response::NoNewsgroupSelected { message }),
            420 => Ok(Response::NoCurrentArticle { message }),
            421 => Ok(Response::NoNextArticle { message }),
            422 => Ok(Response::NoPreviousArticle { message }),
            430 => Ok(Response::NoSuchArticle { message }),
            480 => Ok(Response::AuthenticationRequired { message }),
            500 => Ok(Response::CommandNotRecognized { message }),
            501 => Ok(Response::CommandSyntaxError { message }),
            502 => Ok(Response::AccessDenied { message }),
            503 => Ok(Response::ProgramFault { message }),
            // All other 4xx and 5xx error codes
            401..=410 | 413..=419 | 423..=429 | 431..=479 | 481..=499 | 504..=599 => {
                Ok(Response::Error { code, message })
            }
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

        if let Response::CommandNotRecognized { message } = parsed {
            assert_eq!(message, "Command not recognized");
        } else {
            panic!("Expected CommandNotRecognized response");
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
}
