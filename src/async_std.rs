//! async-std runtime integration for nntp-rs.

use crate::{Client, Command, Error, Response, Result};
use async_std::io::prelude::*;
use async_std::net::TcpStream;

/// NNTP client with async-std integration.
///
/// This client provides a high-level async interface for NNTP operations
/// using async-std for I/O operations.
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
    /// use nntp_rs::async_std::NntpClient;
    ///
    /// # #[async_std::main]
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
}
