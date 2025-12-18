//! High-level threading API for NNTP.
//!
//! This module provides a user-friendly API for working with discussion threads
//! in newsgroups. It builds on the low-level protocol operations to provide:
//!
//! - **Thread building**: Organizing articles into threaded discussions
//! - **Article composition**: Building articles for posting with proper headers
//! - **Extension traits**: Adding high-level methods to NNTP clients
//!
//! # Overview
//!
//! The threading API consists of several key types:
//!
//! - [`ThreadedArticleRef`]: A lightweight reference to an article with threading metadata
//! - [`ThreadNode`]: A node in the thread tree containing an article and its replies
//! - [`Thread`]: A complete discussion thread as a tree structure
//! - [`ThreadCollection`]: A collection of threads from a newsgroup
//! - [`ArticleBuilder`]: A fluent builder for composing new articles
//! - [`FetchedArticle`]: A fully fetched article with threading context
//! - [`NntpClientThreadingExt`]: Extension trait adding threading operations to clients
//!
//! # Example
//!
//! ```no_run
//! use nntp_rs::threading::{NntpClientThreadingExt, ArticleBuilder};
//! # use nntp_rs::runtime::tokio::NntpClient;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let mut client = NntpClient::connect("news.example.com:119").await?;
//! // Fetch recent threads from a newsgroup
//! let threads = client.recent_threads("comp.lang.rust", 100).await?;
//!
//! println!("Found {} threads", threads.len());
//! for thread in threads.iter().take(5) {
//!     println!("  {} ({} articles)", thread.subject(), thread.article_count());
//! }
//!
//! // Build and post a new article
//! let article = ArticleBuilder::new()
//!     .from("User <user@example.com>")
//!     .subject("Hello from nntp-rs!")
//!     .newsgroup("misc.test")
//!     .body("This is a test message.")
//!     .build()?;
//!
//! client.post_article(&article).await?;
//! # Ok(())
//! # }
//! ```

mod algorithm;
mod builder;
#[cfg(any(
    feature = "tokio-runtime",
    feature = "async-std-runtime",
    feature = "smol-runtime"
))]
mod ext;
pub(crate) mod incremental;
mod types;

// Re-export public types
pub use algorithm::{build_threads, normalize_subject};
pub use builder::{ArticleBuilder, FetchedArticle};
#[cfg(any(
    feature = "tokio-runtime",
    feature = "async-std-runtime",
    feature = "smol-runtime"
))]
pub use ext::NntpClientThreadingExt;
pub use types::{
    Thread, ThreadCollection, ThreadIterator, ThreadNode, ThreadNodeIterator, ThreadedArticleRef,
};
