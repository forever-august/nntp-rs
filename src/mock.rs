//! Mock NNTP server for testing purposes.
//!
//! This module provides a mock server implementation that can simulate
//! NNTP server responses for testing client functionality against the spec.

use crate::{Client, Command, Error, Response, Result};
use std::collections::VecDeque;

/// A mock NNTP server that simulates server responses for testing.
///
/// The mock server accepts a series of expected request/response pairs
/// and validates that the client sends the expected commands in order.
pub struct MockServer {
    expected_interactions: VecDeque<(Command, Response)>,
    strict_mode: bool,
}

impl MockServer {
    /// Create a new mock server with a series of expected interactions.
    ///
    /// Each interaction consists of an expected command and the response
    /// that should be sent back to the client.
    pub fn new(interactions: Vec<(Command, Response)>) -> Self {
        Self {
            expected_interactions: interactions.into(),
            strict_mode: true,
        }
    }

    /// Create a new mock server in non-strict mode.
    ///
    /// In non-strict mode, unexpected commands result in a generic error
    /// response rather than panicking.
    pub fn new_relaxed(interactions: Vec<(Command, Response)>) -> Self {
        Self {
            expected_interactions: interactions.into(),
            strict_mode: false,
        }
    }

    /// Process a command from the client and return the appropriate response.
    ///
    /// Returns an error if the command doesn't match the expected sequence
    /// (in strict mode) or if there are no more expected interactions.
    pub fn handle_command(&mut self, command: &Command) -> Result<Response> {
        if let Some((expected_cmd, response)) = self.expected_interactions.pop_front() {
            if *command == expected_cmd {
                Ok(response)
            } else if self.strict_mode {
                Err(Error::InvalidCommand(format!(
                    "Expected command {expected_cmd:?}, got {command:?}"
                )))
            } else {
                // Return the interaction back to the queue and send an error
                self.expected_interactions
                    .push_front((expected_cmd, response));
                Ok(Response::Error {
                    code: 500,
                    message: "Command not recognized".to_string(),
                })
            }
        } else if self.strict_mode {
            Err(Error::InvalidCommand(
                "No more expected commands".to_string(),
            ))
        } else {
            Ok(Response::Error {
                code: 500,
                message: "No handler for command".to_string(),
            })
        }
    }

    /// Check if all expected interactions have been processed.
    pub fn is_complete(&self) -> bool {
        self.expected_interactions.is_empty()
    }

    /// Get the number of remaining expected interactions.
    pub fn remaining_interactions(&self) -> usize {
        self.expected_interactions.len()
    }

    /// Reset the mock server with a new set of interactions.
    pub fn reset(&mut self, interactions: Vec<(Command, Response)>) {
        self.expected_interactions = interactions.into();
    }
}

/// Test helper that combines a Client and MockServer for integration testing.
///
/// This helper simulates the complete request/response cycle, encoding commands
/// from the client and feeding responses back.
pub struct ClientMockTest {
    client: Client,
    mock_server: MockServer,
}

impl ClientMockTest {
    /// Create a new test setup with the given interactions.
    pub fn new(interactions: Vec<(Command, Response)>) -> Self {
        Self {
            client: Client::new(),
            mock_server: MockServer::new(interactions),
        }
    }

    /// Create a new test setup in relaxed mode.
    pub fn new_relaxed(interactions: Vec<(Command, Response)>) -> Self {
        Self {
            client: Client::new(),
            mock_server: MockServer::new_relaxed(interactions),
        }
    }

    /// Send a command through the client and get the response from the mock server.
    ///
    /// This simulates the complete network round-trip by encoding the command,
    /// processing it through the mock server, and feeding the response back to the client.
    pub fn send_command(&mut self, command: Command) -> Result<Response> {
        // Encode command through client
        let _encoded_bytes = self.client.encode_command(command.clone())?;

        // Process command through mock server
        let response = self.mock_server.handle_command(&command)?;

        // Encode response and feed back to client
        let response_bytes = encode_response(&response)?;
        self.client.feed_bytes(&response_bytes);

        // Decode the response from client buffer
        if let Some(decoded_response) = self.client.decode_response()? {
            Ok(decoded_response)
        } else {
            Err(Error::Parse("Failed to decode response".to_string()))
        }
    }

    /// Get a reference to the client.
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Get a mutable reference to the client.
    pub fn client_mut(&mut self) -> &mut Client {
        &mut self.client
    }

    /// Check if all expected interactions have been processed.
    pub fn is_complete(&self) -> bool {
        self.mock_server.is_complete()
    }

    /// Get the number of remaining expected interactions.
    pub fn remaining_interactions(&self) -> usize {
        self.mock_server.remaining_interactions()
    }
}

/// Encode a response as it would come from a real NNTP server.
fn encode_response(response: &Response) -> Result<Vec<u8>> {
    let response_str = match response {
        Response::Capabilities(caps) => {
            let mut result = "101 Capability list:\r\n".to_string();
            for cap in caps {
                result.push_str(cap);
                result.push_str("\r\n");
            }
            result.push_str(".\r\n");
            result
        }
        Response::ModeReader { posting_allowed } => {
            if *posting_allowed {
                "200 Reader mode, posting allowed\r\n".to_string()
            } else {
                "201 Reader mode, posting prohibited\r\n".to_string()
            }
        }
        Response::AuthSuccess => "281 Authentication accepted\r\n".to_string(),
        Response::AuthRequired => "381 More authentication information required\r\n".to_string(),
        Response::GroupSelected {
            count,
            first,
            last,
            name,
        } => {
            format!("211 {count} {first} {last} {name}\r\n")
        }
        Response::ArticleListing(articles) => {
            let mut result = "211 Article list follows\r\n".to_string();
            for article in articles {
                result.push_str(&article.to_string());
                result.push_str("\r\n");
            }
            result.push_str(".\r\n");
            result
        }
        Response::Article {
            number,
            message_id,
            content,
        } => {
            let num_str = number.map_or("0".to_string(), |n| n.to_string());
            let mut result = format!("220 {num_str} {message_id} Article follows\r\n");
            result.push_str(&String::from_utf8_lossy(content));
            if !content.ends_with(b"\r\n") {
                result.push_str("\r\n");
            }
            result.push_str(".\r\n");
            result
        }
        Response::ArticleStatus { number, message_id } => {
            format!("223 {number} {message_id}\r\n")
        }
        Response::NewsgroupList(groups) => {
            let mut result = "215 Newsgroups follow:\r\n".to_string();
            for group in groups {
                result.push_str(&format!(
                    "{} {} {} {}\r\n",
                    group.name, group.last, group.first, group.posting_status
                ));
            }
            result.push_str(".\r\n");
            result
        }
        Response::NewNewsgroups(groups) => {
            let mut result = "231 New newsgroups follow:\r\n".to_string();
            for group in groups {
                result.push_str(&format!(
                    "{} {} {} {}\r\n",
                    group.name, group.last, group.first, group.posting_status
                ));
            }
            result.push_str(".\r\n");
            result
        }
        Response::NewArticles(articles) => {
            let mut result = "230 New articles follow:\r\n".to_string();
            for article in articles {
                result.push_str(article);
                result.push_str("\r\n");
            }
            result.push_str(".\r\n");
            result
        }
        Response::PostAccepted => "340 Send article to be posted\r\n".to_string(),
        Response::PostSuccess => "240 Article posted successfully\r\n".to_string(),
        Response::ArticleWanted => "335 Send article to be transferred\r\n".to_string(),
        Response::ArticleNotWanted => "435 Article not wanted\r\n".to_string(),
        Response::ArticleTransferred => "235 Article transferred successfully\r\n".to_string(),
        Response::Quit => "205 Goodbye\r\n".to_string(),
        Response::Help(help_lines) => {
            let mut result = "100 Help text follows\r\n".to_string();
            for line in help_lines {
                result.push_str(line);
                result.push_str("\r\n");
            }
            result.push_str(".\r\n");
            result
        }
        Response::Date(date) => {
            format!("111 {date}\r\n")
        }
        Response::HeaderData(headers) => {
            let mut result = "225 Header follows\r\n".to_string();
            for header in headers {
                result.push_str(&header.article);
                result.push(' ');
                result.push_str(&header.value);
                result.push_str("\r\n");
            }
            result.push_str(".\r\n");
            result
        }
        Response::OverviewData(overview) => {
            let mut result = "224 Overview information follows\r\n".to_string();
            for entry in overview {
                // Format as tab-separated fields
                result.push_str(&entry.fields.join("\t"));
                result.push_str("\r\n");
            }
            result.push_str(".\r\n");
            result
        }
        Response::OverviewFormat(format_fields) => {
            let mut result = "215 Order of fields in overview database\r\n".to_string();
            for field in format_fields {
                result.push_str(field);
                result.push_str("\r\n");
            }
            result.push_str(".\r\n");
            result
        }
        Response::Success { code, message } => {
            format!("{code} {message}\r\n")
        }
        // RFC 3977 specific error responses
        Response::ServiceDiscontinued { message } => {
            format!("400 {message}\r\n")
        }
        Response::NoSuchNewsgroup { message } => {
            format!("411 {message}\r\n")
        }
        Response::NoNewsgroupSelected { message } => {
            format!("412 {message}\r\n")
        }
        Response::NoCurrentArticle { message } => {
            format!("420 {message}\r\n")
        }
        Response::NoNextArticle { message } => {
            format!("421 {message}\r\n")
        }
        Response::NoPreviousArticle { message } => {
            format!("422 {message}\r\n")
        }
        Response::NoSuchArticle { message } => {
            format!("430 {message}\r\n")
        }
        Response::AuthenticationRequired { message } => {
            format!("480 {message}\r\n")
        }
        Response::CommandNotRecognized { message } => {
            format!("500 {message}\r\n")
        }
        Response::CommandSyntaxError { message } => {
            format!("501 {message}\r\n")
        }
        Response::AccessDenied { message } => {
            format!("502 {message}\r\n")
        }
        Response::ProgramFault { message } => {
            format!("503 {message}\r\n")
        }
        Response::Error { code, message } => {
            format!("{code} {message}\r\n")
        }
    };

    Ok(response_str.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Command, Response};

    #[test]
    fn test_mock_server_basic() {
        let interactions = vec![
            (
                Command::Capabilities,
                Response::Capabilities(vec!["VERSION 2".to_string(), "READER".to_string()]),
            ),
            (
                Command::ModeReader,
                Response::ModeReader {
                    posting_allowed: true,
                },
            ),
        ];

        let mut mock = MockServer::new(interactions);

        // Test expected sequence
        let response1 = mock.handle_command(&Command::Capabilities).unwrap();
        if let Response::Capabilities(caps) = response1 {
            assert_eq!(caps.len(), 2);
        } else {
            panic!("Expected Capabilities response");
        }

        let response2 = mock.handle_command(&Command::ModeReader).unwrap();
        if let Response::ModeReader { posting_allowed } = response2 {
            assert!(posting_allowed);
        } else {
            panic!("Expected ModeReader response");
        }

        assert!(mock.is_complete());
    }

    #[test]
    fn test_mock_server_wrong_command() {
        let interactions = vec![(
            Command::Capabilities,
            Response::Capabilities(vec!["VERSION 2".to_string()]),
        )];

        let mut mock = MockServer::new(interactions);

        // Send wrong command
        let result = mock.handle_command(&Command::ModeReader);
        assert!(result.is_err());
    }

    #[test]
    fn test_mock_server_relaxed_mode() {
        let interactions = vec![(
            Command::Capabilities,
            Response::Capabilities(vec!["VERSION 2".to_string()]),
        )];

        let mut mock = MockServer::new_relaxed(interactions);

        // Send wrong command - should get error response, not panic
        let response = mock.handle_command(&Command::ModeReader).unwrap();
        if let Response::Error { code, .. } = response {
            assert_eq!(code, 500);
        } else {
            panic!("Expected Error response");
        }

        // Original interaction should still be available
        assert_eq!(mock.remaining_interactions(), 1);
    }

    #[test]
    fn test_client_mock_integration() {
        let interactions = vec![
            (
                Command::Capabilities,
                Response::Capabilities(vec!["VERSION 2".to_string(), "READER".to_string()]),
            ),
            (
                Command::ModeReader,
                Response::ModeReader {
                    posting_allowed: true,
                },
            ),
            (
                Command::Group("comp.lang.rust".to_string()),
                Response::GroupSelected {
                    count: 1234,
                    first: 3000,
                    last: 4234,
                    name: "comp.lang.rust".to_string(),
                },
            ),
        ];

        let mut test = ClientMockTest::new(interactions);

        // Test capabilities
        let response = test.send_command(Command::Capabilities).unwrap();
        if let Response::Capabilities(caps) = response {
            assert_eq!(caps.len(), 2);
            assert_eq!(caps[0], "VERSION 2");
            assert_eq!(caps[1], "READER");
        } else {
            panic!("Expected Capabilities response");
        }

        // Test mode reader
        let response = test.send_command(Command::ModeReader).unwrap();
        if let Response::ModeReader { posting_allowed } = response {
            assert!(posting_allowed);
        } else {
            panic!("Expected ModeReader response");
        }

        assert_eq!(test.client().state(), "reader");

        // Test group selection
        let response = test
            .send_command(Command::Group("comp.lang.rust".to_string()))
            .unwrap();
        if let Response::GroupSelected { name, count, .. } = response {
            assert_eq!(name, "comp.lang.rust");
            assert_eq!(count, 1234);
        } else {
            panic!("Expected GroupSelected response");
        }

        assert_eq!(test.client().state(), "group_selected");
        assert_eq!(test.client().current_group(), Some("comp.lang.rust"));
        assert!(test.is_complete());
    }

    #[test]
    fn test_response_encoding() {
        // Test capability response encoding
        let caps_response =
            Response::Capabilities(vec!["VERSION 2".to_string(), "READER".to_string()]);
        let encoded = encode_response(&caps_response).unwrap();
        let expected = b"101 Capability list:\r\nVERSION 2\r\nREADER\r\n.\r\n";
        assert_eq!(encoded, expected);

        // Test group response encoding
        let group_response = Response::GroupSelected {
            count: 1234,
            first: 3000,
            last: 4234,
            name: "comp.lang.rust".to_string(),
        };
        let encoded = encode_response(&group_response).unwrap();
        let expected = b"211 1234 3000 4234 comp.lang.rust\r\n";
        assert_eq!(encoded, expected);

        // Test error response encoding
        let error_response = Response::Error {
            code: 500,
            message: "Command not recognized".to_string(),
        };
        let encoded = encode_response(&error_response).unwrap();
        let expected = b"500 Command not recognized\r\n";
        assert_eq!(encoded, expected);
    }
}
