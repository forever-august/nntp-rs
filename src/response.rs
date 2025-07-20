//! NNTP response types and parsing.

use crate::error::{Error, Result};
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

    /// Article posted successfully (240)
    PostSuccess,

    /// Connection closing (205)
    Quit,

    /// Generic successful response
    Success {
        /// Response code
        code: u16,
        /// Response message
        message: String,
    },

    /// Protocol error response (4xx/5xx)
    Error {
        /// Error code
        code: u16,
        /// Error message
        message: String,
    },
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
            101 => {
                // Capabilities list
                let capabilities = lines[1..]
                    .iter()
                    .take_while(|line| **line != ".")
                    .map(|line| line.to_string())
                    .collect();
                Ok(Response::Capabilities(capabilities))
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
                // Newsgroup list
                let groups = lines[1..]
                    .iter()
                    .take_while(|line| **line != ".")
                    .filter_map(|line| parse_newsgroup_line(line))
                    .collect();
                Ok(Response::NewsgroupList(groups))
            }
            220..=222 => {
                // Article content
                parse_article_response(code, &message, &lines[1..])
            }
            223 => {
                // Article status
                parse_article_status(&message)
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
            240 => Ok(Response::PostSuccess),
            281 => Ok(Response::AuthSuccess),
            340 => Ok(Response::PostAccepted),
            381 => Ok(Response::AuthRequired),
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
}
