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

    fn update_state_for_command(&mut self, command: &Command) -> Result<()> {
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
    matches!(code, 100..=199 | 215 | 220..=222 | 230 | 231)
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
}
