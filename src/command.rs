//! NNTP command types and encoding.

use crate::error::{Error, Result};

/// NNTP commands that can be sent to the server.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// Request server capabilities
    Capabilities,

    /// Switch to reader mode
    ModeReader,

    /// Authenticate with username
    AuthInfoUser(String),

    /// Authenticate with password  
    AuthInfoPass(String),

    /// Select a newsgroup
    Group(String),

    /// List articles in current group with optional range
    ListGroup(Option<String>),

    /// Retrieve full article by message-id or number
    Article(ArticleSpec),

    /// Retrieve article headers by message-id or number
    Head(ArticleSpec),

    /// Retrieve article body by message-id or number
    Body(ArticleSpec),

    /// Get article status by message-id or number
    Stat(ArticleSpec),

    /// List newsgroups with optional wildcard pattern
    List(Option<String>),

    /// List new newsgroups since date/time
    NewGroups {
        /// Date in YYMMDD or YYYYMMDD format
        date: String,
        /// Time in HHMMSS format
        time: String,
        /// Optional timezone (GMT)
        gmt: bool,
    },

    /// List new articles since date/time
    NewNews {
        /// Wildcard pattern for newsgroups
        wildmat: String,
        /// Date in YYMMDD or YYYYMMDD format  
        date: String,
        /// Time in HHMMSS format
        time: String,
        /// Optional timezone (GMT)
        gmt: bool,
    },

    /// Post an article
    Post,

    /// Terminate connection
    Quit,

    /// Request help information
    Help,

    /// Request server date and time
    Date,

    /// Move to previous article in current group
    Last,

    /// Move to next article in current group
    Next,

    /// Retrieve specific header field for articles
    Hdr {
        /// Header field name (e.g. "Subject", "From")
        field: String,
        /// Range specification (message-id, number, or range)
        range: Option<String>,
    },

    /// Retrieve overview information for articles
    Over {
        /// Range specification (message-id, number, or range)
        range: Option<String>,
    },

    /// Offer an article to the server
    Ihave {
        /// Message-ID of the article being offered
        message_id: String,
    },

    /// List overview format
    ListOverviewFmt,
}

/// Article specification - either message-id or article number
#[derive(Debug, Clone, PartialEq)]
pub enum ArticleSpec {
    /// Article number within current group
    Number(u64),
    /// Message-ID in angle brackets
    MessageId(String),
    /// Current article (no parameter)
    Current,
}

impl Command {
    /// Encode command as bytes for transmission to server
    pub fn encode(&self) -> Result<Vec<u8>> {
        let command_line = match self {
            Command::Capabilities => "CAPABILITIES".to_string(),
            Command::ModeReader => "MODE READER".to_string(),
            Command::AuthInfoUser(user) => {
                validate_parameter(user)?;
                format!("AUTHINFO USER {user}")
            }
            Command::AuthInfoPass(pass) => {
                validate_parameter(pass)?;
                format!("AUTHINFO PASS {pass}")
            }
            Command::Group(group) => {
                validate_parameter(group)?;
                format!("GROUP {group}")
            }
            Command::ListGroup(range) => {
                if let Some(range) = range {
                    validate_parameter(range)?;
                    format!("LISTGROUP {range}")
                } else {
                    "LISTGROUP".to_string()
                }
            }
            Command::Article(spec) => format!("ARTICLE {}", spec.encode()?),
            Command::Head(spec) => format!("HEAD {}", spec.encode()?),
            Command::Body(spec) => format!("BODY {}", spec.encode()?),
            Command::Stat(spec) => format!("STAT {}", spec.encode()?),
            Command::List(pattern) => {
                if let Some(pattern) = pattern {
                    validate_parameter(pattern)?;
                    format!("LIST {pattern}")
                } else {
                    "LIST".to_string()
                }
            }
            Command::NewGroups { date, time, gmt } => {
                validate_parameter(date)?;
                validate_parameter(time)?;
                if *gmt {
                    format!("NEWGROUPS {date} {time} GMT")
                } else {
                    format!("NEWGROUPS {date} {time}")
                }
            }
            Command::NewNews {
                wildmat,
                date,
                time,
                gmt,
            } => {
                validate_parameter(wildmat)?;
                validate_parameter(date)?;
                validate_parameter(time)?;
                if *gmt {
                    format!("NEWNEWS {wildmat} {date} {time} GMT")
                } else {
                    format!("NEWNEWS {wildmat} {date} {time}")
                }
            }
            Command::Post => "POST".to_string(),
            Command::Quit => "QUIT".to_string(),
            Command::Help => "HELP".to_string(),
            Command::Date => "DATE".to_string(),
            Command::Last => "LAST".to_string(),
            Command::Next => "NEXT".to_string(),
            Command::Hdr { field, range } => {
                validate_parameter(field)?;
                if let Some(range) = range {
                    validate_parameter(range)?;
                    format!("HDR {field} {range}")
                } else {
                    format!("HDR {field}")
                }
            }
            Command::Over { range } => {
                if let Some(range) = range {
                    validate_parameter(range)?;
                    format!("OVER {range}")
                } else {
                    "OVER".to_string()
                }
            }
            Command::Ihave { message_id } => {
                if !message_id.starts_with('<') || !message_id.ends_with('>') {
                    return Err(Error::InvalidCommand(
                        "Message-ID must be enclosed in angle brackets".to_string(),
                    ));
                }
                validate_parameter(message_id)?;
                format!("IHAVE {message_id}")
            }
            Command::ListOverviewFmt => "LIST OVERVIEW.FMT".to_string(),
        };

        let mut bytes = command_line.into_bytes();
        bytes.extend_from_slice(b"\r\n");
        Ok(bytes)
    }
}

impl ArticleSpec {
    fn encode(&self) -> Result<String> {
        match self {
            ArticleSpec::Number(num) => Ok(num.to_string()),
            ArticleSpec::MessageId(id) => {
                if !id.starts_with('<') || !id.ends_with('>') {
                    return Err(Error::InvalidCommand(
                        "Message-ID must be enclosed in angle brackets".to_string(),
                    ));
                }
                validate_parameter(id)?;
                Ok(id.clone())
            }
            ArticleSpec::Current => Ok(String::new()),
        }
    }
}

/// Validate that a parameter doesn't contain invalid characters
fn validate_parameter(param: &str) -> Result<()> {
    if param.contains('\r') || param.contains('\n') {
        return Err(Error::InvalidCommand(
            "Parameters cannot contain line breaks".to_string(),
        ));
    }
    if param.is_empty() {
        return Err(Error::InvalidCommand(
            "Parameters cannot be empty".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capabilities_command() {
        let cmd = Command::Capabilities;
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"CAPABILITIES\r\n");
    }

    #[test]
    fn test_group_command() {
        let cmd = Command::Group("alt.test".to_string());
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"GROUP alt.test\r\n");
    }

    #[test]
    fn test_article_by_number() {
        let cmd = Command::Article(ArticleSpec::Number(123));
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"ARTICLE 123\r\n");
    }

    #[test]
    fn test_article_by_message_id() {
        let cmd = Command::Article(ArticleSpec::MessageId("<test@example.com>".to_string()));
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"ARTICLE <test@example.com>\r\n");
    }

    #[test]
    fn test_invalid_parameter() {
        let cmd = Command::Group("test\r\nQUIT".to_string());
        assert!(cmd.encode().is_err());
    }

    #[test]
    fn test_help_command() {
        let cmd = Command::Help;
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"HELP\r\n");
    }

    #[test]
    fn test_date_command() {
        let cmd = Command::Date;
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"DATE\r\n");
    }

    #[test]
    fn test_last_command() {
        let cmd = Command::Last;
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"LAST\r\n");
    }

    #[test]
    fn test_next_command() {
        let cmd = Command::Next;
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"NEXT\r\n");
    }

    #[test]
    fn test_hdr_command_simple() {
        let cmd = Command::Hdr {
            field: "Subject".to_string(),
            range: None,
        };
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"HDR Subject\r\n");
    }

    #[test]
    fn test_hdr_command_with_range() {
        let cmd = Command::Hdr {
            field: "From".to_string(),
            range: Some("1-10".to_string()),
        };
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"HDR From 1-10\r\n");
    }

    #[test]
    fn test_over_command_simple() {
        let cmd = Command::Over { range: None };
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"OVER\r\n");
    }

    #[test]
    fn test_over_command_with_range() {
        let cmd = Command::Over {
            range: Some("3000-3002".to_string()),
        };
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"OVER 3000-3002\r\n");
    }

    #[test]
    fn test_ihave_command() {
        let cmd = Command::Ihave {
            message_id: "<article@example.com>".to_string(),
        };
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"IHAVE <article@example.com>\r\n");
    }

    #[test]
    fn test_ihave_invalid_message_id() {
        let cmd = Command::Ihave {
            message_id: "invalid_id".to_string(),
        };
        assert!(cmd.encode().is_err());
    }

    #[test]
    fn test_list_overview_fmt_command() {
        let cmd = Command::ListOverviewFmt;
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"LIST OVERVIEW.FMT\r\n");
    }
}
