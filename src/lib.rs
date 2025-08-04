//! # nntp-rs
//!
//! A sans-io NNTP (Network News Transfer Protocol) client library for Rust.
//!
//! This library provides a safe, ergonomic, and efficient way to interact with NNTP servers
//! while maintaining separation between protocol logic and I/O operations.
//!
//! ## Design Philosophy
//!
//! This library follows the "sans-io" design pattern:
//! - **Protocol Logic**: Core NNTP protocol implementation handles parsing and generation
//! - **I/O Separation**: Network operations are handled separately by the user or runtime integrations
//! - **Flexibility**: Works with any async runtime, transport, or I/O model
//!
//! ## Examples
//!
//! ### Sans-IO Usage
//!
//! ```rust
//! use nntp_rs::{Client, Command};
//!
//! let mut client = Client::new();
//! let command = Command::Capabilities;
//! let request_bytes = client.encode_command(command).unwrap();
//! // Send request_bytes through your I/O layer
//! ```
//!
//! ### With Runtime Integration
//!
//! ```rust,no_run
//! # #[cfg(feature = "tokio-runtime")]
//! # {
//! use nntp_rs::tokio::NntpClient;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut client = NntpClient::connect("news.example.com:119").await?;
//! let capabilities = client.capabilities().await?;
//! # Ok(())
//! # }
//! # }
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod client;
pub mod command;
pub mod error;
pub mod response;

// Mock server for testing
pub mod mock;

pub use client::Client;
pub use command::{Command, ArticleSpec};
pub use error::{Error, Result};
pub use response::Response;

// Optional async runtime integrations
#[cfg(feature = "tokio-runtime")]
#[cfg_attr(docsrs, doc(cfg(feature = "tokio-runtime")))]
pub mod tokio;

#[cfg(feature = "async-std-runtime")]
#[cfg_attr(docsrs, doc(cfg(feature = "async-std-runtime")))]
pub mod async_std;

#[cfg(feature = "smol-runtime")]
#[cfg_attr(docsrs, doc(cfg(feature = "smol-runtime")))]
pub mod smol;
