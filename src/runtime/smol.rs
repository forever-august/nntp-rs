//! smol runtime integration for nntp-rs.
//!
//! This module provides the [`NntpClient`] type alias for use with the smol async runtime.
//! The client uses [`SmolStream`](crate::runtime::stream::SmolStream) for network I/O.
//!
//! # Example
//!
//! ```no_run
//! use nntp_rs::runtime::smol::NntpClient;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     smol::block_on(async {
//!         let mut client = NntpClient::connect("news.example.com:119").await?;
//!         let capabilities = client.capabilities().await?;
//!         println!("Server capabilities: {:?}", capabilities);
//!         client.quit().await?;
//!         Ok(())
//!     })
//! }
//! ```

/// NNTP client with smol integration.
///
/// This is a type alias for [`crate::net_client::NntpClient`] using
/// [`SmolStream`](crate::runtime::stream::SmolStream) as the underlying stream type.
///
/// See [`crate::net_client::NntpClient`] for full documentation of available methods.
pub type NntpClient = crate::net_client::NntpClient<crate::runtime::stream::SmolStream>;
