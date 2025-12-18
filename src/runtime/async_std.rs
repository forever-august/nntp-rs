//! async-std runtime integration for nntp-rs.
//!
//! This module provides the [`NntpClient`] type alias for use with the async-std async runtime.
//! The client uses [`AsyncStdStream`](crate::runtime::stream::AsyncStdStream) for network I/O.
//!
//! # Example
//!
//! ```no_run
//! use nntp_rs::runtime::async_std::NntpClient;
//!
//! #[async_std::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut client = NntpClient::connect("news.example.com:119").await?;
//!     let capabilities = client.capabilities().await?;
//!     println!("Server capabilities: {:?}", capabilities);
//!     client.quit().await?;
//!     Ok(())
//! }
//! ```

/// NNTP client with async-std integration.
///
/// This is a type alias for [`crate::net_client::NntpClient`] using
/// [`AsyncStdStream`](crate::runtime::stream::AsyncStdStream) as the underlying stream type.
///
/// See [`crate::net_client::NntpClient`] for full documentation of available methods.
pub type NntpClient = crate::net_client::NntpClient<crate::runtime::stream::AsyncStdStream>;
