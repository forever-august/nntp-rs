//! Extension trait for high-level threading operations.
//!
//! This module defines the `NntpClientThreadingExt` trait which adds
//! thread-aware operations to NNTP clients.

use async_trait::async_trait;

use crate::error::{Error, Result};
use crate::net_client::NntpClient;
use crate::runtime::AsyncStream;
use crate::ArticleSpec;

use super::algorithm::build_threads;
use super::builder::FetchedArticle;
use super::incremental::{FetchOptions, IncrementalThreadBuilder};
use super::types::{ThreadCollection, ThreadedArticleRef};

/// Extension trait adding high-level threading operations to NNTP clients.
///
/// This trait provides a user-friendly API for working with discussion threads,
/// building on top of the low-level NNTP protocol operations.
///
/// # Example
///
/// ```no_run
/// use nntp_rs::threading::NntpClientThreadingExt;
/// # use nntp_rs::runtime::tokio::NntpClient;
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let mut client = NntpClient::connect("news.example.com:119").await?;
/// // Fetch recent threads from a newsgroup
/// let threads = client.recent_threads("comp.lang.rust", 50).await?;
///
/// for thread in threads.iter() {
///     println!("{}: {} articles", thread.subject(), thread.article_count());
/// }
/// # Ok(())
/// # }
/// ```
#[async_trait]
pub trait NntpClientThreadingExt {
    /// Fetch threads from a newsgroup for a range of articles.
    ///
    /// # Arguments
    ///
    /// * `group` - The newsgroup name
    /// * `range` - Optional article range (e.g., "1-100", "500-")
    ///
    /// # Returns
    ///
    /// A `ThreadCollection` containing all threads found in the range.
    async fn fetch_threads(&mut self, group: &str, range: Option<&str>)
        -> Result<ThreadCollection>;

    /// Fetch a full article by Message-ID.
    ///
    /// # Arguments
    ///
    /// * `message_id` - The Message-ID of the article
    ///
    /// # Returns
    ///
    /// A `FetchedArticle` with full content and threading metadata.
    async fn fetch_article(&mut self, message_id: &str) -> Result<FetchedArticle>;

    /// Post a new article.
    ///
    /// The article should be a complete message with headers and body,
    /// typically built using `ArticleBuilder`.
    ///
    /// # Arguments
    ///
    /// * `article` - The complete article content
    async fn post_article(&mut self, article: &str) -> Result<()>;

    /// Get recent threads from a newsgroup.
    ///
    /// This is a convenience method that fetches the most recent articles
    /// and organizes them into threads.
    ///
    /// # Arguments
    ///
    /// * `group` - The newsgroup name
    /// * `count` - Maximum number of articles to fetch
    ///
    /// # Returns
    ///
    /// A `ThreadCollection` with threads from the most recent articles.
    async fn recent_threads(&mut self, group: &str, count: u64) -> Result<ThreadCollection>;

    /// Fetch recent threads with incremental backfilling of missing parents.
    ///
    /// This method:
    /// 1. Fetches the most recent `count` articles
    /// 2. Builds initial thread trees (with placeholders for missing parents)
    /// 3. Fetches missing parent articles up to the specified limits
    /// 4. Returns the completed thread collection
    ///
    /// # Arguments
    ///
    /// * `group` - The newsgroup name
    /// * `count` - Number of recent articles to fetch initially
    /// * `max_backfill` - Maximum number of missing parent articles to fetch (default: 500)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nntp_rs::threading::NntpClientThreadingExt;
    /// # use nntp_rs::runtime::tokio::NntpClient;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = NntpClient::connect("news.example.com:119").await?;
    /// // Fetch 100 recent articles, then backfill up to 50 missing parents
    /// let threads = client.recent_threads_with_backfill("comp.lang.rust", 100, Some(50)).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn recent_threads_with_backfill(
        &mut self,
        group: &str,
        count: u64,
        max_backfill: Option<usize>,
    ) -> Result<ThreadCollection>;
}

/// Blanket implementation of `NntpClientThreadingExt` for all `NntpClient<S>` where `S: AsyncStream`.
#[async_trait]
impl<S: AsyncStream> NntpClientThreadingExt for NntpClient<S> {
    async fn fetch_threads(
        &mut self,
        group: &str,
        range: Option<&str>,
    ) -> Result<ThreadCollection> {
        // Select the group
        self.group(group).await?;

        // Fetch overview data for the range
        let overview = self.over(range.map(|s| s.to_string())).await?;

        // Convert overview entries to ThreadedArticleRefs
        let articles: Vec<ThreadedArticleRef> = overview
            .iter()
            .filter_map(ThreadedArticleRef::from_overview)
            .collect();

        // Build threads from the articles
        Ok(build_threads(articles, group))
    }

    async fn fetch_article(&mut self, message_id: &str) -> Result<FetchedArticle> {
        let article = self
            .article(ArticleSpec::MessageId(message_id.to_string()))
            .await?;
        Ok(FetchedArticle::new(article))
    }

    async fn post_article(&mut self, article: &str) -> Result<()> {
        self.post(article.to_string()).await
    }

    async fn recent_threads(&mut self, group: &str, count: u64) -> Result<ThreadCollection> {
        // Select the group and get article range
        let stats = self.group(group).await?;

        // Calculate the range for the most recent articles
        let start = if stats.last > count {
            stats.last - count + 1
        } else {
            1
        };
        let range = format!("{}-{}", start, stats.last);

        self.fetch_threads(group, Some(&range)).await
    }

    async fn recent_threads_with_backfill(
        &mut self,
        group: &str,
        count: u64,
        max_backfill: Option<usize>,
    ) -> Result<ThreadCollection> {
        let options = FetchOptions {
            max_total_fetches: max_backfill,
            ..FetchOptions::default()
        };

        let mut builder = self.create_thread_builder(group, count).await?;

        let mut total_fetched = 0;
        let mut depth = 0;

        while builder.has_missing_parents() {
            // Check depth limit
            if let Some(max_depth) = options.max_depth {
                if depth >= max_depth {
                    break;
                }
            }

            // Check total fetch limit
            if let Some(max_total) = options.max_total_fetches {
                if total_fetched >= max_total {
                    break;
                }
            }

            // Calculate how many to fetch this iteration
            let remaining = options
                .max_total_fetches
                .map(|max| max - total_fetched)
                .unwrap_or(usize::MAX);
            let batch_size = remaining.min(options.batch_size);

            // Get the missing parents for this batch
            let missing: Vec<String> = builder
                .missing_parents()
                .into_iter()
                .take(batch_size)
                .map(|s| s.to_string())
                .collect();

            if missing.is_empty() {
                break;
            }

            // Fetch each missing article
            for msg_id in &missing {
                match self.fetch_article(msg_id).await {
                    Ok(article) => {
                        builder.add_fetched_article(&article);
                        total_fetched += 1;
                    }
                    Err(Error::Protocol { code: 430, .. }) => {
                        // Article not found
                        builder.mark_unavailable(msg_id);
                    }
                    Err(e) => {
                        if options.continue_on_error {
                            builder.mark_unavailable(msg_id);
                        } else {
                            return Err(e);
                        }
                    }
                }
            }

            depth += 1;
        }

        Ok(builder.into_threads())
    }
}

impl<S: AsyncStream> NntpClient<S> {
    /// Create an incremental thread builder pre-populated with recent articles.
    ///
    /// This is an internal helper for building threads incrementally.
    async fn create_thread_builder(
        &mut self,
        group: &str,
        count: u64,
    ) -> Result<IncrementalThreadBuilder> {
        // Select the group and get article range
        let stats = self.group(group).await?;

        // Calculate the range for the most recent articles
        let start = if stats.last > count {
            stats.last - count + 1
        } else {
            1
        };
        let range = format!("{}-{}", start, stats.last);

        // Fetch overview data
        let overview = self.over(Some(range)).await?;

        // Create builder and populate
        let mut builder = IncrementalThreadBuilder::with_capacity(group, overview.len());
        builder.add_from_overview(&overview);

        Ok(builder)
    }
}
