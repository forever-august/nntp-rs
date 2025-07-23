//! Tokio async runtime integration for nntp-rs.

use crate::{Client, Command, Error, Response, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// NNTP client with Tokio integration.
///
/// This client provides a high-level async interface for NNTP operations
/// using Tokio for I/O operations.
pub struct NntpClient {
    client: Client,
    stream: TcpStream,
}

impl NntpClient {
    /// Connect to an NNTP server.
    ///
    /// # Arguments
    ///
    /// * `addr` - Server address in format "host:port"
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nntp_rs::tokio::NntpClient;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = NntpClient::connect("news.example.com:119").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect(addr: &str) -> Result<Self> {
        let stream = TcpStream::connect(addr)
            .await
            .map_err(|e| Error::Connection(format!("Failed to connect: {e}")))?;

        let mut client = Self {
            client: Client::new(),
            stream,
        };

        // Read initial server greeting
        let _greeting = client.read_response().await?;

        Ok(client)
    }

    /// Request server capabilities.
    pub async fn capabilities(&mut self) -> Result<Vec<String>> {
        let response = self.send_command(Command::Capabilities).await?;
        match response {
            Response::Capabilities(caps) => Ok(caps),
            Response::Error { code, message } => Err(Error::Protocol { code, message }),
            _ => Err(Error::InvalidResponse(
                "Expected capabilities response".to_string(),
            )),
        }
    }

    /// Switch to reader mode.
    pub async fn mode_reader(&mut self) -> Result<bool> {
        let response = self.send_command(Command::ModeReader).await?;
        match response {
            Response::ModeReader { posting_allowed } => Ok(posting_allowed),
            Response::Error { code, message } => Err(Error::Protocol { code, message }),
            _ => Err(Error::InvalidResponse(
                "Expected mode reader response".to_string(),
            )),
        }
    }

    /// Authenticate with username and password.
    pub async fn authenticate(&mut self, username: &str, password: &str) -> Result<()> {
        // Send username
        let response = self
            .send_command(Command::AuthInfoUser(username.to_string()))
            .await?;
        match response {
            Response::AuthSuccess => return Ok(()),
            Response::AuthRequired => {
                // Continue with password
            }
            Response::Error { code, message } => {
                return Err(Error::Protocol { code, message });
            }
            _ => {
                return Err(Error::InvalidResponse(
                    "Unexpected auth response".to_string(),
                ))
            }
        }

        // Send password
        let response = self
            .send_command(Command::AuthInfoPass(password.to_string()))
            .await?;
        match response {
            Response::AuthSuccess => Ok(()),
            Response::Error { code, message } => Err(Error::Protocol { code, message }),
            _ => Err(Error::InvalidResponse("Expected auth response".to_string())),
        }
    }

    /// Select a newsgroup.
    pub async fn group(&mut self, name: &str) -> Result<(u64, u64, u64)> {
        let response = self.send_command(Command::Group(name.to_string())).await?;
        match response {
            Response::GroupSelected {
                count, first, last, ..
            } => Ok((count, first, last)),
            Response::Error { code, message } => Err(Error::Protocol { code, message }),
            _ => Err(Error::InvalidResponse(
                "Expected group response".to_string(),
            )),
        }
    }

    /// Quit and close connection.
    pub async fn quit(mut self) -> Result<()> {
        let _response = self.send_command(Command::Quit).await?;
        self.stream
            .shutdown()
            .await
            .map_err(|e| Error::Io(format!("Failed to shutdown connection: {e}")))?;
        Ok(())
    }

    /// Send a command and wait for response.
    async fn send_command(&mut self, command: Command) -> Result<Response> {
        let request = self.client.encode_command(command)?;

        self.stream
            .write_all(&request)
            .await
            .map_err(|e| Error::Io(format!("Failed to send command: {e}")))?;

        self.read_response().await
    }

    /// Read a complete response from the server.
    async fn read_response(&mut self) -> Result<Response> {
        loop {
            // Try to decode a response from buffered data
            if let Some(response) = self.client.decode_response()? {
                return Ok(response);
            }

            // Read more data from the network
            let mut buffer = [0; 4096];
            let n = self
                .stream
                .read(&mut buffer)
                .await
                .map_err(|e| Error::Io(format!("Failed to read response: {e}")))?;

            if n == 0 {
                return Err(Error::Connection("Connection closed by server".to_string()));
            }

            self.client.feed_bytes(&buffer[..n]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        // Basic compilation test
        let _client = Client::new();
    }

    // Note: Integration tests would require a test NNTP server
    // These are better placed in separate integration test files
}
