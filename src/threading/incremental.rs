//! Incremental thread building for efficient handling of large groups.
//!
//! This module provides `IncrementalThreadBuilder`, a mutable builder that
//! allows constructing thread trees incrementally as articles are fetched.
//! This is more efficient than rebuilding the entire tree for each new batch.
//!
//! For most use cases, prefer using `NntpClientThreadingExt::recent_threads_with_backfill()`
//! which handles incremental fetching automatically:
//!
//! ```no_run
//! use nntp_rs::threading::NntpClientThreadingExt;
//! # use nntp_rs::runtime::tokio::NntpClient;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let mut client = NntpClient::connect("news.example.com:119").await?;
//! // Fetch 100 recent articles, then backfill up to 50 missing parents
//! let threads = client.recent_threads_with_backfill("comp.lang.rust", 100, Some(50)).await?;
//!
//! for thread in threads.iter() {
//!     println!("{}: {} articles", thread.subject(), thread.article_count());
//! }
//! # Ok(())
//! # }
//! ```

use std::collections::{HashMap, HashSet};

use crate::response::OverviewEntry;

use super::algorithm::build_threads;
use super::builder::FetchedArticle;
use super::types::{ThreadCollection, ThreadedArticleRef};

/// A mutable builder for incrementally constructing thread trees.
///
/// This builder maintains an internal collection of articles and can
/// efficiently add new articles as they are fetched. It tracks which
/// parent articles are missing (placeholders) so they can be fetched
/// on demand.
///
/// # Performance
///
/// - Adding articles: O(1) per article
/// - Building threads: O(n) where n = total articles
/// - Querying missing parents: O(1) (cached)
///
/// The builder caches the set of missing parents and updates it
/// incrementally as articles are added, avoiding repeated scans.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct IncrementalThreadBuilder {
    /// The newsgroup name
    group: String,
    /// All articles indexed by Message-ID
    articles: HashMap<String, ThreadedArticleRef>,
    /// Message-IDs that are referenced but not present
    missing_parents: HashSet<String>,
    /// Message-IDs that we tried to fetch but weren't available
    unavailable: HashSet<String>,
    /// Article number ranges that have been fetched
    fetched_ranges: Vec<(u64, u64)>,
    /// Whether the missing_parents cache needs rebuilding
    dirty: bool,
}

#[allow(dead_code)]
impl IncrementalThreadBuilder {
    /// Create a new builder for the given newsgroup.
    pub fn new(group: &str) -> Self {
        Self {
            group: group.to_string(),
            articles: HashMap::new(),
            missing_parents: HashSet::new(),
            unavailable: HashSet::new(),
            fetched_ranges: Vec::new(),
            dirty: false,
        }
    }

    /// Create a new builder with pre-allocated capacity.
    ///
    /// Use this when you know approximately how many articles you'll be processing.
    pub fn with_capacity(group: &str, capacity: usize) -> Self {
        Self {
            group: group.to_string(),
            articles: HashMap::with_capacity(capacity),
            missing_parents: HashSet::with_capacity(capacity / 4),
            unavailable: HashSet::new(),
            fetched_ranges: Vec::new(),
            dirty: false,
        }
    }

    /// Get the newsgroup name.
    pub fn group(&self) -> &str {
        &self.group
    }

    /// Get the number of articles currently in the builder.
    pub fn article_count(&self) -> usize {
        self.articles.len()
    }

    /// Check if the builder has any articles.
    pub fn is_empty(&self) -> bool {
        self.articles.is_empty()
    }

    /// Add articles from an OVER (XOVER) response.
    ///
    /// This is the primary way to add articles in bulk. The overview
    /// entries are converted to `ThreadedArticleRef` and added to the
    /// internal collection.
    ///
    /// # Arguments
    ///
    /// * `entries` - Slice of overview entries from an OVER command
    ///
    /// # Returns
    ///
    /// The number of articles successfully added (some may be skipped
    /// if they're missing required fields).
    pub fn add_from_overview(&mut self, entries: &[OverviewEntry]) -> usize {
        let mut added = 0;
        for entry in entries {
            if let Some(article) = ThreadedArticleRef::from_overview(entry) {
                self.add_article_internal(article);
                added += 1;
            }
        }

        // Track the range if we can determine it
        if let (Some(first), Some(last)) = (
            entries.first().and_then(|e| e.number()),
            entries.last().and_then(|e| e.number()),
        ) {
            self.fetched_ranges.push((first, last));
        }

        added
    }

    /// Add a single article from a full ARTICLE fetch.
    ///
    /// Use this to fill in placeholder nodes after fetching missing articles.
    pub fn add_fetched_article(&mut self, article: &FetchedArticle) -> bool {
        let article_ref = ThreadedArticleRef {
            message_id: article.message_id().to_string(),
            number: None, // Full fetch doesn't give us the article number
            subject: article.subject().to_string(),
            from: article.from().to_string(),
            date: article.date().to_string(),
            parent_id: article.parent_id().map(|s| s.to_string()),
            references: article.references().iter().map(|s| s.to_string()).collect(),
            byte_count: None,
            line_count: None,
        };

        let message_id = article_ref.message_id.clone();
        let was_missing = self.missing_parents.remove(&message_id);
        self.add_article_internal(article_ref);
        was_missing
    }

    /// Add a `ThreadedArticleRef` directly.
    pub fn add_article(&mut self, article: ThreadedArticleRef) {
        self.add_article_internal(article);
    }

    /// Internal method to add an article and update tracking.
    fn add_article_internal(&mut self, article: ThreadedArticleRef) {
        let message_id = article.message_id.clone();

        // Remove from missing if it was there
        self.missing_parents.remove(&message_id);

        // Check if this article's parent is missing
        if let Some(ref parent_id) = article.parent_id {
            if !self.articles.contains_key(parent_id)
                && !self.unavailable.contains(parent_id)
            {
                self.missing_parents.insert(parent_id.clone());
            }
        }

        // Insert the article
        self.articles.insert(message_id, article);
    }

    /// Mark a Message-ID as unavailable (tried to fetch but failed).
    ///
    /// This prevents the builder from repeatedly suggesting this ID
    /// as a missing parent.
    pub fn mark_unavailable(&mut self, message_id: &str) {
        self.missing_parents.remove(message_id);
        self.unavailable.insert(message_id.to_string());
    }

    /// Get all Message-IDs that are referenced but not present.
    ///
    /// These are the "placeholder" parents that could be fetched to
    /// complete the thread trees.
    ///
    /// # Returns
    ///
    /// A vector of Message-IDs that are missing. These are sorted
    /// for deterministic ordering.
    pub fn missing_parents(&self) -> Vec<&str> {
        let mut missing: Vec<&str> = self
            .missing_parents
            .iter()
            .map(|s| s.as_str())
            .collect();
        missing.sort();
        missing
    }

    /// Get the count of missing parent articles.
    pub fn missing_count(&self) -> usize {
        self.missing_parents.len()
    }

    /// Check if there are any missing parents that could be fetched.
    pub fn has_missing_parents(&self) -> bool {
        !self.missing_parents.is_empty()
    }

    /// Get Message-IDs that were marked as unavailable.
    pub fn unavailable(&self) -> Vec<&str> {
        self.unavailable.iter().map(|s| s.as_str()).collect()
    }

    /// Get the article number ranges that have been fetched.
    pub fn fetched_ranges(&self) -> &[(u64, u64)] {
        &self.fetched_ranges
    }

    /// Check if a specific article number has been fetched.
    pub fn is_number_fetched(&self, number: u64) -> bool {
        self.fetched_ranges
            .iter()
            .any(|(start, end)| number >= *start && number <= *end)
    }

    /// Check if a specific Message-ID is present.
    pub fn contains(&self, message_id: &str) -> bool {
        self.articles.contains_key(message_id)
    }

    /// Get an article by Message-ID.
    pub fn get(&self, message_id: &str) -> Option<&ThreadedArticleRef> {
        self.articles.get(message_id)
    }

    /// Build the thread collection from the current state.
    ///
    /// This constructs the full thread tree structure from all articles
    /// currently in the builder. The operation is O(n) where n is the
    /// number of articles.
    ///
    /// The builder retains all articles after building, so you can
    /// continue adding articles and rebuild.
    pub fn build(&self) -> ThreadCollection {
        let articles: Vec<ThreadedArticleRef> = self.articles.values().cloned().collect();
        build_threads(articles, &self.group)
    }

    /// Build threads and consume the builder.
    ///
    /// This is slightly more efficient than `build()` as it avoids
    /// cloning the articles.
    pub fn into_threads(self) -> ThreadCollection {
        let articles: Vec<ThreadedArticleRef> = self.articles.into_values().collect();
        build_threads(articles, &self.group)
    }

    /// Clear all articles and reset the builder.
    pub fn clear(&mut self) {
        self.articles.clear();
        self.missing_parents.clear();
        self.unavailable.clear();
        self.fetched_ranges.clear();
        self.dirty = false;
    }

    /// Merge another builder into this one.
    ///
    /// This is useful when fetching from multiple sources or when
    /// combining results from parallel fetches.
    pub fn merge(&mut self, other: IncrementalThreadBuilder) {
        for article in other.articles.into_values() {
            self.add_article_internal(article);
        }
        for msg_id in other.unavailable {
            self.missing_parents.remove(&msg_id);
            self.unavailable.insert(msg_id);
        }
        self.fetched_ranges.extend(other.fetched_ranges);
    }

    /// Rebuild the missing parents cache from scratch.
    ///
    /// This is called automatically when needed, but can be called
    /// manually if you suspect the cache is out of sync.
    pub fn rebuild_missing_cache(&mut self) {
        self.missing_parents.clear();

        for article in self.articles.values() {
            if let Some(ref parent_id) = article.parent_id {
                if !self.articles.contains_key(parent_id)
                    && !self.unavailable.contains(parent_id)
                {
                    self.missing_parents.insert(parent_id.clone());
                }
            }
        }

        self.dirty = false;
    }
}

/// Options for controlling incremental fetching behavior.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FetchOptions {
    /// Maximum number of missing parents to fetch per iteration
    pub batch_size: usize,
    /// Maximum depth of ancestors to fetch (None = unlimited)
    pub max_depth: Option<usize>,
    /// Whether to continue fetching if some articles fail
    pub continue_on_error: bool,
    /// Maximum total articles to fetch (None = unlimited)
    pub max_total_fetches: Option<usize>,
}

impl Default for FetchOptions {
    fn default() -> Self {
        Self {
            batch_size: 50,
            max_depth: Some(10),
            continue_on_error: true,
            max_total_fetches: Some(500),
        }
    }
}

#[allow(dead_code)]
impl FetchOptions {
    /// Create options for fetching all missing parents without limits.
    pub fn unlimited() -> Self {
        Self {
            batch_size: 100,
            max_depth: None,
            continue_on_error: true,
            max_total_fetches: None,
        }
    }

    /// Create options that only fetch immediate parents (depth 1).
    pub fn shallow() -> Self {
        Self {
            batch_size: 50,
            max_depth: Some(1),
            continue_on_error: true,
            max_total_fetches: Some(100),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_article(
        message_id: &str,
        subject: &str,
        parent_id: Option<&str>,
    ) -> ThreadedArticleRef {
        ThreadedArticleRef {
            message_id: message_id.to_string(),
            number: Some(1),
            subject: subject.to_string(),
            from: "test@example.com".to_string(),
            date: "2024-01-01".to_string(),
            parent_id: parent_id.map(|s| s.to_string()),
            references: parent_id.map(|s| vec![s.to_string()]).unwrap_or_default(),
            byte_count: Some(100),
            line_count: Some(10),
        }
    }

    #[test]
    fn test_builder_new() {
        let builder = IncrementalThreadBuilder::new("test.group");
        assert_eq!(builder.group(), "test.group");
        assert!(builder.is_empty());
        assert_eq!(builder.article_count(), 0);
        assert!(!builder.has_missing_parents());
    }

    #[test]
    fn test_builder_add_article() {
        let mut builder = IncrementalThreadBuilder::new("test.group");
        
        let article = make_article("<root@x.com>", "Hello", None);
        builder.add_article(article);

        assert_eq!(builder.article_count(), 1);
        assert!(builder.contains("<root@x.com>"));
        assert!(!builder.has_missing_parents());
    }

    #[test]
    fn test_builder_tracks_missing_parents() {
        let mut builder = IncrementalThreadBuilder::new("test.group");

        // Add a reply without its parent
        let reply = make_article("<reply@x.com>", "Re: Hello", Some("<root@x.com>"));
        builder.add_article(reply);

        assert_eq!(builder.article_count(), 1);
        assert!(builder.has_missing_parents());
        assert_eq!(builder.missing_count(), 1);
        assert!(builder.missing_parents().contains(&"<root@x.com>"));
    }

    #[test]
    fn test_builder_resolves_missing_when_added() {
        let mut builder = IncrementalThreadBuilder::new("test.group");

        // Add a reply first (parent is missing)
        let reply = make_article("<reply@x.com>", "Re: Hello", Some("<root@x.com>"));
        builder.add_article(reply);
        assert!(builder.has_missing_parents());

        // Add the parent
        let root = make_article("<root@x.com>", "Hello", None);
        builder.add_article(root);

        // Parent should no longer be missing
        assert!(!builder.has_missing_parents());
        assert_eq!(builder.missing_count(), 0);
    }

    #[test]
    fn test_builder_mark_unavailable() {
        let mut builder = IncrementalThreadBuilder::new("test.group");

        // Add a reply with missing parent
        let reply = make_article("<reply@x.com>", "Re: Hello", Some("<root@x.com>"));
        builder.add_article(reply);
        assert!(builder.has_missing_parents());

        // Mark parent as unavailable
        builder.mark_unavailable("<root@x.com>");

        // Should no longer be in missing parents
        assert!(!builder.has_missing_parents());
        assert!(builder.unavailable().contains(&"<root@x.com>"));
    }

    #[test]
    fn test_builder_build_threads() {
        let mut builder = IncrementalThreadBuilder::new("test.group");

        builder.add_article(make_article("<root@x.com>", "Hello", None));
        builder.add_article(make_article("<reply1@x.com>", "Re: Hello", Some("<root@x.com>")));
        builder.add_article(make_article("<reply2@x.com>", "Re: Hello", Some("<root@x.com>")));

        let threads = builder.build();
        assert_eq!(threads.len(), 1);
        assert_eq!(threads.total_articles(), 3);
        assert_eq!(threads.threads()[0].root().reply_count(), 2);
    }

    #[test]
    fn test_builder_multiple_threads() {
        let mut builder = IncrementalThreadBuilder::new("test.group");

        builder.add_article(make_article("<t1@x.com>", "Thread 1", None));
        builder.add_article(make_article("<t2@x.com>", "Thread 2", None));
        builder.add_article(make_article("<t1r@x.com>", "Re: Thread 1", Some("<t1@x.com>")));

        let threads = builder.build();
        assert_eq!(threads.len(), 2);
        assert_eq!(threads.total_articles(), 3);
    }

    #[test]
    fn test_builder_retains_after_build() {
        let mut builder = IncrementalThreadBuilder::new("test.group");

        builder.add_article(make_article("<root@x.com>", "Hello", None));
        let threads1 = builder.build();
        assert_eq!(threads1.len(), 1);

        // Builder still has articles
        assert_eq!(builder.article_count(), 1);

        // Can add more and rebuild
        builder.add_article(make_article("<reply@x.com>", "Re: Hello", Some("<root@x.com>")));
        let threads2 = builder.build();
        assert_eq!(threads2.total_articles(), 2);
    }

    #[test]
    fn test_builder_into_threads() {
        let mut builder = IncrementalThreadBuilder::new("test.group");

        builder.add_article(make_article("<root@x.com>", "Hello", None));
        builder.add_article(make_article("<reply@x.com>", "Re: Hello", Some("<root@x.com>")));

        let threads = builder.into_threads();
        assert_eq!(threads.len(), 1);
        assert_eq!(threads.total_articles(), 2);
        // builder is consumed
    }

    #[test]
    fn test_builder_merge() {
        let mut builder1 = IncrementalThreadBuilder::new("test.group");
        builder1.add_article(make_article("<a@x.com>", "Article A", None));
        builder1.add_article(make_article("<c@x.com>", "Re: B", Some("<b@x.com>"))); // missing parent

        let mut builder2 = IncrementalThreadBuilder::new("test.group");
        builder2.add_article(make_article("<b@x.com>", "Article B", None));
        builder2.mark_unavailable("<missing@x.com>");

        builder1.merge(builder2);

        assert_eq!(builder1.article_count(), 3);
        assert!(!builder1.has_missing_parents()); // <b@x.com> now present
        assert!(builder1.unavailable().contains(&"<missing@x.com>"));
    }

    #[test]
    fn test_builder_clear() {
        let mut builder = IncrementalThreadBuilder::new("test.group");
        builder.add_article(make_article("<root@x.com>", "Hello", None));
        builder.add_article(make_article("<reply@x.com>", "Re: Hello", Some("<missing@x.com>")));
        builder.mark_unavailable("<other@x.com>");

        builder.clear();

        assert!(builder.is_empty());
        assert!(!builder.has_missing_parents());
        assert!(builder.unavailable().is_empty());
        assert!(builder.fetched_ranges().is_empty());
    }

    #[test]
    fn test_builder_with_capacity() {
        let builder = IncrementalThreadBuilder::with_capacity("test.group", 10000);
        assert!(builder.is_empty());
        assert_eq!(builder.group(), "test.group");
    }

    #[test]
    fn test_builder_get_article() {
        let mut builder = IncrementalThreadBuilder::new("test.group");
        builder.add_article(make_article("<root@x.com>", "Hello", None));

        let article = builder.get("<root@x.com>");
        assert!(article.is_some());
        assert_eq!(article.unwrap().subject, "Hello");

        assert!(builder.get("<nonexistent@x.com>").is_none());
    }

    #[test]
    fn test_fetch_options_default() {
        let opts = FetchOptions::default();
        assert_eq!(opts.batch_size, 50);
        assert_eq!(opts.max_depth, Some(10));
        assert!(opts.continue_on_error);
        assert_eq!(opts.max_total_fetches, Some(500));
    }

    #[test]
    fn test_fetch_options_unlimited() {
        let opts = FetchOptions::unlimited();
        assert!(opts.max_depth.is_none());
        assert!(opts.max_total_fetches.is_none());
    }

    #[test]
    fn test_fetch_options_shallow() {
        let opts = FetchOptions::shallow();
        assert_eq!(opts.max_depth, Some(1));
    }

    #[test]
    fn test_add_from_overview() {
        use crate::response::OverviewEntry;
        
        let mut builder = IncrementalThreadBuilder::new("test.group");
        
        // Create overview entries with tab-separated fields
        let entry1 = OverviewEntry { 
            fields: vec![
                "100".to_string(),
                "Subject 1".to_string(),
                "user@example.com".to_string(),
                "2024-01-01".to_string(),
                "<msg1@example.com>".to_string(),
                "".to_string(),
                "1000".to_string(),
                "50".to_string(),
            ]
        };
        let entry2 = OverviewEntry { 
            fields: vec![
                "101".to_string(),
                "Subject 2".to_string(),
                "user@example.com".to_string(),
                "2024-01-02".to_string(),
                "<msg2@example.com>".to_string(),
                "".to_string(),
                "2000".to_string(),
                "60".to_string(),
            ]
        };
        
        let count = builder.add_from_overview(&[entry1, entry2]);
        assert_eq!(count, 2);
        assert_eq!(builder.article_count(), 2);
        
        // Check fetched ranges
        let ranges = builder.fetched_ranges();
        assert!(!ranges.is_empty());
        assert!(builder.is_number_fetched(100));
        assert!(builder.is_number_fetched(101));
        assert!(!builder.is_number_fetched(99));
    }

    #[test]
    fn test_add_fetched_article() {
        use crate::response::Article;
        use crate::threading::builder::FetchedArticle;
        
        let mut builder = IncrementalThreadBuilder::new("test.group");
        
        // Add an article with a missing parent
        builder.add_article(make_article("<reply@x.com>", "Re: Hello", Some("<root@x.com>")));
        
        assert!(builder.has_missing_parents());
        assert!(builder.missing_parents().contains(&"<root@x.com>"));
        
        // Now add the parent via FetchedArticle
        let content = b"From: test@example.com\r\nSubject: Hello\r\nMessage-ID: <root@x.com>\r\n\r\nBody\r\n".to_vec();
        let article = Article::new(None, "<root@x.com>".to_string(), content);
        let fetched = FetchedArticle::new(article);
        
        let was_missing = builder.add_fetched_article(&fetched);
        assert!(was_missing);
        assert!(!builder.has_missing_parents());
    }

    #[test]
    fn test_rebuild_missing_cache() {
        let mut builder = IncrementalThreadBuilder::new("test.group");
        
        // Add articles with missing parents
        builder.add_article(make_article("<reply@x.com>", "Re: Hello", Some("<root@x.com>")));
        assert!(builder.has_missing_parents());
        
        // Mark the parent as unavailable
        builder.mark_unavailable("<root@x.com>");
        
        // Rebuild cache
        builder.rebuild_missing_cache();
        
        // Should have no missing parents since root is unavailable
        assert!(!builder.has_missing_parents());
    }

    #[test]
    fn test_contains() {
        let mut builder = IncrementalThreadBuilder::new("test.group");
        builder.add_article(make_article("<root@x.com>", "Hello", None));
        
        assert!(builder.contains("<root@x.com>"));
        assert!(!builder.contains("<other@x.com>"));
    }

    #[test]
    fn test_is_number_fetched() {
        let mut builder = IncrementalThreadBuilder::new("test.group");
        
        // Manually add range tracking by using add_from_overview
        use crate::response::OverviewEntry;
        let entry = OverviewEntry { 
            fields: vec![
                "100".to_string(),
                "Subject".to_string(),
                "user@example.com".to_string(),
                "2024-01-01".to_string(),
                "<msg@example.com>".to_string(),
                "".to_string(),
                "1000".to_string(),
                "50".to_string(),
            ]
        };
        builder.add_from_overview(&[entry]);
        
        assert!(builder.is_number_fetched(100));
        assert!(!builder.is_number_fetched(50));
    }

    #[test]
    fn test_unavailable() {
        let mut builder = IncrementalThreadBuilder::new("test.group");
        
        builder.mark_unavailable("<missing1@x.com>");
        builder.mark_unavailable("<missing2@x.com>");
        
        let unavailable = builder.unavailable();
        assert_eq!(unavailable.len(), 2);
        assert!(unavailable.contains(&"<missing1@x.com>"));
        assert!(unavailable.contains(&"<missing2@x.com>"));
    }
}
