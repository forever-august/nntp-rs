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

    /// List articles in current group with optional range.
    pub async fn listgroup(&mut self, range: Option<String>) -> Result<Vec<u64>> {
        let response = self.send_command(Command::ListGroup(range)).await?;
        match response {
            Response::ArticleListing(articles) => Ok(articles),
            Response::Error { code, message } => Err(Error::Protocol { code, message }),
            _ => Err(Error::InvalidResponse(
                "Expected article list response".to_string(),
            )),
        }
    }

    /// Retrieve full article by message-id or number.
    pub async fn article(&mut self, spec: crate::ArticleSpec) -> Result<crate::ParsedArticle> {
        let response = self.send_command(Command::Article(spec)).await?;
        match response {
            Response::Article {
                number,
                message_id,
                content,
            } => Ok(crate::ParsedArticle::new(number, message_id, content)),
            Response::Error { code, message } => Err(Error::Protocol { code, message }),
            _ => Err(Error::InvalidResponse(
                "Expected article response".to_string(),
            )),
        }
    }

    /// Retrieve article headers by message-id or number.
    pub async fn head(&mut self, spec: crate::ArticleSpec) -> Result<Vec<u8>> {
        let response = self.send_command(Command::Head(spec)).await?;
        match response {
            Response::Article { content, .. } => Ok(content),
            Response::Error { code, message } => Err(Error::Protocol { code, message }),
            _ => Err(Error::InvalidResponse(
                "Expected headers response".to_string(),
            )),
        }
    }

    /// Retrieve article body by message-id or number.
    pub async fn body(&mut self, spec: crate::ArticleSpec) -> Result<Vec<u8>> {
        let response = self.send_command(Command::Body(spec)).await?;
        match response {
            Response::Article { content, .. } => Ok(content),
            Response::Error { code, message } => Err(Error::Protocol { code, message }),
            _ => Err(Error::InvalidResponse("Expected body response".to_string())),
        }
    }

    /// Get article status by message-id or number.
    pub async fn stat(&mut self, spec: crate::ArticleSpec) -> Result<u64> {
        let response = self.send_command(Command::Stat(spec)).await?;
        match response {
            Response::ArticleStatus { number, .. } => Ok(number),
            Response::Error { code, message } => Err(Error::Protocol { code, message }),
            _ => Err(Error::InvalidResponse(
                "Expected status response".to_string(),
            )),
        }
    }

    /// List information with specific variants.
    pub async fn list(&mut self, variant: crate::ListVariant) -> Result<Vec<crate::NewsGroup>> {
        let response = self.send_command(Command::List(variant)).await?;
        match response {
            Response::NewsgroupList(list) => Ok(list),
            Response::OverviewFormat(_list) => {
                // Convert to dummy NewsGroup format for now
                Ok(vec![])
            }
            Response::Error { code, message } => Err(Error::Protocol { code, message }),
            _ => Err(Error::InvalidResponse("Expected list response".to_string())),
        }
    }

    /// List new newsgroups since date/time.
    pub async fn newgroups(
        &mut self,
        date: String,
        time: String,
        gmt: bool,
        distributions: Option<String>,
    ) -> Result<Vec<crate::NewsGroup>> {
        let response = self
            .send_command(Command::NewGroups {
                date,
                time,
                gmt,
                distributions,
            })
            .await?;
        match response {
            Response::NewNewsgroups(groups) => Ok(groups),
            Response::Error { code, message } => Err(Error::Protocol { code, message }),
            _ => Err(Error::InvalidResponse(
                "Expected newgroups response".to_string(),
            )),
        }
    }

    /// List new articles since date/time.
    pub async fn newnews(
        &mut self,
        wildmat: String,
        date: String,
        time: String,
        gmt: bool,
    ) -> Result<Vec<String>> {
        let response = self
            .send_command(Command::NewNews {
                wildmat,
                date,
                time,
                gmt,
            })
            .await?;
        match response {
            Response::NewArticles(articles) => Ok(articles),
            Response::Error { code, message } => Err(Error::Protocol { code, message }),
            _ => Err(Error::InvalidResponse(
                "Expected newnews response".to_string(),
            )),
        }
    }

    /// Post an article.
    pub async fn post(&mut self, article: String) -> Result<()> {
        let response = self.send_command(Command::Post).await?;
        match response {
            Response::PostAccepted => {
                // Send article content followed by a line with just a dot
                let mut content = article.into_bytes();
                content.extend_from_slice(b"\r\n.\r\n");
                self.stream
                    .write_all(&content)
                    .await
                    .map_err(|e| Error::Io(format!("Failed to send article: {e}")))?;

                // Read response
                let response = self.read_response().await?;
                match response {
                    Response::PostSuccess => Ok(()),
                    Response::Error { code, message } => Err(Error::Protocol { code, message }),
                    _ => Err(Error::InvalidResponse("Expected post response".to_string())),
                }
            }
            Response::Error { code, message } => Err(Error::Protocol { code, message }),
            _ => Err(Error::InvalidResponse(
                "Expected post ready response".to_string(),
            )),
        }
    }

    /// Request help information.
    pub async fn help(&mut self) -> Result<Vec<String>> {
        let response = self.send_command(Command::Help).await?;
        match response {
            Response::Help(help_text) => Ok(help_text),
            Response::Error { code, message } => Err(Error::Protocol { code, message }),
            _ => Err(Error::InvalidResponse("Expected help response".to_string())),
        }
    }

    /// Request server date and time.
    pub async fn date(&mut self) -> Result<String> {
        let response = self.send_command(Command::Date).await?;
        match response {
            Response::Date(date) => Ok(date),
            Response::Error { code, message } => Err(Error::Protocol { code, message }),
            _ => Err(Error::InvalidResponse("Expected date response".to_string())),
        }
    }

    /// Move to previous article in current group.
    pub async fn last(&mut self) -> Result<u64> {
        let response = self.send_command(Command::Last).await?;
        match response {
            Response::ArticleStatus { number, .. } => Ok(number),
            Response::Error { code, message } => Err(Error::Protocol { code, message }),
            _ => Err(Error::InvalidResponse(
                "Expected article status response".to_string(),
            )),
        }
    }

    /// Move to next article in current group.
    pub async fn next(&mut self) -> Result<u64> {
        let response = self.send_command(Command::Next).await?;
        match response {
            Response::ArticleStatus { number, .. } => Ok(number),
            Response::Error { code, message } => Err(Error::Protocol { code, message }),
            _ => Err(Error::InvalidResponse(
                "Expected article status response".to_string(),
            )),
        }
    }

    /// Retrieve specific header field for articles.
    pub async fn hdr(
        &mut self,
        field: String,
        range: Option<String>,
    ) -> Result<Vec<crate::HeaderEntry>> {
        let response = self.send_command(Command::Hdr { field, range }).await?;
        match response {
            Response::HeaderData(headers) => Ok(headers),
            Response::Error { code, message } => Err(Error::Protocol { code, message }),
            _ => Err(Error::InvalidResponse(
                "Expected headers response".to_string(),
            )),
        }
    }

    /// Retrieve overview information for articles.
    pub async fn over(&mut self, range: Option<String>) -> Result<Vec<crate::OverviewEntry>> {
        let response = self.send_command(Command::Over { range }).await?;
        match response {
            Response::OverviewData(overview) => Ok(overview),
            Response::Error { code, message } => Err(Error::Protocol { code, message }),
            _ => Err(Error::InvalidResponse(
                "Expected overview response".to_string(),
            )),
        }
    }

    /// Offer an article to the server.
    pub async fn ihave(&mut self, message_id: String, article: String) -> Result<()> {
        let response = self.send_command(Command::Ihave { message_id }).await?;
        match response {
            Response::ArticleWanted => {
                // Send article content followed by a line with just a dot
                let mut content = article.into_bytes();
                content.extend_from_slice(b"\r\n.\r\n");
                self.stream
                    .write_all(&content)
                    .await
                    .map_err(|e| Error::Io(format!("Failed to send article: {e}")))?;

                // Read response
                let response = self.read_response().await?;
                match response {
                    Response::ArticleTransferred => Ok(()),
                    Response::Error { code, message } => Err(Error::Protocol { code, message }),
                    _ => Err(Error::InvalidResponse(
                        "Expected transfer response".to_string(),
                    )),
                }
            }
            Response::ArticleNotWanted => Err(Error::Protocol {
                code: 435,
                message: "Article not wanted".to_string(),
            }),
            Response::Error { code, message } => Err(Error::Protocol { code, message }),
            _ => Err(Error::InvalidResponse(
                "Expected ihave response".to_string(),
            )),
        }
    }

    /// Quit and close connection.
    pub async fn quit(mut self) -> Result<()> {
        let _response = self.send_command(Command::Quit).await?;
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
}
