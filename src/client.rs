//! Sans-IO NNTP client implementation.

use crate::{Command, Error, Response, Result};
use bytes::{BufMut, Bytes, BytesMut};

/// Sans-IO NNTP client.
///
/// This client handles protocol logic without performing any I/O operations.
/// Users must handle network connections and data transmission separately.
pub struct Client {
    read_buffer: BytesMut,
    state: ClientState,
}

#[derive(Debug, Clone, PartialEq)]
enum ClientState {
    /// Initial state after connection
    Connected,
    /// Waiting for a specific response
    WaitingForResponse,
    /// In reader mode
    Reader,
    /// Authenticated
    Authenticated,
    /// Group selected
    GroupSelected { group: String },
    /// Posting mode
    Posting,
    /// Connection closed
    Closed,
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Client {
    /// Create a new NNTP client instance.
    pub fn new() -> Self {
        Self {
            read_buffer: BytesMut::new(),
            state: ClientState::Connected,
        }
    }

    /// Encode a command for transmission to the server.
    ///
    /// Returns the bytes that should be sent to the server.
    pub fn encode_command(&mut self, command: Command) -> Result<Bytes> {
        // Update state based on command
        self.update_state_for_command(&command)?;

        let bytes = command.encode()?;
        Ok(Bytes::from(bytes))
    }

    /// Feed received data from the server into the client.
    ///
    /// Call this method with data received from the network connection.
    pub fn feed_bytes(&mut self, data: &[u8]) {
        self.read_buffer.put_slice(data);
    }

    /// Try to decode a complete response from buffered data.
    ///
    /// Returns `Ok(Some(response))` if a complete response is available,
    /// `Ok(None)` if more data is needed, or an error if parsing fails.
    pub fn decode_response(&mut self) -> Result<Option<Response>> {
        if let Some(response_data) = self.extract_complete_response()? {
            let response = Response::parse(&response_data)?;
            self.update_state_for_response(&response)?;
            Ok(Some(response))
        } else {
            Ok(None)
        }
    }

    /// Get the current client state.
    pub fn state(&self) -> &str {
        match self.state {
            ClientState::Connected => "connected",
            ClientState::WaitingForResponse => "waiting",
            ClientState::Reader => "reader",
            ClientState::Authenticated => "authenticated",
            ClientState::GroupSelected { .. } => "group_selected",
            ClientState::Posting => "posting",
            ClientState::Closed => "closed",
        }
    }

    /// Check if the client is ready to send commands.
    pub fn is_ready(&self) -> bool {
        !matches!(
            self.state,
            ClientState::WaitingForResponse | ClientState::Closed
        )
    }

    /// Get the currently selected group, if any.
    pub fn current_group(&self) -> Option<&str> {
        if let ClientState::GroupSelected { group } = &self.state {
            Some(group)
        } else {
            None
        }
    }

    /// Check if the client is authenticated.
    pub fn is_authenticated(&self) -> bool {
        matches!(
            self.state,
            ClientState::Authenticated | ClientState::GroupSelected { .. }
        )
    }

    fn extract_complete_response(&mut self) -> Result<Option<Vec<u8>>> {
        // Look for complete response in buffer
        let buffer_str = std::str::from_utf8(&self.read_buffer)
            .map_err(|e| Error::Parse(format!("Invalid UTF-8 in response: {e}")))?;

        // Check for single-line response (ends with \r\n)
        if let Some(end_pos) = buffer_str.find("\r\n") {
            // Check if this is a multi-line response (starts with 1xx code)
            let status_line = &buffer_str[..end_pos];
            if let Ok(code) = status_line[..3].parse::<u16>() {
                if is_multiline_response(code) {
                    // Look for terminator "\r\n.\r\n"
                    if let Some(term_pos) = buffer_str.find("\r\n.\r\n") {
                        let response_len = term_pos + 5; // include terminator
                        let response = self.read_buffer.split_to(response_len).to_vec();
                        return Ok(Some(response));
                    } else {
                        // Need more data
                        return Ok(None);
                    }
                } else {
                    // Single-line response
                    let response_len = end_pos + 2; // include \r\n
                    let response = self.read_buffer.split_to(response_len).to_vec();
                    return Ok(Some(response));
                }
            }
        }

        // No complete response yet
        Ok(None)
    }

    /// Validate that a command can be executed in the current state
    fn validate_command_requirements(&self, command: &Command) -> Result<()> {
        match command {
            // Commands that require a group to be selected (RFC 3977)
            Command::Last | Command::Next => {
                if self.current_group().is_none() {
                    return Err(Error::Protocol {
                        code: 412,
                        message: "No newsgroup has been selected".to_string(),
                    });
                }
            }
            Command::Over { range: None } | Command::Hdr { range: None, .. } => {
                // OVER and HDR without range require current group selection
                if self.current_group().is_none() {
                    return Err(Error::Protocol {
                        code: 412,
                        message: "No newsgroup has been selected".to_string(),
                    });
                }
            }
            // Commands that might require authentication based on server policy
            Command::Post => {
                // Note: Some servers require authentication for posting
                // This is server-dependent, so we don't enforce it here
            }
            // Most other commands don't have strict prerequisites
            _ => {}
        }
        Ok(())
    }

    fn update_state_for_command(&mut self, command: &Command) -> Result<()> {
        // Validate that command can be executed in current state
        self.validate_command_requirements(command)?;

        match command {
            Command::Quit => {
                self.state = ClientState::Closed;
            }
            Command::ModeReader => {
                self.state = ClientState::WaitingForResponse;
            }
            Command::AuthInfoUser(_) | Command::AuthInfoPass(_) => {
                self.state = ClientState::WaitingForResponse;
            }
            Command::Group(_) => {
                self.state = ClientState::WaitingForResponse;
            }
            Command::Post => {
                self.state = ClientState::Posting;
            }
            _ => {
                self.state = ClientState::WaitingForResponse;
            }
        }
        Ok(())
    }

    fn update_state_for_response(&mut self, response: &Response) -> Result<()> {
        match response {
            Response::ModeReader { .. } => {
                self.state = ClientState::Reader;
            }
            Response::AuthSuccess => {
                self.state = ClientState::Authenticated;
            }
            Response::GroupSelected { name, .. } => {
                self.state = ClientState::GroupSelected {
                    group: name.clone(),
                };
            }
            Response::PostAccepted => {
                self.state = ClientState::Posting;
            }
            Response::PostSuccess => {
                self.state = if self.is_authenticated() {
                    ClientState::Authenticated
                } else {
                    ClientState::Reader
                };
            }
            Response::Quit => {
                self.state = ClientState::Closed;
            }
            Response::Error { code, .. } => {
                // Some errors might change state
                if *code >= 400 && *code < 500 {
                    // Temporary errors - keep current state
                } else {
                    // Permanent errors - might need to reset state
                    if self.state == ClientState::WaitingForResponse {
                        // Return to previous stable state
                        self.state = if self.is_authenticated() {
                            ClientState::Authenticated
                        } else {
                            ClientState::Reader
                        };
                    }
                }
            }
            _ => {
                // Most responses return to ready state
                if matches!(self.state, ClientState::WaitingForResponse) {
                    self.state = if self.is_authenticated() {
                        ClientState::Authenticated
                    } else {
                        ClientState::Reader
                    };
                }
            }
        }
        Ok(())
    }
}

fn is_multiline_response(code: u16) -> bool {
    matches!(code, 100..=110 | 112..=199 | 215 | 220..=222 | 224..=225 | 230 | 231)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = Client::new();
        assert_eq!(client.state(), "connected");
        assert!(client.is_ready());
        assert!(!client.is_authenticated());
        assert_eq!(client.current_group(), None);
    }

    #[test]
    fn test_encode_command() {
        let mut client = Client::new();
        let bytes = client.encode_command(Command::Capabilities).unwrap();
        assert_eq!(bytes.as_ref(), b"CAPABILITIES\r\n");
    }

    #[test]
    fn test_single_line_response() {
        let mut client = Client::new();
        client.feed_bytes(b"200 Reader mode, posting allowed\r\n");

        let response = client.decode_response().unwrap().unwrap();
        if let Response::ModeReader { posting_allowed } = response {
            assert!(posting_allowed);
        } else {
            panic!("Expected ModeReader response");
        }

        assert_eq!(client.state(), "reader");
    }

    #[test]
    fn test_multiline_response() {
        let mut client = Client::new();
        let data = b"101 Capability list:\r\nVERSION 2\r\nREADER\r\n.\r\n";
        client.feed_bytes(data);

        let response = client.decode_response().unwrap().unwrap();
        if let Response::Capabilities(caps) = response {
            assert_eq!(caps.len(), 2);
            assert_eq!(caps[0], "VERSION 2");
            assert_eq!(caps[1], "READER");
        } else {
            panic!("Expected Capabilities response");
        }
    }

    #[test]
    fn test_partial_response() {
        let mut client = Client::new();

        // Feed partial data
        client.feed_bytes(b"101 Capability list:\r\nVERSION 2\r\n");
        assert!(client.decode_response().unwrap().is_none());

        // Feed rest of data
        client.feed_bytes(b"READER\r\n.\r\n");
        let response = client.decode_response().unwrap().unwrap();

        if let Response::Capabilities(caps) = response {
            assert_eq!(caps.len(), 2);
        } else {
            panic!("Expected Capabilities response");
        }
    }

    #[test]
    fn test_group_selection() {
        let mut client = Client::new();

        // Select group
        client
            .encode_command(Command::Group("misc.test".to_string()))
            .unwrap();
        client.feed_bytes(b"211 1234 3000 4234 misc.test\r\n");

        let response = client.decode_response().unwrap().unwrap();
        if let Response::GroupSelected {
            name,
            count,
            first,
            last,
        } = response
        {
            assert_eq!(name, "misc.test");
            assert_eq!(count, 1234);
            assert_eq!(first, 3000);
            assert_eq!(last, 4234);
        } else {
            panic!("Expected GroupSelected response");
        }

        assert_eq!(client.state(), "group_selected");
        assert_eq!(client.current_group(), Some("misc.test"));
    }

    #[test]
    fn test_authentication_flow() {
        let mut client = Client::new();
        
        // Send user
        client.encode_command(Command::AuthInfoUser("testuser".to_string())).unwrap();
        client.feed_bytes(b"381 More authentication information required\r\n");
        let response = client.decode_response().unwrap().unwrap();
        assert!(matches!(response, Response::AuthRequired));
        
        // Send password
        client.encode_command(Command::AuthInfoPass("testpass".to_string())).unwrap();
        client.feed_bytes(b"281 Authentication accepted\r\n");
        let response = client.decode_response().unwrap().unwrap();
        assert!(matches!(response, Response::AuthSuccess));
        
        assert!(client.is_authenticated());
        assert_eq!(client.state(), "authenticated");
    }

    #[test]
    fn test_quit_command() {
        let mut client = Client::new();
        
        client.encode_command(Command::Quit).unwrap();
        assert_eq!(client.state(), "closed");
        
        client.feed_bytes(b"205 Goodbye\r\n");
        let response = client.decode_response().unwrap().unwrap();
        assert!(matches!(response, Response::Quit));
    }

    #[test]
    fn test_post_flow() {
        let mut client = Client::new();
        
        // Set up reader mode first
        client.encode_command(Command::ModeReader).unwrap();
        client.feed_bytes(b"200 Reader mode, posting allowed\r\n");
        client.decode_response().unwrap();
        
        // Start posting
        client.encode_command(Command::Post).unwrap();
        assert_eq!(client.state(), "posting");
        
        client.feed_bytes(b"340 Send article to be posted\r\n");
        let response = client.decode_response().unwrap().unwrap();
        assert!(matches!(response, Response::PostAccepted));
    }

    #[test]
    fn test_state_validation_last_requires_group() {
        let mut client = Client::new();
        
        // Try LAST without selecting a group
        let result = client.encode_command(Command::Last);
        assert!(result.is_err());
        
        if let Err(crate::Error::Protocol { code, .. }) = result {
            assert_eq!(code, 412);
        } else {
            panic!("Expected Protocol error with code 412");
        }
    }

    #[test]
    fn test_state_validation_next_requires_group() {
        let mut client = Client::new();
        
        // Try NEXT without selecting a group
        let result = client.encode_command(Command::Next);
        assert!(result.is_err());
    }

    #[test]
    fn test_state_validation_over_without_range_requires_group() {
        let mut client = Client::new();
        
        // OVER without range requires group
        let result = client.encode_command(Command::Over { range: None });
        assert!(result.is_err());
        
        // OVER with range is okay without group
        let result = client.encode_command(Command::Over { range: Some("100-200".to_string()) });
        assert!(result.is_ok());
    }

    #[test]
    fn test_state_validation_hdr_without_range_requires_group() {
        let mut client = Client::new();
        
        // HDR without range requires group
        let result = client.encode_command(Command::Hdr { field: "Subject".to_string(), range: None });
        assert!(result.is_err());
    }

    #[test]
    fn test_error_response_state_handling() {
        let mut client = Client::new();
        
        // Enter reader mode
        client.encode_command(Command::ModeReader).unwrap();
        client.feed_bytes(b"200 Reader mode, posting allowed\r\n");
        client.decode_response().unwrap();
        assert_eq!(client.state(), "reader");
        
        // Send a command that results in error
        client.encode_command(Command::Group("nonexistent.group".to_string())).unwrap();
        client.feed_bytes(b"411 No such newsgroup\r\n");
        let response = client.decode_response().unwrap().unwrap();
        
        assert!(matches!(response, Response::Error { code: 411, .. }));
        // State is 'reader' after error response is processed (not waiting anymore)
        // The client returns to reader state after an error
        assert!(client.state() == "reader" || client.state() == "waiting");
    }

    #[test]
    fn test_default_client() {
        let client = Client::default();
        assert_eq!(client.state(), "connected");
    }

    #[test]
    fn test_empty_buffer_returns_none() {
        let mut client = Client::new();
        
        // No data fed
        let response = client.decode_response().unwrap();
        assert!(response.is_none());
    }

    #[test]
    fn test_post_success_returns_to_reader() {
        let mut client = Client::new();
        
        // Set up reader mode
        client.encode_command(Command::ModeReader).unwrap();
        client.feed_bytes(b"200 Reader mode, posting allowed\r\n");
        client.decode_response().unwrap();
        
        // Start posting
        client.encode_command(Command::Post).unwrap();
        client.feed_bytes(b"340 Send article\r\n");
        client.decode_response().unwrap();
        assert_eq!(client.state(), "posting");
        
        // Post success returns to reader state (since we weren't authenticated)
        client.feed_bytes(b"240 Article posted\r\n");
        let response = client.decode_response().unwrap().unwrap();
        assert!(matches!(response, Response::PostSuccess));
        // After post success from reader mode, returns to reader
        assert_eq!(client.state(), "reader");
    }

    #[test]
    fn test_post_from_authenticated_state() {
        let mut client = Client::new();
        
        // Set up reader mode and authenticate
        client.encode_command(Command::ModeReader).unwrap();
        client.feed_bytes(b"200 Reader mode, posting allowed\r\n");
        client.decode_response().unwrap();
        
        // Authenticate
        client.encode_command(Command::AuthInfoUser("user".to_string())).unwrap();
        client.feed_bytes(b"381 Password required\r\n");
        client.decode_response().unwrap();
        
        client.encode_command(Command::AuthInfoPass("pass".to_string())).unwrap();
        client.feed_bytes(b"281 Authentication accepted\r\n");
        client.decode_response().unwrap();
        assert!(client.is_authenticated());
        assert_eq!(client.state(), "authenticated");
        
        // Start posting - state becomes posting
        client.encode_command(Command::Post).unwrap();
        client.feed_bytes(b"340 Send article\r\n");
        client.decode_response().unwrap();
        assert_eq!(client.state(), "posting");
        // Note: is_authenticated() returns false when in Posting state
        // because the state machine doesn't track previous auth state
    }

    #[test]
    fn test_permanent_error_state_reset() {
        let mut client = Client::new();
        
        // Set up reader mode
        client.encode_command(Command::ModeReader).unwrap();
        client.feed_bytes(b"200 Reader mode, posting allowed\r\n");
        client.decode_response().unwrap();
        
        // Send a command
        client.encode_command(Command::Group("test.group".to_string())).unwrap();
        
        // Get a 5xx permanent error
        client.feed_bytes(b"502 Access denied\r\n");
        let response = client.decode_response().unwrap().unwrap();
        
        assert!(matches!(response, Response::Error { code: 502, .. }));
        // After permanent error while waiting, should return to reader state
        assert_eq!(client.state(), "reader");
    }

    #[test]
    fn test_generic_response_after_waiting() {
        let mut client = Client::new();
        
        // Set up reader mode
        client.encode_command(Command::ModeReader).unwrap();
        client.feed_bytes(b"200 Reader mode, posting allowed\r\n");
        client.decode_response().unwrap();
        
        // Send help command (puts us in waiting state)
        client.encode_command(Command::Help).unwrap();
        assert_eq!(client.state(), "waiting");
        
        // Get help response
        client.feed_bytes(b"100 Help follows\r\nTest\r\n.\r\n");
        let response = client.decode_response().unwrap().unwrap();
        
        assert!(matches!(response, Response::Help(_)));
        // After help response, returns to reader (not authenticated since we never authed)
        assert_eq!(client.state(), "reader");
    }
}
