//! Core types for the high-level threading API.

use crate::response::OverviewEntry;

/// A lightweight reference to an article within a thread.
///
/// Contains threading metadata without full article content.
/// Use this for building thread trees efficiently without fetching
/// the full content of every article.
#[derive(Debug, Clone, PartialEq)]
pub struct ThreadedArticleRef {
    /// Message-ID (globally unique identifier)
    pub message_id: String,
    /// Article number in the group (if known)
    pub number: Option<u64>,
    /// Subject line
    pub subject: String,
    /// Author (From header)
    pub from: String,
    /// Date as string
    pub date: String,
    /// Parent Message-ID (from References/In-Reply-To)
    pub parent_id: Option<String>,
    /// All ancestors from References header (oldest first)
    pub references: Vec<String>,
    /// Byte count (from overview)
    pub byte_count: Option<u64>,
    /// Line count (from overview)
    pub line_count: Option<u64>,
}

impl ThreadedArticleRef {
    /// Create a ThreadedArticleRef from an OverviewEntry.
    ///
    /// Returns None if the overview entry is missing required fields
    /// (message_id, subject, from, date).
    pub fn from_overview(overview: &OverviewEntry) -> Option<Self> {
        let message_id = overview.message_id()?.to_string();
        let references = Self::parse_references(overview.references());
        let parent_id = references.last().cloned();

        Some(Self {
            message_id,
            number: overview.number(),
            subject: overview.subject()?.to_string(),
            from: overview.from()?.to_string(),
            date: overview.date()?.to_string(),
            parent_id,
            references,
            byte_count: overview.byte_count(),
            line_count: overview.line_count(),
        })
    }

    /// Parse References header into list of Message-IDs (oldest first).
    ///
    /// The References header contains space-separated Message-IDs representing
    /// the ancestry of an article, from oldest ancestor to immediate parent.
    pub fn parse_references(references: Option<&str>) -> Vec<String> {
        references
            .map(|refs| {
                refs.split_whitespace()
                    .filter(|s| s.starts_with('<') && s.ends_with('>'))
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Check if this article is a reply (has a parent).
    pub fn is_reply(&self) -> bool {
        self.parent_id.is_some()
    }

    /// Get the depth of this article in the thread (0 for root articles).
    pub fn depth(&self) -> usize {
        self.references.len()
    }
}

/// A node in the thread tree, containing an article reference and its replies.
///
/// The article field is `Option<ThreadedArticleRef>` to handle cases where
/// a referenced parent article is not available (e.g., expired, not fetched,
/// or on a different server). This allows sibling replies to a missing parent
/// to remain grouped together under a placeholder node.
#[derive(Debug, Clone)]
pub struct ThreadNode {
    /// The article at this node, or None for placeholder nodes (missing articles)
    pub article: Option<ThreadedArticleRef>,
    /// Direct replies to this article (children in the tree)
    pub replies: Vec<ThreadNode>,
    /// Message-ID for this node (always present, even for placeholders)
    pub message_id: String,
}

impl ThreadNode {
    /// Create a new thread node with no replies.
    pub fn new(article: ThreadedArticleRef) -> Self {
        let message_id = article.message_id.clone();
        Self {
            article: Some(article),
            replies: Vec::new(),
            message_id,
        }
    }

    /// Create a placeholder node for a missing article.
    ///
    /// Placeholder nodes represent articles that are referenced but not available.
    /// This allows sibling replies to remain grouped together.
    pub fn placeholder(message_id: String) -> Self {
        Self {
            article: None,
            replies: Vec::new(),
            message_id,
        }
    }

    /// Check if this is a placeholder node (missing article).
    pub fn is_placeholder(&self) -> bool {
        self.article.is_none()
    }

    /// Get the number of direct replies to this article.
    pub fn reply_count(&self) -> usize {
        self.replies.len()
    }

    /// Check if this article has any replies.
    pub fn has_replies(&self) -> bool {
        !self.replies.is_empty()
    }

    /// Find a node by Message-ID in this subtree.
    pub fn find_by_message_id(&self, message_id: &str) -> Option<&ThreadNode> {
        if self.message_id == message_id {
            return Some(self);
        }
        for reply in &self.replies {
            if let Some(found) = reply.find_by_message_id(message_id) {
                return Some(found);
            }
        }
        None
    }

    /// Count all present articles in this subtree (excluding placeholders).
    pub fn count_articles(&self) -> usize {
        let self_count = if self.article.is_some() { 1 } else { 0 };
        self_count
            + self
                .replies
                .iter()
                .map(|r| r.count_articles())
                .sum::<usize>()
    }

    /// Count all nodes in this subtree (including placeholders).
    pub fn count_nodes(&self) -> usize {
        1 + self
            .replies
            .iter()
            .map(|r| r.count_nodes())
            .sum::<usize>()
    }

    /// Get the maximum depth of the subtree (0 if no replies).
    pub fn max_depth(&self) -> usize {
        if self.replies.is_empty() {
            0
        } else {
            1 + self
                .replies
                .iter()
                .map(|r| r.max_depth())
                .max()
                .unwrap_or(0)
        }
    }
}

/// A complete discussion thread as a tree structure.
///
/// A thread consists of a root article (the original post) and
/// all its replies organized hierarchically.
#[derive(Debug, Clone)]
pub struct Thread {
    /// The root article (original post) of the thread
    root: ThreadNode,
    /// Total number of articles in this thread
    article_count: usize,
    /// The normalized thread subject (without Re:, Fwd:, etc.)
    subject: String,
}

/// Recursively collect message IDs from a thread node (including placeholders).
fn collect_message_ids<'a>(node: &'a ThreadNode, ids: &mut Vec<&'a str>) {
    ids.push(&node.message_id);
    for reply in &node.replies {
        collect_message_ids(reply, ids);
    }
}

impl Thread {
    /// Create a new thread with the given root node.
    pub fn new(root: ThreadNode, subject: String) -> Self {
        let article_count = root.count_articles();
        Self {
            root,
            article_count,
            subject,
        }
    }

    /// Get the root article of the thread.
    pub fn root(&self) -> &ThreadNode {
        &self.root
    }

    /// Get the thread subject (normalized, without Re: prefixes).
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// Get the total number of articles in this thread.
    pub fn article_count(&self) -> usize {
        self.article_count
    }

    /// Get the Message-ID of the root node.
    pub fn root_message_id(&self) -> &str {
        &self.root.message_id
    }

    /// Find an article by Message-ID within this thread.
    pub fn find_by_message_id(&self, message_id: &str) -> Option<&ThreadNode> {
        self.root.find_by_message_id(message_id)
    }

    /// Get the depth of the thread (longest reply chain).
    pub fn max_depth(&self) -> usize {
        self.root.max_depth()
    }

    /// Get all Message-IDs in this thread.
    pub fn all_message_ids(&self) -> Vec<&str> {
        let mut ids = Vec::with_capacity(self.article_count);
        collect_message_ids(&self.root, &mut ids);
        ids
    }

    /// Iterate over all articles in the thread (depth-first traversal).
    ///
    /// This iterator skips placeholder nodes and only yields present articles.
    pub fn iter(&self) -> ThreadIterator<'_> {
        ThreadIterator::new(&self.root)
    }

    /// Iterate over all nodes in the thread (depth-first traversal), including placeholders.
    pub fn iter_nodes(&self) -> ThreadNodeIterator<'_> {
        ThreadNodeIterator::new(&self.root)
    }

    /// Check if the root of this thread is a placeholder (missing article).
    pub fn has_placeholder_root(&self) -> bool {
        self.root.is_placeholder()
    }
}

/// Iterator over all articles in a thread (depth-first traversal).
///
/// This iterator skips placeholder nodes and only yields present articles.
pub struct ThreadIterator<'a> {
    stack: Vec<&'a ThreadNode>,
}

impl<'a> ThreadIterator<'a> {
    fn new(root: &'a ThreadNode) -> Self {
        Self { stack: vec![root] }
    }
}

impl<'a> Iterator for ThreadIterator<'a> {
    type Item = &'a ThreadedArticleRef;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let node = self.stack.pop()?;
            // Push replies in reverse order so they're processed left-to-right
            for reply in node.replies.iter().rev() {
                self.stack.push(reply);
            }
            // Skip placeholder nodes, only return present articles
            if let Some(ref article) = node.article {
                return Some(article);
            }
            // Continue to next node if this was a placeholder
        }
    }
}

/// Iterator over all nodes in a thread (depth-first traversal), including placeholders.
pub struct ThreadNodeIterator<'a> {
    stack: Vec<&'a ThreadNode>,
}

impl<'a> ThreadNodeIterator<'a> {
    /// Create a new iterator starting from the given root node.
    pub fn new(root: &'a ThreadNode) -> Self {
        Self { stack: vec![root] }
    }
}

impl<'a> Iterator for ThreadNodeIterator<'a> {
    type Item = &'a ThreadNode;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.stack.pop()?;
        // Push replies in reverse order so they're processed left-to-right
        for reply in node.replies.iter().rev() {
            self.stack.push(reply);
        }
        Some(node)
    }
}

/// A collection of threads from a newsgroup.
#[derive(Debug, Clone)]
pub struct ThreadCollection {
    /// The newsgroup these threads are from
    pub group: String,
    /// The threads, sorted by most recent activity
    threads: Vec<Thread>,
}

impl ThreadCollection {
    /// Create a new thread collection.
    pub fn new(group: String, threads: Vec<Thread>) -> Self {
        Self { group, threads }
    }

    /// Get all threads.
    pub fn threads(&self) -> &[Thread] {
        &self.threads
    }

    /// Get the number of threads.
    pub fn len(&self) -> usize {
        self.threads.len()
    }

    /// Check if the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.threads.is_empty()
    }

    /// Get the total number of articles across all threads.
    pub fn total_articles(&self) -> usize {
        self.threads.iter().map(|t| t.article_count()).sum()
    }

    /// Find a thread containing a specific Message-ID.
    pub fn find_thread_by_message_id(&self, message_id: &str) -> Option<&Thread> {
        self.threads
            .iter()
            .find(|t| t.find_by_message_id(message_id).is_some())
    }

    /// Iterate over threads.
    pub fn iter(&self) -> impl Iterator<Item = &Thread> {
        self.threads.iter()
    }
}

impl IntoIterator for ThreadCollection {
    type Item = Thread;
    type IntoIter = std::vec::IntoIter<Thread>;

    fn into_iter(self) -> Self::IntoIter {
        self.threads.into_iter()
    }
}

impl<'a> IntoIterator for &'a ThreadCollection {
    type Item = &'a Thread;
    type IntoIter = std::slice::Iter<'a, Thread>;

    fn into_iter(self) -> Self::IntoIter {
        self.threads.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_article_ref(
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
    fn test_threaded_article_ref_parse_references() {
        // Empty references
        assert!(ThreadedArticleRef::parse_references(None).is_empty());
        assert!(ThreadedArticleRef::parse_references(Some("")).is_empty());

        // Single reference
        let refs = ThreadedArticleRef::parse_references(Some("<abc@example.com>"));
        assert_eq!(refs, vec!["<abc@example.com>"]);

        // Multiple references
        let refs = ThreadedArticleRef::parse_references(Some("<a@x.com> <b@x.com> <c@x.com>"));
        assert_eq!(refs, vec!["<a@x.com>", "<b@x.com>", "<c@x.com>"]);

        // Invalid entries filtered out
        let refs =
            ThreadedArticleRef::parse_references(Some("<valid@x.com> invalid <also-valid@y.com>"));
        assert_eq!(refs, vec!["<valid@x.com>", "<also-valid@y.com>"]);
    }

    #[test]
    fn test_threaded_article_ref_is_reply() {
        let root = make_article_ref("<root@x.com>", "Hello", None);
        assert!(!root.is_reply());

        let reply = make_article_ref("<reply@x.com>", "Re: Hello", Some("<root@x.com>"));
        assert!(reply.is_reply());
    }

    #[test]
    fn test_thread_node_find_by_message_id() {
        let root = ThreadNode {
            article: Some(make_article_ref("<root@x.com>", "Hello", None)),
            message_id: "<root@x.com>".to_string(),
            replies: vec![
                ThreadNode {
                    article: Some(make_article_ref(
                        "<reply1@x.com>",
                        "Re: Hello",
                        Some("<root@x.com>"),
                    )),
                    message_id: "<reply1@x.com>".to_string(),
                    replies: vec![],
                },
                ThreadNode {
                    article: Some(make_article_ref(
                        "<reply2@x.com>",
                        "Re: Hello",
                        Some("<root@x.com>"),
                    )),
                    message_id: "<reply2@x.com>".to_string(),
                    replies: vec![ThreadNode {
                        article: Some(make_article_ref(
                            "<nested@x.com>",
                            "Re: Re: Hello",
                            Some("<reply2@x.com>"),
                        )),
                        message_id: "<nested@x.com>".to_string(),
                        replies: vec![],
                    }],
                },
            ],
        };

        assert!(root.find_by_message_id("<root@x.com>").is_some());
        assert!(root.find_by_message_id("<reply1@x.com>").is_some());
        assert!(root.find_by_message_id("<nested@x.com>").is_some());
        assert!(root.find_by_message_id("<nonexistent@x.com>").is_none());
    }

    #[test]
    fn test_thread_node_count_articles() {
        let root = ThreadNode {
            article: Some(make_article_ref("<root@x.com>", "Hello", None)),
            message_id: "<root@x.com>".to_string(),
            replies: vec![
                ThreadNode::new(make_article_ref(
                    "<reply1@x.com>",
                    "Re: Hello",
                    Some("<root@x.com>"),
                )),
                ThreadNode {
                    article: Some(make_article_ref(
                        "<reply2@x.com>",
                        "Re: Hello",
                        Some("<root@x.com>"),
                    )),
                    message_id: "<reply2@x.com>".to_string(),
                    replies: vec![ThreadNode::new(make_article_ref(
                        "<nested@x.com>",
                        "Re: Re: Hello",
                        Some("<reply2@x.com>"),
                    ))],
                },
            ],
        };

        assert_eq!(root.count_articles(), 4);
    }

    #[test]
    fn test_thread_node_count_articles_with_placeholder() {
        // Test that placeholders don't count toward article count
        let root = ThreadNode {
            article: None, // Placeholder
            message_id: "<missing@x.com>".to_string(),
            replies: vec![
                ThreadNode::new(make_article_ref(
                    "<reply1@x.com>",
                    "Re: Missing",
                    Some("<missing@x.com>"),
                )),
                ThreadNode::new(make_article_ref(
                    "<reply2@x.com>",
                    "Re: Missing",
                    Some("<missing@x.com>"),
                )),
            ],
        };

        // Only 2 articles, not 3 (placeholder doesn't count)
        assert_eq!(root.count_articles(), 2);
        // But 3 nodes total
        assert_eq!(root.count_nodes(), 3);
        assert!(root.is_placeholder());
    }

    #[test]
    fn test_thread_node_max_depth() {
        // Single node
        let single = ThreadNode::new(make_article_ref("<root@x.com>", "Hello", None));
        assert_eq!(single.max_depth(), 0);

        // One level of replies
        let with_replies = ThreadNode {
            article: Some(make_article_ref("<root@x.com>", "Hello", None)),
            message_id: "<root@x.com>".to_string(),
            replies: vec![ThreadNode::new(make_article_ref(
                "<reply@x.com>",
                "Re: Hello",
                Some("<root@x.com>"),
            ))],
        };
        assert_eq!(with_replies.max_depth(), 1);

        // Nested replies
        let nested = ThreadNode {
            article: Some(make_article_ref("<root@x.com>", "Hello", None)),
            message_id: "<root@x.com>".to_string(),
            replies: vec![ThreadNode {
                article: Some(make_article_ref(
                    "<reply@x.com>",
                    "Re: Hello",
                    Some("<root@x.com>"),
                )),
                message_id: "<reply@x.com>".to_string(),
                replies: vec![ThreadNode::new(make_article_ref(
                    "<nested@x.com>",
                    "Re: Re: Hello",
                    Some("<reply@x.com>"),
                ))],
            }],
        };
        assert_eq!(nested.max_depth(), 2);
    }

    #[test]
    fn test_thread_iterator() {
        let root = ThreadNode {
            article: Some(make_article_ref("<root@x.com>", "Hello", None)),
            message_id: "<root@x.com>".to_string(),
            replies: vec![
                ThreadNode::new(make_article_ref(
                    "<reply1@x.com>",
                    "Re: Hello",
                    Some("<root@x.com>"),
                )),
                ThreadNode::new(make_article_ref(
                    "<reply2@x.com>",
                    "Re: Hello",
                    Some("<root@x.com>"),
                )),
            ],
        };

        let thread = Thread::new(root, "Hello".to_string());
        let message_ids: Vec<&str> = thread.iter().map(|a| a.message_id.as_str()).collect();

        assert_eq!(
            message_ids,
            vec!["<root@x.com>", "<reply1@x.com>", "<reply2@x.com>"]
        );
    }

    #[test]
    fn test_thread_iterator_skips_placeholders() {
        // Iterator should skip placeholder nodes
        let root = ThreadNode {
            article: None, // Placeholder root
            message_id: "<missing@x.com>".to_string(),
            replies: vec![
                ThreadNode::new(make_article_ref(
                    "<reply1@x.com>",
                    "Re: Missing",
                    Some("<missing@x.com>"),
                )),
                ThreadNode::new(make_article_ref(
                    "<reply2@x.com>",
                    "Re: Missing",
                    Some("<missing@x.com>"),
                )),
            ],
        };

        let thread = Thread::new(root, "Missing".to_string());
        let message_ids: Vec<&str> = thread.iter().map(|a| a.message_id.as_str()).collect();

        // Should only contain the two real articles, not the placeholder
        assert_eq!(message_ids, vec!["<reply1@x.com>", "<reply2@x.com>"]);
    }

    #[test]
    fn test_thread_node_iterator_includes_placeholders() {
        // Node iterator should include placeholder nodes
        let root = ThreadNode {
            article: None, // Placeholder root
            message_id: "<missing@x.com>".to_string(),
            replies: vec![
                ThreadNode::new(make_article_ref(
                    "<reply1@x.com>",
                    "Re: Missing",
                    Some("<missing@x.com>"),
                )),
                ThreadNode::new(make_article_ref(
                    "<reply2@x.com>",
                    "Re: Missing",
                    Some("<missing@x.com>"),
                )),
            ],
        };

        let thread = Thread::new(root, "Missing".to_string());
        let message_ids: Vec<&str> = thread.iter_nodes().map(|n| n.message_id.as_str()).collect();

        // Should contain all three nodes including the placeholder
        assert_eq!(
            message_ids,
            vec!["<missing@x.com>", "<reply1@x.com>", "<reply2@x.com>"]
        );
    }

    #[test]
    fn test_thread_collection() {
        let thread1 = Thread::new(
            ThreadNode::new(make_article_ref("<t1@x.com>", "Thread 1", None)),
            "Thread 1".to_string(),
        );
        let thread2 = Thread::new(
            ThreadNode::new(make_article_ref("<t2@x.com>", "Thread 2", None)),
            "Thread 2".to_string(),
        );

        let collection = ThreadCollection::new("test.group".to_string(), vec![thread1, thread2]);

        assert_eq!(collection.len(), 2);
        assert_eq!(collection.total_articles(), 2);
        assert!(collection.find_thread_by_message_id("<t1@x.com>").is_some());
        assert!(collection
            .find_thread_by_message_id("<nonexistent@x.com>")
            .is_none());
    }

    #[test]
    fn test_threaded_article_ref_from_overview() {
        let overview = OverviewEntry {
            fields: vec![
                "100".to_string(),
                "Test Subject".to_string(),
                "test@example.com".to_string(),
                "Mon, 01 Jan 2024 00:00:00 +0000".to_string(),
                "<test@example.com>".to_string(),
                "<parent@example.com>".to_string(),
                "1000".to_string(),
                "50".to_string(),
            ],
        };

        let article_ref = ThreadedArticleRef::from_overview(&overview);
        assert!(article_ref.is_some());
        
        let article_ref = article_ref.unwrap();
        assert_eq!(article_ref.message_id, "<test@example.com>");
        assert_eq!(article_ref.number, Some(100));
        assert_eq!(article_ref.subject, "Test Subject");
        assert_eq!(article_ref.from, "test@example.com");
        assert!(article_ref.is_reply());
        assert_eq!(article_ref.parent_id, Some("<parent@example.com>".to_string()));
    }

    #[test]
    fn test_threaded_article_ref_from_overview_missing_fields() {
        // Overview entry missing required fields
        let overview = OverviewEntry {
            fields: vec!["100".to_string()], // Only number, missing subject, from, date, message-id
        };

        let article_ref = ThreadedArticleRef::from_overview(&overview);
        assert!(article_ref.is_none());
    }

    #[test]
    fn test_threaded_article_ref_depth() {
        // Root article has no references
        let root = make_article_ref("<root@x.com>", "Root", None);
        assert_eq!(root.depth(), 0);

        // First-level reply
        let reply1 = ThreadedArticleRef {
            message_id: "<reply1@x.com>".to_string(),
            number: Some(2),
            subject: "Re: Root".to_string(),
            from: "test@example.com".to_string(),
            date: "2024-01-01".to_string(),
            parent_id: Some("<root@x.com>".to_string()),
            references: vec!["<root@x.com>".to_string()],
            byte_count: None,
            line_count: None,
        };
        assert_eq!(reply1.depth(), 1);

        // Second-level reply
        let reply2 = ThreadedArticleRef {
            message_id: "<reply2@x.com>".to_string(),
            number: Some(3),
            subject: "Re: Re: Root".to_string(),
            from: "test@example.com".to_string(),
            date: "2024-01-01".to_string(),
            parent_id: Some("<reply1@x.com>".to_string()),
            references: vec!["<root@x.com>".to_string(), "<reply1@x.com>".to_string()],
            byte_count: None,
            line_count: None,
        };
        assert_eq!(reply2.depth(), 2);
    }

    #[test]
    fn test_thread_node_new_and_placeholder() {
        let article = make_article_ref("<test@x.com>", "Test", None);
        let node = ThreadNode::new(article);
        
        assert!(!node.is_placeholder());
        assert_eq!(node.reply_count(), 0);
        assert!(!node.has_replies());
        assert_eq!(node.message_id, "<test@x.com>");
        
        let placeholder = ThreadNode::placeholder("<missing@x.com>".to_string());
        assert!(placeholder.is_placeholder());
        assert_eq!(placeholder.message_id, "<missing@x.com>");
    }

    #[test]
    fn test_thread_all_message_ids() {
        let root = ThreadNode {
            article: Some(make_article_ref("<root@x.com>", "Hello", None)),
            message_id: "<root@x.com>".to_string(),
            replies: vec![
                ThreadNode::new(make_article_ref("<reply1@x.com>", "Re: Hello", Some("<root@x.com>"))),
                ThreadNode::new(make_article_ref("<reply2@x.com>", "Re: Hello", Some("<root@x.com>"))),
            ],
        };

        let thread = Thread::new(root, "Hello".to_string());
        let ids = thread.all_message_ids();
        
        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&"<root@x.com>"));
        assert!(ids.contains(&"<reply1@x.com>"));
        assert!(ids.contains(&"<reply2@x.com>"));
    }

    #[test]
    fn test_thread_find_by_message_id() {
        let root = ThreadNode {
            article: Some(make_article_ref("<root@x.com>", "Hello", None)),
            message_id: "<root@x.com>".to_string(),
            replies: vec![
                ThreadNode::new(make_article_ref("<reply@x.com>", "Re: Hello", Some("<root@x.com>"))),
            ],
        };

        let thread = Thread::new(root, "Hello".to_string());
        
        assert!(thread.find_by_message_id("<root@x.com>").is_some());
        assert!(thread.find_by_message_id("<reply@x.com>").is_some());
        assert!(thread.find_by_message_id("<missing@x.com>").is_none());
    }

    #[test]
    fn test_thread_has_placeholder_root() {
        // Thread with real root
        let real_root = ThreadNode::new(make_article_ref("<root@x.com>", "Hello", None));
        let thread1 = Thread::new(real_root, "Hello".to_string());
        assert!(!thread1.has_placeholder_root());

        // Thread with placeholder root
        let placeholder_root = ThreadNode::placeholder("<missing@x.com>".to_string());
        let thread2 = Thread::new(placeholder_root, "Missing".to_string());
        assert!(thread2.has_placeholder_root());
    }

    #[test]
    fn test_thread_properties() {
        let root = ThreadNode {
            article: Some(make_article_ref("<root@x.com>", "Hello World", None)),
            message_id: "<root@x.com>".to_string(),
            replies: vec![
                ThreadNode::new(make_article_ref("<reply@x.com>", "Re: Hello World", Some("<root@x.com>"))),
            ],
        };

        let thread = Thread::new(root, "Hello World".to_string());
        
        assert_eq!(thread.subject(), "Hello World");
        assert_eq!(thread.article_count(), 2);
        assert_eq!(thread.root_message_id(), "<root@x.com>");
        assert_eq!(thread.max_depth(), 1);
        assert!(thread.root().article.is_some());
    }

    #[test]
    fn test_thread_collection_empty() {
        let collection = ThreadCollection::new("test.group".to_string(), vec![]);
        
        assert!(collection.is_empty());
        assert_eq!(collection.len(), 0);
        assert_eq!(collection.total_articles(), 0);
    }

    #[test]
    fn test_thread_collection_into_iter() {
        let thread = Thread::new(
            ThreadNode::new(make_article_ref("<t1@x.com>", "Thread 1", None)),
            "Thread 1".to_string(),
        );

        let collection = ThreadCollection::new("test.group".to_string(), vec![thread]);
        
        // Test IntoIterator for owned collection
        let threads: Vec<Thread> = collection.into_iter().collect();
        assert_eq!(threads.len(), 1);
    }

    #[test]
    fn test_thread_collection_iter_ref() {
        let thread = Thread::new(
            ThreadNode::new(make_article_ref("<t1@x.com>", "Thread 1", None)),
            "Thread 1".to_string(),
        );

        let collection = ThreadCollection::new("test.group".to_string(), vec![thread]);
        
        // Test iter() method
        let count = collection.iter().count();
        assert_eq!(count, 1);
        
        // Test IntoIterator for reference
        let count2 = (&collection).into_iter().count();
        assert_eq!(count2, 1);
    }
}
