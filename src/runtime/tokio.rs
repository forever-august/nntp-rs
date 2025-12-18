//! Tokio runtime integration for nntp-rs.
//!
//! This module provides the [`NntpClient`] type alias for use with the Tokio async runtime.
//! The client uses [`TokioStream`](crate::runtime::stream::TokioStream) for network I/O.
//!
//! # Example
//!
//! ```no_run
//! use nntp_rs::runtime::tokio::NntpClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut client = NntpClient::connect("news.example.com:119").await?;
//!     let capabilities = client.capabilities().await?;
//!     println!("Server capabilities: {:?}", capabilities);
//!     client.quit().await?;
//!     Ok(())
//! }
//! ```

/// NNTP client with Tokio integration.
///
/// This is a type alias for [`crate::net_client::NntpClient`] using
/// [`TokioStream`](crate::runtime::stream::TokioStream) as the underlying stream type.
///
/// See [`crate::net_client::NntpClient`] for full documentation of available methods.
pub type NntpClient = crate::net_client::NntpClient<crate::runtime::stream::TokioStream>;
