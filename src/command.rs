//! NNTP command types and encoding.

use crate::error::{Error, Result};

/// LIST command variants as specified in RFC 3977
#[derive(Debug, Clone, PartialEq)]
pub enum ListVariant {
    /// LIST ACTIVE \[wildmat\] - Active newsgroups (RFC 3977 Section 7.6.3)
    Active(Option<String>),
    /// LIST NEWSGROUPS \[wildmat\] - Newsgroup descriptions (RFC 3977 Section 7.6.6)
    Newsgroups(Option<String>),
    /// LIST HEADERS - Available header fields for HDR command
    Headers,
    /// LIST ACTIVE.TIMES - Newsgroup creation times (RFC 3977 Section 7.6.4)
    ActiveTimes,
    /// LIST DISTRIBUTIONS - Distribution values (RFC 3977 Section 7.6.5)
    Distributions,
    /// LIST OVERVIEW.FMT - Overview format specification
    OverviewFmt,
    /// LIST \[wildmat\] - Basic list (backwards compatibility)
    Basic(Option<String>),
}

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

    /// List information with specific variants
    List(ListVariant),

    /// List new newsgroups since date/time
    NewGroups {
        /// Date in YYMMDD or YYYYMMDD format
        date: String,
        /// Time in HHMMSS format
        time: String,
        /// Optional timezone (GMT)
        gmt: bool,
        /// Optional distributions parameter
        distributions: Option<String>,
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

    /// STARTTLS command (RFC 4642).
    ///
    /// Initiates TLS negotiation. After receiving a successful response
    /// ([`Response::TlsReady`]), the client should perform a TLS handshake
    /// on the underlying connection.
    ///
    /// # Protocol Notes
    ///
    /// This library provides the protocol-level support for STARTTLS but does
    /// not perform the actual TLS handshake. Callers using the sans-IO layer
    /// should:
    ///
    /// 1. Send the `StartTls` command
    /// 2. Receive and parse the response
    /// 3. If `TlsReady` is received, wrap the underlying stream with a TLS layer
    /// 4. Continue NNTP communication over the now-encrypted connection
    ///
    /// [`Response::TlsReady`]: crate::response::Response::TlsReady
    StartTls,
}

/// Article specification - either message-id or article number within a group
#[derive(Debug, Clone, PartialEq)]
pub enum ArticleSpec {
    /// Article number within a specific group
    GroupNumber {
        /// The newsgroup name
        group: String,
        /// Article number within the group
        article_number: u64,
    },
    /// Message-ID in angle brackets (globally unique)
    MessageId(String),
    /// Current article (no parameter)
    Current,
}

impl ArticleSpec {
    /// Create an ArticleSpec for an article number within a group
    pub fn number_in_group(group: impl Into<String>, number: u64) -> Self {
        Self::GroupNumber {
            group: group.into(),
            article_number: number,
        }
    }
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
            Command::List(variant) => match variant {
                ListVariant::Active(pattern) => {
                    if let Some(pattern) = pattern {
                        validate_parameter(pattern)?;
                        format!("LIST ACTIVE {pattern}")
                    } else {
                        "LIST ACTIVE".to_string()
                    }
                }
                ListVariant::Newsgroups(pattern) => {
                    if let Some(pattern) = pattern {
                        validate_parameter(pattern)?;
                        format!("LIST NEWSGROUPS {pattern}")
                    } else {
                        "LIST NEWSGROUPS".to_string()
                    }
                }
                ListVariant::Headers => "LIST HEADERS".to_string(),
                ListVariant::ActiveTimes => "LIST ACTIVE.TIMES".to_string(),
                ListVariant::Distributions => "LIST DISTRIBUTIONS".to_string(),
                ListVariant::OverviewFmt => "LIST OVERVIEW.FMT".to_string(),
                ListVariant::Basic(pattern) => {
                    if let Some(pattern) = pattern {
                        validate_parameter(pattern)?;
                        format!("LIST {pattern}")
                    } else {
                        "LIST".to_string()
                    }
                }
            },
            Command::NewGroups {
                date,
                time,
                gmt,
                distributions,
            } => {
                validate_parameter(date)?;
                validate_parameter(time)?;
                let mut cmd = if *gmt {
                    format!("NEWGROUPS {date} {time} GMT")
                } else {
                    format!("NEWGROUPS {date} {time}")
                };
                if let Some(dist) = distributions {
                    validate_parameter(dist)?;
                    cmd.push_str(&format!(" {dist}"));
                }
                cmd
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
            Command::StartTls => "STARTTLS".to_string(),
        };

        // RFC 3977: Command lines MUST NOT exceed 512 octets including CRLF
        validate_command_length(&command_line)?;

        let mut bytes = command_line.into_bytes();
        bytes.extend_from_slice(b"\r\n");
        Ok(bytes)
    }
}

impl ArticleSpec {
    fn encode(&self) -> Result<String> {
        match self {
            ArticleSpec::GroupNumber { article_number, .. } => Ok(article_number.to_string()),
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

/// Validate command length according to RFC 3977
/// Command lines MUST NOT exceed 512 octets, which includes the terminating CRLF pair
fn validate_command_length(command: &str) -> Result<()> {
    // 510 bytes for command + 2 bytes for CRLF = 512 total
    if command.len() > 510 {
        return Err(Error::InvalidCommand(format!(
            "Command exceeds maximum length of 510 octets (got {})",
            command.len()
        )));
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
        let cmd = Command::Article(ArticleSpec::number_in_group("misc.test", 123));
        let encoded = cmd.encode().unwrap();
        // Note: group is for client-side context only, wire protocol only sends article number
        assert_eq!(encoded, b"ARTICLE 123\r\n");
    }

    #[test]
    fn test_article_spec_group_number() {
        let spec = ArticleSpec::GroupNumber {
            group: "alt.test".to_string(),
            article_number: 456,
        };
        let cmd = Command::Article(spec);
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"ARTICLE 456\r\n");
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
        let cmd = Command::List(ListVariant::OverviewFmt);
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"LIST OVERVIEW.FMT\r\n");
    }

    #[test]
    fn test_starttls_command() {
        let cmd = Command::StartTls;
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"STARTTLS\r\n");
    }

    #[test]
    fn test_mode_reader_command() {
        let cmd = Command::ModeReader;
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"MODE READER\r\n");
    }

    #[test]
    fn test_auth_info_user_command() {
        let cmd = Command::AuthInfoUser("testuser".to_string());
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"AUTHINFO USER testuser\r\n");
    }

    #[test]
    fn test_auth_info_pass_command() {
        let cmd = Command::AuthInfoPass("testpass".to_string());
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"AUTHINFO PASS testpass\r\n");
    }

    #[test]
    fn test_listgroup_no_range() {
        let cmd = Command::ListGroup(None);
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"LISTGROUP\r\n");
    }

    #[test]
    fn test_listgroup_with_range() {
        let cmd = Command::ListGroup(Some("misc.test 1-100".to_string()));
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"LISTGROUP misc.test 1-100\r\n");
    }

    #[test]
    fn test_head_command() {
        let cmd = Command::Head(ArticleSpec::MessageId("<test@example.com>".to_string()));
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"HEAD <test@example.com>\r\n");
    }

    #[test]
    fn test_body_command() {
        let cmd = Command::Body(ArticleSpec::MessageId("<test@example.com>".to_string()));
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"BODY <test@example.com>\r\n");
    }

    #[test]
    fn test_stat_command() {
        let cmd = Command::Stat(ArticleSpec::number_in_group("misc.test", 42));
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"STAT 42\r\n");
    }

    #[test]
    fn test_article_current() {
        let cmd = Command::Article(ArticleSpec::Current);
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"ARTICLE \r\n");
    }

    #[test]
    fn test_list_active_no_pattern() {
        let cmd = Command::List(ListVariant::Active(None));
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"LIST ACTIVE\r\n");
    }

    #[test]
    fn test_list_active_with_pattern() {
        let cmd = Command::List(ListVariant::Active(Some("comp.*".to_string())));
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"LIST ACTIVE comp.*\r\n");
    }

    #[test]
    fn test_list_newsgroups_no_pattern() {
        let cmd = Command::List(ListVariant::Newsgroups(None));
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"LIST NEWSGROUPS\r\n");
    }

    #[test]
    fn test_list_newsgroups_with_pattern() {
        let cmd = Command::List(ListVariant::Newsgroups(Some("alt.*".to_string())));
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"LIST NEWSGROUPS alt.*\r\n");
    }

    #[test]
    fn test_list_headers() {
        let cmd = Command::List(ListVariant::Headers);
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"LIST HEADERS\r\n");
    }

    #[test]
    fn test_list_active_times() {
        let cmd = Command::List(ListVariant::ActiveTimes);
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"LIST ACTIVE.TIMES\r\n");
    }

    #[test]
    fn test_list_distributions() {
        let cmd = Command::List(ListVariant::Distributions);
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"LIST DISTRIBUTIONS\r\n");
    }

    #[test]
    fn test_list_basic_no_pattern() {
        let cmd = Command::List(ListVariant::Basic(None));
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"LIST\r\n");
    }

    #[test]
    fn test_list_basic_with_pattern() {
        let cmd = Command::List(ListVariant::Basic(Some("misc.*".to_string())));
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"LIST misc.*\r\n");
    }

    #[test]
    fn test_newgroups_no_gmt() {
        let cmd = Command::NewGroups {
            date: "20240101".to_string(),
            time: "120000".to_string(),
            gmt: false,
            distributions: None,
        };
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"NEWGROUPS 20240101 120000\r\n");
    }

    #[test]
    fn test_newgroups_with_gmt() {
        let cmd = Command::NewGroups {
            date: "20240101".to_string(),
            time: "120000".to_string(),
            gmt: true,
            distributions: None,
        };
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"NEWGROUPS 20240101 120000 GMT\r\n");
    }

    #[test]
    fn test_newgroups_with_distributions() {
        let cmd = Command::NewGroups {
            date: "20240101".to_string(),
            time: "120000".to_string(),
            gmt: true,
            distributions: Some("local".to_string()),
        };
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"NEWGROUPS 20240101 120000 GMT local\r\n");
    }

    #[test]
    fn test_newnews_no_gmt() {
        let cmd = Command::NewNews {
            wildmat: "*".to_string(),
            date: "20240101".to_string(),
            time: "120000".to_string(),
            gmt: false,
        };
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"NEWNEWS * 20240101 120000\r\n");
    }

    #[test]
    fn test_newnews_with_gmt() {
        let cmd = Command::NewNews {
            wildmat: "comp.*".to_string(),
            date: "20240101".to_string(),
            time: "120000".to_string(),
            gmt: true,
        };
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"NEWNEWS comp.* 20240101 120000 GMT\r\n");
    }

    #[test]
    fn test_post_command() {
        let cmd = Command::Post;
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"POST\r\n");
    }

    #[test]
    fn test_quit_command() {
        let cmd = Command::Quit;
        let encoded = cmd.encode().unwrap();
        assert_eq!(encoded, b"QUIT\r\n");
    }

    #[test]
    fn test_invalid_message_id_no_brackets() {
        let cmd = Command::Article(ArticleSpec::MessageId("test@example.com".to_string()));
        let result = cmd.encode();
        assert!(result.is_err());
        if let Err(Error::InvalidCommand(msg)) = result {
            assert!(msg.contains("angle brackets"));
        } else {
            panic!("Expected InvalidCommand error");
        }
    }

    #[test]
    fn test_empty_parameter_error() {
        let cmd = Command::Group(String::new());
        let result = cmd.encode();
        assert!(result.is_err());
        if let Err(Error::InvalidCommand(msg)) = result {
            assert!(msg.contains("empty"));
        } else {
            panic!("Expected InvalidCommand error for empty parameter");
        }
    }

    #[test]
    fn test_command_too_long() {
        // Create a command that exceeds 510 bytes
        let long_param = "x".repeat(600);
        let cmd = Command::Group(long_param);
        let result = cmd.encode();
        assert!(result.is_err());
        if let Err(Error::InvalidCommand(msg)) = result {
            assert!(msg.contains("maximum length"));
        } else {
            panic!("Expected InvalidCommand error for long command");
        }
    }

    #[test]
    fn test_parameter_with_newline() {
        let cmd = Command::Group("misc.test\nQUIT".to_string());
        let result = cmd.encode();
        assert!(result.is_err());
    }

    #[test]
    fn test_list_active_with_invalid_pattern() {
        let cmd = Command::List(ListVariant::Active(Some("comp.*\r\nQUIT".to_string())));
        let result = cmd.encode();
        assert!(result.is_err());
    }

    #[test]
    fn test_hdr_with_invalid_field() {
        let cmd = Command::Hdr {
            field: "Subject\r\n".to_string(),
            range: None,
        };
        let result = cmd.encode();
        assert!(result.is_err());
    }

    #[test]
    fn test_hdr_with_invalid_range() {
        let cmd = Command::Hdr {
            field: "Subject".to_string(),
            range: Some("1-10\r\nQUIT".to_string()),
        };
        let result = cmd.encode();
        assert!(result.is_err());
    }

    #[test]
    fn test_over_with_invalid_range() {
        let cmd = Command::Over {
            range: Some("1-10\r\n".to_string()),
        };
        let result = cmd.encode();
        assert!(result.is_err());
    }

    #[test]
    fn test_auth_user_invalid() {
        let cmd = Command::AuthInfoUser("user\r\n".to_string());
        assert!(cmd.encode().is_err());
    }

    #[test]
    fn test_auth_pass_invalid() {
        let cmd = Command::AuthInfoPass("pass\r\n".to_string());
        assert!(cmd.encode().is_err());
    }

    #[test]
    fn test_listgroup_invalid_range() {
        let cmd = Command::ListGroup(Some("misc.test\r\n".to_string()));
        assert!(cmd.encode().is_err());
    }

    #[test]
    fn test_newgroups_invalid_date() {
        let cmd = Command::NewGroups {
            date: "2024\r\n0101".to_string(),
            time: "120000".to_string(),
            gmt: false,
            distributions: None,
        };
        assert!(cmd.encode().is_err());
    }

    #[test]
    fn test_newgroups_invalid_time() {
        let cmd = Command::NewGroups {
            date: "20240101".to_string(),
            time: "12\r\n0000".to_string(),
            gmt: false,
            distributions: None,
        };
        assert!(cmd.encode().is_err());
    }

    #[test]
    fn test_newgroups_invalid_distributions() {
        let cmd = Command::NewGroups {
            date: "20240101".to_string(),
            time: "120000".to_string(),
            gmt: false,
            distributions: Some("local\r\n".to_string()),
        };
        assert!(cmd.encode().is_err());
    }

    #[test]
    fn test_newnews_invalid_wildmat() {
        let cmd = Command::NewNews {
            wildmat: "*\r\n".to_string(),
            date: "20240101".to_string(),
            time: "120000".to_string(),
            gmt: false,
        };
        assert!(cmd.encode().is_err());
    }

    #[test]
    fn test_ihave_invalid_message_id_no_brackets() {
        let cmd = Command::Ihave {
            message_id: "article@example.com".to_string(),
        };
        let result = cmd.encode();
        assert!(result.is_err());
    }

    #[test]
    fn test_ihave_invalid_message_id_with_newline() {
        let cmd = Command::Ihave {
            message_id: "<article@example.com\r\n>".to_string(),
        };
        let result = cmd.encode();
        assert!(result.is_err());
    }

    #[test]
    fn test_list_newsgroups_invalid_pattern() {
        let cmd = Command::List(ListVariant::Newsgroups(Some("alt.*\r\n".to_string())));
        assert!(cmd.encode().is_err());
    }

    #[test]
    fn test_list_basic_invalid_pattern() {
        let cmd = Command::List(ListVariant::Basic(Some("misc.*\r\n".to_string())));
        assert!(cmd.encode().is_err());
    }
}
