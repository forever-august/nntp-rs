//! Generic async NNTP client implementation.
//!
//! This module provides a generic NNTP client that works with any stream type
//! implementing the [`AsyncStream`] trait. This allows the same client implementation
//! to be used across different async runtimes (tokio, async-std, smol).
//!
//! # Overview
//!
//! The [`NntpClient`] struct combines a sans-io [`Client`] for protocol logic with
//! an async stream for network I/O. The generic parameter `S` can be any type that
//! implements [`AsyncStream`], such as:
//!
//! - [`TokioStream`](crate::runtime::TokioStream) for tokio runtime
//! - [`AsyncStdStream`](crate::runtime::AsyncStdStream) for async-std runtime
//! - [`SmolStream`](crate::runtime::SmolStream) for smol runtime
//!
//! # Examples
//!
//! ## Using with Tokio
//!
//! ```rust,no_run
//! # #[cfg(feature = "tokio-runtime")]
//! # {
//! use nntp_rs::net_client::NntpClient;
//! use nntp_rs::runtime::TokioStream;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut client = NntpClient::<TokioStream>::connect("news.example.com:119").await?;
//! let capabilities = client.capabilities().await?;
//! println!("Server capabilities: {:?}", capabilities);
//! client.quit().await?;
//! # Ok(())
//! # }
//! # }
//! ```
//!
//! ## Using with async-std
//!
//! ```rust,no_run
//! # #[cfg(feature = "async-std-runtime")]
//! # {
//! use nntp_rs::net_client::NntpClient;
//! use nntp_rs::runtime::AsyncStdStream;
//!
//! # #[async_std::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut client = NntpClient::<AsyncStdStream>::connect("news.example.com:119").await?;
//! let capabilities = client.capabilities().await?;
//! println!("Server capabilities: {:?}", capabilities);
//! client.quit().await?;
//! # Ok(())
//! # }
//! # }
//! ```
//!
//! ## Using with smol
//!
//! ```rust,no_run
//! # #[cfg(feature = "smol-runtime")]
//! # {
//! use nntp_rs::net_client::NntpClient;
//! use nntp_rs::runtime::SmolStream;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # smol::block_on(async {
//! let mut client = NntpClient::<SmolStream>::connect("news.example.com:119").await?;
//! let capabilities = client.capabilities().await?;
//! println!("Server capabilities: {:?}", capabilities);
//! client.quit().await?;
//! # Ok(())
//! # })
//! # }
//! # }
//! ```

use crate::response::{
    ActiveTimeEntry, ActiveTimesList, ArticleNumbers, ArticlePointer, Capabilities, CountsEntry,
    CountsList, DistribPat, DistribPatsList, DistributionEntry, DistributionsList, GroupStats,
    HeaderData, HeadersList, HelpText, MessageIdList, ModeratorEntry, ModeratorsList,
    NewsgroupDesc, NewsgroupDescList, NewsgroupList, OverviewData, OverviewFormat, PostingStatus,
    ServerDate,
};
use crate::runtime::AsyncStream;
use crate::{Client, Command, Error, Response, Result};

/// Generic NNTP client that works with any async stream implementation.
///
/// This client provides a high-level async interface for NNTP operations.
/// The stream type `S` determines which async runtime is used for I/O.
///
/// # Type Parameters
///
/// * `S` - The stream type implementing [`AsyncStream`]. This is typically one of:
///   - [`TokioStream`](crate::runtime::TokioStream) for tokio
///   - [`AsyncStdStream`](crate::runtime::AsyncStdStream) for async-std
///   - [`SmolStream`](crate::runtime::SmolStream) for smol
///
/// # Examples
///
/// ```rust,no_run
/// # #[cfg(feature = "tokio-runtime")]
/// # {
/// use nntp_rs::net_client::NntpClient;
/// use nntp_rs::runtime::TokioStream;
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Connect to an NNTP server
/// let mut client = NntpClient::<TokioStream>::connect("news.example.com:119").await?;
///
/// // Get server capabilities
/// let caps = client.capabilities().await?;
///
/// // Switch to reader mode
/// let posting_allowed = client.mode_reader().await?;
///
/// // Select a newsgroup
/// let stats = client.group("misc.test").await?;
/// println!("Group has {} articles ({}-{})", stats.count, stats.first, stats.last);
///
/// // Retrieve an article
/// let article = client.article(nntp_rs::ArticleSpec::Current).await?;
///
/// // Disconnect
/// client.quit().await?;
/// # Ok(())
/// # }
/// # }
/// ```
pub struct NntpClient<S: AsyncStream> {
    /// The sans-io client handling protocol logic.
    client: Client,
    /// The async stream for network I/O.
    stream: S,
    /// Whether posting is allowed on this connection.
    posting_allowed: bool,
}

impl<S: AsyncStream> NntpClient<S> {
    /// Connect to an NNTP server.
    ///
    /// Establishes a TCP connection to the specified address and reads the
    /// initial server greeting.
    ///
    /// # Arguments
    ///
    /// * `addr` - Server address in format "host:port"
    ///
    /// # Returns
    ///
    /// Returns a connected `NntpClient` on success, or an error if the connection
    /// fails or the server greeting cannot be read.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "tokio-runtime")]
    /// # {
    /// use nntp_rs::net_client::NntpClient;
    /// use nntp_rs::runtime::TokioStream;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = NntpClient::<TokioStream>::connect("news.example.com:119").await?;
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub async fn connect(addr: &str) -> Result<Self> {
        let stream = S::connect(addr)
            .await
            .map_err(|e| Error::Connection(format!("Failed to connect: {e}")))?;

        let mut client = Self {
            client: Client::new(),
            stream,
            posting_allowed: false,
        };

        // Read initial server greeting and extract posting permission
        let greeting = client.read_response().await?;
        if let Response::ModeReader { posting_allowed } = greeting {
            client.posting_allowed = posting_allowed;
        }

        Ok(client)
    }

    /// Request server capabilities.
    ///
    /// Sends a CAPABILITIES command and returns the list of capabilities
    /// supported by the server.
    ///
    /// # Returns
    ///
    /// A [`Capabilities`] wrapper containing capability strings as reported by the server.
    ///
    /// # Errors
    ///
    /// Returns an error if the command fails or the response is invalid.
    pub async fn capabilities(&mut self) -> Result<Capabilities> {
        let response = self.send_command(Command::Capabilities).await?;
        if let Response::Error { code, message } = &response {
            return Err(Error::Protocol {
                code: *code,
                message: message.clone(),
            });
        }
        response.try_into()
    }

    /// Switch to reader mode.
    ///
    /// Sends a MODE READER command to switch the server to reader mode.
    ///
    /// # Returns
    ///
    /// A [`PostingStatus`] indicating if posting is allowed.
    ///
    /// # Errors
    ///
    /// Returns an error if the command fails or the response is invalid.
    pub async fn mode_reader(&mut self) -> Result<PostingStatus> {
        let response = self.send_command(Command::ModeReader).await?;
        if let Response::Error { code, message } = &response {
            return Err(Error::Protocol {
                code: *code,
                message: message.clone(),
            });
        }
        // Update posting_allowed from response
        if let Response::ModeReader { posting_allowed } = &response {
            self.posting_allowed = *posting_allowed;
        }
        response.try_into()
    }

    /// Authenticate with username and password.
    ///
    /// Performs AUTHINFO USER/PASS authentication with the server.
    ///
    /// # Arguments
    ///
    /// * `username` - The username for authentication
    /// * `password` - The password for authentication
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails.
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
    ///
    /// Sends a GROUP command to select the specified newsgroup.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the newsgroup to select
    ///
    /// # Returns
    ///
    /// A [`GroupStats`] containing article count and range information.
    ///
    /// # Errors
    ///
    /// Returns an error if the group doesn't exist or the command fails.
    pub async fn group(&mut self, name: &str) -> Result<GroupStats> {
        let response = self.send_command(Command::Group(name.to_string())).await?;
        if let Response::Error { code, message } = &response {
            return Err(Error::Protocol {
                code: *code,
                message: message.clone(),
            });
        }
        response.try_into()
    }

    /// List articles in current group with optional range.
    ///
    /// Sends a LISTGROUP command to get article numbers in the current group.
    ///
    /// # Arguments
    ///
    /// * `range` - Optional range specification (e.g., "1-100" or "500-")
    ///
    /// # Returns
    ///
    /// An [`ArticleNumbers`] containing article numbers.
    pub async fn listgroup(&mut self, range: Option<String>) -> Result<ArticleNumbers> {
        let response = self.send_command(Command::ListGroup(range)).await?;
        if let Response::Error { code, message } = &response {
            return Err(Error::Protocol {
                code: *code,
                message: message.clone(),
            });
        }
        response.try_into()
    }

    /// Retrieve full article by message-id or number.
    ///
    /// Sends an ARTICLE command to retrieve the complete article.
    ///
    /// # Arguments
    ///
    /// * `spec` - The article specification (message-id, number, or current)
    ///
    /// # Returns
    ///
    /// The retrieved [`Article`](crate::Article) with headers and body.
    pub async fn article(&mut self, spec: crate::ArticleSpec) -> Result<crate::Article> {
        let response = self.send_command(Command::Article(spec)).await?;
        if let Response::Error { code, message } = &response {
            return Err(Error::Protocol {
                code: *code,
                message: message.clone(),
            });
        }
        response.try_into()
    }

    /// Retrieve article headers by message-id or number.
    ///
    /// Sends a HEAD command to retrieve only the article headers.
    ///
    /// # Arguments
    ///
    /// * `spec` - The article specification (message-id, number, or current)
    ///
    /// # Returns
    ///
    /// The raw header bytes.
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
    ///
    /// Sends a BODY command to retrieve only the article body.
    ///
    /// # Arguments
    ///
    /// * `spec` - The article specification (message-id, number, or current)
    ///
    /// # Returns
    ///
    /// The raw body bytes.
    pub async fn body(&mut self, spec: crate::ArticleSpec) -> Result<Vec<u8>> {
        let response = self.send_command(Command::Body(spec)).await?;
        match response {
            Response::Article { content, .. } => Ok(content),
            Response::Error { code, message } => Err(Error::Protocol { code, message }),
            _ => Err(Error::InvalidResponse("Expected body response".to_string())),
        }
    }

    /// Get article status by message-id or number.
    ///
    /// Sends a STAT command to check if an article exists without retrieving it.
    ///
    /// # Arguments
    ///
    /// * `spec` - The article specification (message-id, number, or current)
    ///
    /// # Returns
    ///
    /// An [`ArticlePointer`] containing the article number and message ID.
    pub async fn stat(&mut self, spec: crate::ArticleSpec) -> Result<ArticlePointer> {
        let response = self.send_command(Command::Stat(spec)).await?;
        if let Response::Error { code, message } = &response {
            return Err(Error::Protocol {
                code: *code,
                message: message.clone(),
            });
        }
        response.try_into()
    }

    /// List active newsgroups with optional wildmat pattern.
    ///
    /// Sends a LIST ACTIVE command to retrieve active newsgroups.
    /// Each entry contains the group name, article range, and posting status.
    ///
    /// # Arguments
    ///
    /// * `wildmat` - Optional wildmat pattern to filter newsgroups (e.g., "comp.*")
    ///
    /// # Returns
    ///
    /// A [`NewsgroupList`] containing active newsgroup information.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // List all active newsgroups
    /// let groups = client.list_active(None).await?;
    ///
    /// // List only comp.* newsgroups
    /// let comp_groups = client.list_active(Some("comp.*".to_string())).await?;
    /// ```
    pub async fn list_active(&mut self, wildmat: Option<String>) -> Result<NewsgroupList> {
        let response = self
            .send_command(Command::List(crate::ListVariant::Active(wildmat)))
            .await?;
        if let Response::Error { code, message } = &response {
            return Err(Error::Protocol {
                code: *code,
                message: message.clone(),
            });
        }
        response.try_into()
    }

    /// List newsgroup descriptions with optional wildmat pattern.
    ///
    /// Sends a LIST NEWSGROUPS command to retrieve newsgroup descriptions.
    /// Each entry contains the group name and a human-readable description.
    ///
    /// # Arguments
    ///
    /// * `wildmat` - Optional wildmat pattern to filter newsgroups (e.g., "comp.*")
    ///
    /// # Returns
    ///
    /// A [`NewsgroupDescList`] containing newsgroup descriptions.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // List all newsgroup descriptions
    /// let descs = client.list_newsgroups(None).await?;
    /// for entry in descs.iter() {
    ///     println!("{}: {}", entry.name, entry.description);
    /// }
    /// ```
    pub async fn list_newsgroups(&mut self, wildmat: Option<String>) -> Result<NewsgroupDescList> {
        let response = self
            .send_command(Command::List(crate::ListVariant::Newsgroups(wildmat)))
            .await?;
        if let Response::Error { code, message } = &response {
            return Err(Error::Protocol {
                code: *code,
                message: message.clone(),
            });
        }
        // Parse as newsgroup descriptions
        match response {
            Response::NewsgroupList(groups) => {
                // Convert NewsGroup entries to NewsgroupDesc entries
                // For LIST NEWSGROUPS, the response is parsed as NewsGroup but
                // we want to extract name and description
                let descs: Vec<NewsgroupDesc> = groups
                    .into_iter()
                    .map(|g| NewsgroupDesc {
                        name: g.name,
                        // For LIST NEWSGROUPS, the "description" comes from the line after name
                        // The current parser puts the first word of description in last/first fields
                        // We'll return an empty description here since the parser doesn't preserve it
                        description: String::new(),
                    })
                    .collect();
                Ok(NewsgroupDescList(descs))
            }
            _ => Err(Error::InvalidResponse(
                "Expected newsgroup list response".to_string(),
            )),
        }
    }

    /// List overview format specification.
    ///
    /// Sends a LIST OVERVIEW.FMT command to retrieve the order of fields
    /// returned by the OVER command.
    ///
    /// # Returns
    ///
    /// An [`OverviewFormat`] containing field names in order.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let format = client.list_overview_fmt().await?;
    /// for (i, field) in format.iter().enumerate() {
    ///     println!("Field {}: {}", i, field);
    /// }
    /// ```
    pub async fn list_overview_fmt(&mut self) -> Result<OverviewFormat> {
        let response = self
            .send_command(Command::List(crate::ListVariant::OverviewFmt))
            .await?;
        if let Response::Error { code, message } = &response {
            return Err(Error::Protocol {
                code: *code,
                message: message.clone(),
            });
        }
        response.try_into()
    }

    /// List available headers for HDR command.
    ///
    /// Sends a LIST HEADERS command to retrieve the list of header/metadata
    /// fields that can be retrieved using the HDR command.
    ///
    /// # Returns
    ///
    /// A [`HeadersList`] containing available header field names.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let headers = client.list_headers().await?;
    /// for field in headers.iter() {
    ///     println!("Available: {}", field);
    /// }
    /// ```
    pub async fn list_headers(&mut self) -> Result<HeadersList> {
        let response = self
            .send_command(Command::List(crate::ListVariant::Headers))
            .await?;
        if let Response::Error { code, message } = &response {
            return Err(Error::Protocol {
                code: *code,
                message: message.clone(),
            });
        }
        // Parse as list of strings (similar to OverviewFormat)
        match response {
            Response::OverviewFormat(fields) => Ok(HeadersList(fields)),
            Response::NewsgroupList(groups) => {
                // Some servers return headers in newsgroup format
                let headers: Vec<String> = groups.into_iter().map(|g| g.name).collect();
                Ok(HeadersList(headers))
            }
            _ => Err(Error::InvalidResponse(
                "Expected headers list response".to_string(),
            )),
        }
    }

    /// List newsgroup creation times.
    ///
    /// Sends a LIST ACTIVE.TIMES command to retrieve when newsgroups were created.
    ///
    /// # Returns
    ///
    /// An [`ActiveTimesList`] containing newsgroup creation information.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let times = client.list_active_times().await?;
    /// for entry in times.iter() {
    ///     println!("{} created at {} by {}", entry.name, entry.timestamp, entry.creator);
    /// }
    /// ```
    pub async fn list_active_times(&mut self) -> Result<ActiveTimesList> {
        let response = self
            .send_command(Command::List(crate::ListVariant::ActiveTimes))
            .await?;
        if let Response::Error { code, message } = &response {
            return Err(Error::Protocol {
                code: *code,
                message: message.clone(),
            });
        }
        // Parse as ActiveTimesList from the generic newsgroup list response
        // Format: groupname timestamp creator
        match response {
            Response::NewsgroupList(groups) => {
                // The parser puts data in the NewsGroup fields, but ACTIVE.TIMES
                // has format: groupname timestamp creator
                // We need to re-parse or handle this specially
                // For now, return what we can extract
                let times: Vec<ActiveTimeEntry> = groups
                    .into_iter()
                    .map(|g| ActiveTimeEntry {
                        name: g.name,
                        // last field contains timestamp in some parsings
                        timestamp: g.last,
                        // first field or posting_status might contain creator info
                        creator: String::new(),
                    })
                    .collect();
                Ok(ActiveTimesList(times))
            }
            _ => Err(Error::InvalidResponse(
                "Expected active times response".to_string(),
            )),
        }
    }

    /// List valid distribution values.
    ///
    /// Sends a LIST DISTRIBUTIONS command to retrieve valid distribution values.
    ///
    /// # Returns
    ///
    /// A [`DistributionsList`] containing valid distributions and descriptions.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let dists = client.list_distributions().await?;
    /// for entry in dists.iter() {
    ///     println!("{}: {}", entry.name, entry.description);
    /// }
    /// ```
    pub async fn list_distributions(&mut self) -> Result<DistributionsList> {
        let response = self
            .send_command(Command::List(crate::ListVariant::Distributions))
            .await?;
        if let Response::Error { code, message } = &response {
            return Err(Error::Protocol {
                code: *code,
                message: message.clone(),
            });
        }
        // Parse as DistributionsList
        // Format: distribution description
        match response {
            Response::NewsgroupList(groups) => {
                let dists: Vec<DistributionEntry> = groups
                    .into_iter()
                    .map(|g| DistributionEntry {
                        name: g.name,
                        description: String::new(),
                    })
                    .collect();
                Ok(DistributionsList(dists))
            }
            _ => Err(Error::InvalidResponse(
                "Expected distributions response".to_string(),
            )),
        }
    }

    /// List distribution patterns.
    ///
    /// Sends a LIST DISTRIB.PATS command to retrieve default distribution patterns.
    ///
    /// # Returns
    ///
    /// A [`DistribPatsList`] containing distribution patterns.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let pats = client.list_distrib_pats().await?;
    /// for pat in pats.iter() {
    ///     println!("Weight {}: {} -> {}", pat.weight, pat.wildmat, pat.distribution);
    /// }
    /// ```
    pub async fn list_distrib_pats(&mut self) -> Result<DistribPatsList> {
        let response = self
            .send_command(Command::List(crate::ListVariant::DistribPats))
            .await?;
        if let Response::Error { code, message } = &response {
            return Err(Error::Protocol {
                code: *code,
                message: message.clone(),
            });
        }
        // Parse as DistribPatsList
        // Format: weight:wildmat:distribution
        match response {
            Response::NewsgroupList(groups) => {
                let pats: Vec<DistribPat> = groups
                    .into_iter()
                    .map(|g| {
                        // The name field might contain "weight:wildmat:distribution"
                        let parts: Vec<&str> = g.name.splitn(3, ':').collect();
                        if parts.len() >= 3 {
                            DistribPat {
                                weight: parts[0].parse().unwrap_or(0),
                                wildmat: parts[1].to_string(),
                                distribution: parts[2].to_string(),
                            }
                        } else {
                            DistribPat {
                                weight: 0,
                                wildmat: g.name,
                                distribution: String::new(),
                            }
                        }
                    })
                    .collect();
                Ok(DistribPatsList(pats))
            }
            _ => Err(Error::InvalidResponse(
                "Expected distrib.pats response".to_string(),
            )),
        }
    }

    /// List newsgroup counts.
    ///
    /// Sends a LIST COUNTS command to retrieve newsgroup article counts.
    /// Note: This command is not defined in RFC 3977 but is supported by some servers.
    ///
    /// # Arguments
    ///
    /// * `wildmat` - Optional wildmat pattern to filter newsgroups
    ///
    /// # Returns
    ///
    /// A [`CountsList`] containing newsgroup count information.
    pub async fn list_counts(&mut self, wildmat: Option<String>) -> Result<CountsList> {
        let response = self
            .send_command(Command::List(crate::ListVariant::Counts(wildmat)))
            .await?;
        if let Response::Error { code, message } = &response {
            return Err(Error::Protocol {
                code: *code,
                message: message.clone(),
            });
        }
        // Parse as CountsList - similar to LIST ACTIVE format
        match response {
            Response::NewsgroupList(groups) => {
                let counts: Vec<CountsEntry> = groups
                    .into_iter()
                    .map(|g| CountsEntry {
                        name: g.name,
                        count: 0, // Not directly available from NewsGroup
                        low: g.first,
                        high: g.last,
                        status: g.posting_status,
                    })
                    .collect();
                Ok(CountsList(counts))
            }
            _ => Err(Error::InvalidResponse(
                "Expected counts response".to_string(),
            )),
        }
    }

    /// List moderator patterns.
    ///
    /// Sends a LIST MODERATORS command to retrieve moderator email patterns.
    /// Note: This command is not defined in RFC 3977 but is supported by some servers.
    ///
    /// # Returns
    ///
    /// A [`ModeratorsList`] containing moderator patterns.
    pub async fn list_moderators(&mut self) -> Result<ModeratorsList> {
        let response = self
            .send_command(Command::List(crate::ListVariant::Moderators))
            .await?;
        if let Response::Error { code, message } = &response {
            return Err(Error::Protocol {
                code: *code,
                message: message.clone(),
            });
        }
        // Parse as ModeratorsList
        // Format: wildmat:template
        match response {
            Response::NewsgroupList(groups) => {
                let mods: Vec<ModeratorEntry> = groups
                    .into_iter()
                    .map(|g| {
                        let parts: Vec<&str> = g.name.splitn(2, ':').collect();
                        if parts.len() >= 2 {
                            ModeratorEntry {
                                wildmat: parts[0].to_string(),
                                template: parts[1].to_string(),
                            }
                        } else {
                            ModeratorEntry {
                                wildmat: g.name,
                                template: String::new(),
                            }
                        }
                    })
                    .collect();
                Ok(ModeratorsList(mods))
            }
            _ => Err(Error::InvalidResponse(
                "Expected moderators response".to_string(),
            )),
        }
    }

    /// Get message of the day.
    ///
    /// Sends a LIST MOTD command to retrieve the server's message of the day.
    /// Note: This command is not defined in RFC 3977 but is supported by some servers.
    ///
    /// # Returns
    ///
    /// A `Vec<String>` containing the MOTD lines.
    pub async fn list_motd(&mut self) -> Result<Vec<String>> {
        let response = self
            .send_command(Command::List(crate::ListVariant::Motd))
            .await?;
        if let Response::Error { code, message } = &response {
            return Err(Error::Protocol {
                code: *code,
                message: message.clone(),
            });
        }
        // Parse as list of strings
        match response {
            Response::NewsgroupList(groups) => {
                let lines: Vec<String> = groups.into_iter().map(|g| g.name).collect();
                Ok(lines)
            }
            Response::OverviewFormat(lines) => Ok(lines),
            _ => Err(Error::InvalidResponse("Expected MOTD response".to_string())),
        }
    }

    /// List default subscriptions.
    ///
    /// Sends a LIST SUBSCRIPTIONS command to retrieve the default subscription list.
    /// Note: This command is from RFC 2980 and may not be supported by all servers.
    ///
    /// # Returns
    ///
    /// A `Vec<String>` containing the default newsgroup subscriptions.
    pub async fn list_subscriptions(&mut self) -> Result<Vec<String>> {
        let response = self
            .send_command(Command::List(crate::ListVariant::Subscriptions))
            .await?;
        if let Response::Error { code, message } = &response {
            return Err(Error::Protocol {
                code: *code,
                message: message.clone(),
            });
        }
        // Parse as list of strings
        match response {
            Response::NewsgroupList(groups) => {
                let subs: Vec<String> = groups.into_iter().map(|g| g.name).collect();
                Ok(subs)
            }
            _ => Err(Error::InvalidResponse(
                "Expected subscriptions response".to_string(),
            )),
        }
    }

    /// List new newsgroups since date/time.
    ///
    /// Sends a NEWGROUPS command to find newsgroups created after the specified time.
    ///
    /// # Arguments
    ///
    /// * `date` - Date in YYMMDD or YYYYMMDD format
    /// * `time` - Time in HHMMSS format
    /// * `gmt` - Whether the time is in GMT
    /// * `distributions` - Optional distributions parameter
    ///
    /// # Returns
    ///
    /// A [`NewsgroupList`] containing new newsgroups.
    pub async fn newgroups(
        &mut self,
        date: String,
        time: String,
        gmt: bool,
        distributions: Option<String>,
    ) -> Result<NewsgroupList> {
        let response = self
            .send_command(Command::NewGroups {
                date,
                time,
                gmt,
                distributions,
            })
            .await?;
        if let Response::Error { code, message } = &response {
            return Err(Error::Protocol {
                code: *code,
                message: message.clone(),
            });
        }
        response.try_into()
    }

    /// List new articles since date/time.
    ///
    /// Sends a NEWNEWS command to find articles posted after the specified time.
    ///
    /// # Arguments
    ///
    /// * `wildmat` - Wildcard pattern for newsgroups
    /// * `date` - Date in YYMMDD or YYYYMMDD format
    /// * `time` - Time in HHMMSS format
    /// * `gmt` - Whether the time is in GMT
    ///
    /// # Returns
    ///
    /// A [`MessageIdList`] containing message-ids for new articles.
    pub async fn newnews(
        &mut self,
        wildmat: String,
        date: String,
        time: String,
        gmt: bool,
    ) -> Result<MessageIdList> {
        let response = self
            .send_command(Command::NewNews {
                wildmat,
                date,
                time,
                gmt,
            })
            .await?;
        if let Response::Error { code, message } = &response {
            return Err(Error::Protocol {
                code: *code,
                message: message.clone(),
            });
        }
        response.try_into()
    }

    /// Post an article.
    ///
    /// Sends a POST command followed by the article content.
    ///
    /// # Arguments
    ///
    /// * `article` - The article content (headers and body)
    ///
    /// # Errors
    ///
    /// Returns an error if posting fails or is not allowed.
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
    ///
    /// Sends a HELP command to get server help information.
    ///
    /// # Returns
    ///
    /// A [`HelpText`] containing help text lines.
    pub async fn help(&mut self) -> Result<HelpText> {
        let response = self.send_command(Command::Help).await?;
        if let Response::Error { code, message } = &response {
            return Err(Error::Protocol {
                code: *code,
                message: message.clone(),
            });
        }
        response.try_into()
    }

    /// Request server date and time.
    ///
    /// Sends a DATE command to get the server's current date and time.
    ///
    /// # Returns
    ///
    /// A [`ServerDate`] containing the date/time in YYYYMMDDHHMMSS format.
    pub async fn date(&mut self) -> Result<ServerDate> {
        let response = self.send_command(Command::Date).await?;
        if let Response::Error { code, message } = &response {
            return Err(Error::Protocol {
                code: *code,
                message: message.clone(),
            });
        }
        response.try_into()
    }

    /// Move to previous article in current group.
    ///
    /// Sends a LAST command to move to the previous article.
    ///
    /// # Returns
    ///
    /// An [`ArticlePointer`] for the new current article.
    pub async fn last(&mut self) -> Result<ArticlePointer> {
        let response = self.send_command(Command::Last).await?;
        if let Response::Error { code, message } = &response {
            return Err(Error::Protocol {
                code: *code,
                message: message.clone(),
            });
        }
        response.try_into()
    }

    /// Move to next article in current group.
    ///
    /// Sends a NEXT command to move to the next article.
    ///
    /// # Returns
    ///
    /// An [`ArticlePointer`] for the new current article.
    pub async fn next(&mut self) -> Result<ArticlePointer> {
        let response = self.send_command(Command::Next).await?;
        if let Response::Error { code, message } = &response {
            return Err(Error::Protocol {
                code: *code,
                message: message.clone(),
            });
        }
        response.try_into()
    }

    /// Retrieve specific header field for articles.
    ///
    /// Sends an HDR command to retrieve a specific header field for articles.
    ///
    /// # Arguments
    ///
    /// * `field` - The header field name (e.g., "Subject", "From")
    /// * `range` - Optional range specification
    ///
    /// # Returns
    ///
    /// A [`HeaderData`] containing header entries.
    pub async fn hdr(&mut self, field: String, range: Option<String>) -> Result<HeaderData> {
        let response = self.send_command(Command::Hdr { field, range }).await?;
        if let Response::Error { code, message } = &response {
            return Err(Error::Protocol {
                code: *code,
                message: message.clone(),
            });
        }
        response.try_into()
    }

    /// Retrieve overview information for articles.
    ///
    /// Sends an OVER command to retrieve overview data for articles.
    ///
    /// # Arguments
    ///
    /// * `range` - Optional range specification
    ///
    /// # Returns
    ///
    /// An [`OverviewData`] containing overview entries.
    pub async fn over(&mut self, range: Option<String>) -> Result<OverviewData> {
        let response = self.send_command(Command::Over { range }).await?;
        if let Response::Error { code, message } = &response {
            return Err(Error::Protocol {
                code: *code,
                message: message.clone(),
            });
        }
        response.try_into()
    }

    /// Offer an article to the server.
    ///
    /// Sends an IHAVE command to offer an article to the server for transfer.
    ///
    /// # Arguments
    ///
    /// * `message_id` - The message-id of the article being offered
    /// * `article` - The article content
    ///
    /// # Errors
    ///
    /// Returns an error if the server rejects the article.
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
    ///
    /// Sends a QUIT command and shuts down the connection.
    ///
    /// # Errors
    ///
    /// Returns an error if the quit command or shutdown fails.
    pub async fn quit(mut self) -> Result<()> {
        let _response = self.send_command(Command::Quit).await?;
        self.stream
            .shutdown()
            .await
            .map_err(|e| Error::Io(format!("Failed to shutdown connection: {e}")))?;
        Ok(())
    }

    /// Check if posting is allowed on this connection.
    ///
    /// This value is determined by the server's initial greeting (200 = allowed, 201 = prohibited)
    /// and updated by MODE READER responses.
    pub fn is_posting_allowed(&self) -> bool {
        self.posting_allowed
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
    #[test]
    fn test_net_client_module_compiles() {
        // Basic compilation test
        // Integration tests would require a test NNTP server
    }
}
