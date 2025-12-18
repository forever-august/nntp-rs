//! Async runtime integrations for nntp-rs.
//!
//! This module provides abstractions and integrations for various async runtimes,
//! enabling the library to work seamlessly with tokio, async-std, or smol.
//!
//! ## Stream Abstraction
//!
//! The [`stream`] submodule provides the [`AsyncStream`] trait which abstracts
//! over different runtime TCP stream implementations.
//!
//! ## Runtime-Specific Clients
//!
//! Each supported runtime has its own submodule with a pre-configured [`NntpClient`]
//! type alias:
//!
//! - [`tokio`] - For use with the Tokio async runtime
//! - [`async_std`] - For use with the async-std runtime  
//! - [`smol`] - For use with the smol runtime

pub mod stream;

pub use stream::AsyncStream;

#[cfg(feature = "tokio-runtime")]
#[cfg_attr(docsrs, doc(cfg(feature = "tokio-runtime")))]
pub use stream::TokioStream;

#[cfg(feature = "async-std-runtime")]
#[cfg_attr(docsrs, doc(cfg(feature = "async-std-runtime")))]
pub use stream::AsyncStdStream;

#[cfg(feature = "smol-runtime")]
#[cfg_attr(docsrs, doc(cfg(feature = "smol-runtime")))]
pub use stream::SmolStream;

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
