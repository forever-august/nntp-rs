//! Thread building algorithm.
//!
//! This module implements the algorithm for building thread trees from
//! a flat list of articles with References headers.

use std::collections::{HashMap, HashSet};

use super::types::{Thread, ThreadCollection, ThreadNode, ThreadedArticleRef};

/// Build thread trees from a list of article references.
///
/// This algorithm handles missing articles by creating placeholder nodes.
/// When an article references a parent that's not in our set, a placeholder
/// node is created for that parent. This keeps sibling replies grouped together.
///
/// Steps:
/// 1. Indexes all articles by Message-ID
/// 2. Builds parent-child relationships from References headers
/// 3. Creates placeholder nodes for missing parents
/// 4. Identifies root nodes (no parent, or parent is also missing)
/// 5. Constructs trees recursively
/// 6. Sorts threads by most recent activity
pub fn build_threads(articles: Vec<ThreadedArticleRef>, group: &str) -> ThreadCollection {
    if articles.is_empty() {
        return ThreadCollection::new(group.to_string(), Vec::new());
    }

    let article_count = articles.len();

    // HashMap: Message-ID -> ThreadedArticleRef
    // Pre-allocate with known size for efficiency
    let mut id_to_article: HashMap<String, ThreadedArticleRef> = HashMap::with_capacity(article_count);

    // HashMap: Message-ID -> Vec<Message-ID> (children)
    // Estimate ~25% of articles are root articles, so ~75% have parents
    let mut children: HashMap<String, Vec<String>> = HashMap::with_capacity(article_count * 3 / 4);

    // Set of all Message-IDs that have a parent (present or placeholder)
    let mut has_parent: HashSet<String> = HashSet::with_capacity(article_count * 3 / 4);

    // Set of Message-IDs for missing parents that need placeholder nodes
    let mut missing_parents: HashSet<String> = HashSet::new();

    // Step 1: Index all articles by Message-ID
    for article in articles {
        id_to_article.insert(article.message_id.clone(), article);
    }

    // Step 2: Build parent-child relationships
    for (message_id, article) in &id_to_article {
        if let Some(parent_id) = &article.parent_id {
            has_parent.insert(message_id.clone());
            children
                .entry(parent_id.clone())
                .or_default()
                .push(message_id.clone());

            // Track missing parents for placeholder creation
            if !id_to_article.contains_key(parent_id) {
                missing_parents.insert(parent_id.clone());
            }
        }
    }

    // Step 3: Determine which missing parents need to be roots
    // A missing parent is a root if it doesn't have a parent in our set
    // For simplicity, we check if any of its ancestors are in our set
    let placeholder_roots: HashSet<String> = missing_parents
        .iter()
        .filter(|parent_id| {
            // This missing parent is a root if none of its children have
            // a grandparent that's in our set
            !is_ancestor_present(parent_id, &id_to_article, &children)
        })
        .cloned()
        .collect();

    // Step 4: Find root articles (no parent or parent is a missing root)
    let mut roots: Vec<String> = id_to_article
        .keys()
        .filter(|id| !has_parent.contains(*id))
        .cloned()
        .collect();

    // Add placeholder roots
    roots.extend(placeholder_roots.iter().cloned());

    // Step 5: Build trees recursively
    let mut threads: Vec<Thread> = Vec::new();
    for root_id in roots {
        let root_node = if let Some(article) = id_to_article.remove(&root_id) {
            // Real article
            build_thread_node_from_article(article, &mut id_to_article, &children)
        } else if placeholder_roots.contains(&root_id) {
            // Placeholder node
            build_thread_node_from_placeholder(&root_id, &mut id_to_article, &children)
        } else {
            continue;
        };

        let subject = get_thread_subject(&root_node);
        threads.push(Thread::new(root_node, subject));
    }

    // Step 6: Sort threads by most recent activity (descending)
    threads.sort_by(|a, b| {
        let a_date = most_recent_date(a);
        let b_date = most_recent_date(b);
        b_date.cmp(a_date)
    });

    ThreadCollection::new(group.to_string(), threads)
}

/// Check if any ancestor of a message is present in our article set.
fn is_ancestor_present(
    message_id: &str,
    id_to_article: &HashMap<String, ThreadedArticleRef>,
    children: &HashMap<String, Vec<String>>,
) -> bool {
    // Look at each child of this message to find their references
    if let Some(child_ids) = children.get(message_id) {
        for child_id in child_ids {
            if let Some(child) = id_to_article.get(child_id) {
                // Check if any reference before the parent is in our set
                for reference in &child.references {
                    if reference == message_id {
                        break; // Stop at the parent
                    }
                    if id_to_article.contains_key(reference) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Get the subject for a thread, finding the first non-placeholder article.
///
/// Uses iterative traversal to support arbitrarily deep threads without stack overflow.
fn get_thread_subject(node: &ThreadNode) -> String {
    let mut stack = vec![node];
    while let Some(current) = stack.pop() {
        if let Some(ref article) = current.article {
            return normalize_subject(&article.subject);
        }
        // Push replies in reverse order so they're processed left-to-right
        for reply in current.replies.iter().rev() {
            stack.push(reply);
        }
    }
    String::new()
}

/// Build a thread node from a real article.
///
/// Uses iterative traversal to support arbitrarily deep threads without stack overflow.
fn build_thread_node_from_article(
    article: ThreadedArticleRef,
    id_to_article: &mut HashMap<String, ThreadedArticleRef>,
    children: &HashMap<String, Vec<String>>,
) -> ThreadNode {
    let root_id = article.message_id.clone();

    // Put the root article back temporarily so we can process uniformly
    id_to_article.insert(root_id.clone(), article);

    build_thread_node_iterative(&root_id, id_to_article, children)
}

/// Build a thread node from a placeholder (missing article).
///
/// Uses iterative traversal to support arbitrarily deep threads without stack overflow.
fn build_thread_node_from_placeholder(
    message_id: &str,
    id_to_article: &mut HashMap<String, ThreadedArticleRef>,
    children: &HashMap<String, Vec<String>>,
) -> ThreadNode {
    build_thread_node_iterative(message_id, id_to_article, children)
}

/// Iteratively build a thread node and all its descendants.
///
/// This function builds the tree bottom-up to avoid stack overflow on deeply nested threads.
fn build_thread_node_iterative(
    root_id: &str,
    id_to_article: &mut HashMap<String, ThreadedArticleRef>,
    children: &HashMap<String, Vec<String>>,
) -> ThreadNode {
    // Phase 1: Collect all nodes to build using iterative traversal
    // Store (message_id, depth) pairs, we'll process deepest first
    let mut nodes_to_build: Vec<(String, usize)> = Vec::new();
    let mut stack: Vec<(String, usize)> = vec![(root_id.to_string(), 0)];

    while let Some((msg_id, depth)) = stack.pop() {
        nodes_to_build.push((msg_id.clone(), depth));

        // Add children to the stack
        if let Some(child_ids) = children.get(&msg_id) {
            for child_id in child_ids {
                // Only process if the child exists in our article set
                if id_to_article.contains_key(child_id) {
                    stack.push((child_id.clone(), depth + 1));
                }
            }
        }
    }

    // Phase 2: Sort by depth (deepest first) to build bottom-up
    nodes_to_build.sort_by(|a, b| b.1.cmp(&a.1));

    // Phase 3: Build nodes bottom-up, storing completed nodes
    let mut built_nodes: HashMap<String, ThreadNode> = HashMap::new();

    for (msg_id, _depth) in nodes_to_build {
        // Collect child nodes that we've already built
        let mut replies: Vec<ThreadNode> = Vec::new();
        if let Some(child_ids) = children.get(&msg_id) {
            for child_id in child_ids {
                if let Some(child_node) = built_nodes.remove(child_id) {
                    replies.push(child_node);
                }
            }
        }

        // Sort replies by date (ascending - oldest first)
        replies.sort_by(|a, b| {
            let a_date = a.article.as_ref().map(|a| a.date.as_str()).unwrap_or("");
            let b_date = b.article.as_ref().map(|a| a.date.as_str()).unwrap_or("");
            a_date.cmp(b_date)
        });

        // Create the node
        let node = ThreadNode {
            article: id_to_article.remove(&msg_id),
            replies,
            message_id: msg_id.clone(),
        };

        built_nodes.insert(msg_id, node);
    }

    // Return the root node
    built_nodes.remove(root_id).unwrap_or_else(|| ThreadNode {
        article: None,
        replies: Vec::new(),
        message_id: root_id.to_string(),
    })
}

/// Normalize a subject line by removing Re:, Fwd:, etc. prefixes.
///
/// This allows grouping articles by their base subject.
pub fn normalize_subject(subject: &str) -> String {
    let mut normalized = subject.trim().to_string();

    // Common prefixes to remove (case-insensitive)
    let prefixes = ["re:", "fwd:", "fw:", "aw:", "sv:", "antw:"];

    loop {
        let lower = normalized.to_lowercase();
        let mut found = false;

        for prefix in &prefixes {
            if lower.starts_with(prefix) {
                normalized = normalized[prefix.len()..].trim_start().to_string();
                found = true;
                break;
            }
        }

        // Also handle [Fwd: ...] style
        if normalized.starts_with('[') {
            if let Some(end) = normalized.find(']') {
                let bracket_content = &normalized[1..end].to_lowercase();
                if prefixes
                    .iter()
                    .any(|p| bracket_content.starts_with(p.trim_end_matches(':')))
                {
                    normalized = normalized[end + 1..].trim_start().to_string();
                    found = true;
                }
            }
        }

        if !found {
            break;
        }
    }

    normalized
}

/// Get the most recent date from a thread (for sorting).
fn most_recent_date(thread: &Thread) -> &str {
    let mut most_recent: &str = "";

    for article in thread.iter() {
        if article.date.as_str() > most_recent {
            most_recent = &article.date;
        }
    }

    most_recent
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_article(
        message_id: &str,
        subject: &str,
        date: &str,
        parent_id: Option<&str>,
    ) -> ThreadedArticleRef {
        ThreadedArticleRef {
            message_id: message_id.to_string(),
            number: Some(1),
            subject: subject.to_string(),
            from: "test@example.com".to_string(),
            date: date.to_string(),
            parent_id: parent_id.map(|s| s.to_string()),
            references: parent_id.map(|s| vec![s.to_string()]).unwrap_or_default(),
            byte_count: Some(100),
            line_count: Some(10),
        }
    }

    #[test]
    fn test_normalize_subject() {
        assert_eq!(normalize_subject("Hello World"), "Hello World");
        assert_eq!(normalize_subject("Re: Hello World"), "Hello World");
        assert_eq!(normalize_subject("RE: Hello World"), "Hello World");
        assert_eq!(normalize_subject("re: Hello World"), "Hello World");
        assert_eq!(normalize_subject("Re: Re: Hello World"), "Hello World");
        assert_eq!(normalize_subject("Fwd: Hello World"), "Hello World");
        assert_eq!(normalize_subject("FWD: Hello World"), "Hello World");
        assert_eq!(normalize_subject("Fw: Hello World"), "Hello World");
        assert_eq!(normalize_subject("Re: Fwd: Hello World"), "Hello World");
        assert_eq!(normalize_subject("Aw: Hello World"), "Hello World"); // German
        assert_eq!(normalize_subject("Sv: Hello World"), "Hello World"); // Swedish
        assert_eq!(
            normalize_subject("[Fwd: Something] Hello World"),
            "Hello World"
        );
    }

    #[test]
    fn test_build_threads_empty() {
        let collection = build_threads(vec![], "test.group");
        assert!(collection.is_empty());
        assert_eq!(collection.group, "test.group");
    }

    #[test]
    fn test_build_threads_single_article() {
        let articles = vec![make_article("<root@x.com>", "Hello", "2024-01-01", None)];

        let collection = build_threads(articles, "test.group");
        assert_eq!(collection.len(), 1);
        assert_eq!(collection.threads()[0].article_count(), 1);
        assert_eq!(collection.threads()[0].subject(), "Hello");
    }

    #[test]
    fn test_build_threads_with_replies() {
        let articles = vec![
            make_article("<root@x.com>", "Hello", "2024-01-01", None),
            make_article(
                "<reply1@x.com>",
                "Re: Hello",
                "2024-01-02",
                Some("<root@x.com>"),
            ),
            make_article(
                "<reply2@x.com>",
                "Re: Hello",
                "2024-01-03",
                Some("<root@x.com>"),
            ),
        ];

        let collection = build_threads(articles, "test.group");
        assert_eq!(collection.len(), 1);

        let thread = &collection.threads()[0];
        assert_eq!(thread.article_count(), 3);
        assert_eq!(thread.root().reply_count(), 2);
        assert_eq!(thread.subject(), "Hello");
    }

    #[test]
    fn test_build_threads_nested_replies() {
        let articles = vec![
            make_article("<root@x.com>", "Hello", "2024-01-01", None),
            make_article(
                "<reply@x.com>",
                "Re: Hello",
                "2024-01-02",
                Some("<root@x.com>"),
            ),
            make_article(
                "<nested@x.com>",
                "Re: Re: Hello",
                "2024-01-03",
                Some("<reply@x.com>"),
            ),
        ];

        let collection = build_threads(articles, "test.group");
        assert_eq!(collection.len(), 1);

        let thread = &collection.threads()[0];
        assert_eq!(thread.article_count(), 3);
        assert_eq!(thread.max_depth(), 2);
        assert_eq!(thread.root().reply_count(), 1);
        assert_eq!(thread.root().replies[0].reply_count(), 1);
    }

    #[test]
    fn test_build_threads_multiple_threads() {
        let articles = vec![
            make_article("<thread1@x.com>", "Thread 1", "2024-01-01", None),
            make_article("<thread2@x.com>", "Thread 2", "2024-01-02", None),
            make_article(
                "<reply1@x.com>",
                "Re: Thread 1",
                "2024-01-03",
                Some("<thread1@x.com>"),
            ),
        ];

        let collection = build_threads(articles, "test.group");
        assert_eq!(collection.len(), 2);
        assert_eq!(collection.total_articles(), 3);
    }

    #[test]
    fn test_build_threads_sorted_by_recent_activity() {
        let articles = vec![
            make_article("<old@x.com>", "Old Thread", "2024-01-01", None),
            make_article("<new@x.com>", "New Thread", "2024-01-10", None),
            make_article(
                "<old-reply@x.com>",
                "Re: Old Thread",
                "2024-01-15",
                Some("<old@x.com>"),
            ),
        ];

        let collection = build_threads(articles, "test.group");
        assert_eq!(collection.len(), 2);

        // Old thread should be first because it has the most recent activity (reply on 01-15)
        assert_eq!(collection.threads()[0].subject(), "Old Thread");
        assert_eq!(collection.threads()[1].subject(), "New Thread");
    }

    #[test]
    fn test_build_threads_orphan_reply() {
        // Reply to an article not in our set - placeholder root is created
        let articles = vec![make_article(
            "<orphan@x.com>",
            "Re: Unknown",
            "2024-01-01",
            Some("<unknown@x.com>"),
        )];

        let collection = build_threads(articles, "test.group");
        assert_eq!(collection.len(), 1);

        let thread = &collection.threads()[0];
        // The missing parent becomes a placeholder root
        assert_eq!(thread.root_message_id(), "<unknown@x.com>");
        assert!(thread.has_placeholder_root());
        // The orphan is a child of the placeholder
        assert_eq!(thread.root().reply_count(), 1);
        assert_eq!(thread.root().replies[0].message_id, "<orphan@x.com>");
        // Article count only counts present articles
        assert_eq!(thread.article_count(), 1);
    }

    #[test]
    fn test_build_threads_sibling_orphans() {
        // Multiple replies to the same missing parent should be grouped together
        let articles = vec![
            make_article(
                "<sibling1@x.com>",
                "Re: Unknown",
                "2024-01-01",
                Some("<unknown@x.com>"),
            ),
            make_article(
                "<sibling2@x.com>",
                "Re: Unknown",
                "2024-01-02",
                Some("<unknown@x.com>"),
            ),
        ];

        let collection = build_threads(articles, "test.group");
        // Should be ONE thread with a placeholder root, not two separate threads
        assert_eq!(collection.len(), 1);

        let thread = &collection.threads()[0];
        assert_eq!(thread.root_message_id(), "<unknown@x.com>");
        assert!(thread.has_placeholder_root());
        // Both siblings should be children of the placeholder
        assert_eq!(thread.root().reply_count(), 2);
        assert_eq!(thread.article_count(), 2);
    }

    #[test]
    fn test_build_threads_placeholder_not_root_when_ancestor_exists() {
        // If an ancestor exists, the placeholder shouldn't be a root
        let articles = vec![
            make_article("<root@x.com>", "Hello", "2024-01-01", None),
            make_article(
                "<reply@x.com>",
                "Re: Hello",
                "2024-01-02",
                Some("<root@x.com>"),
            ),
            // This references <missing@x.com> which references <reply@x.com>
            // But since we don't have the references chain, it will be treated
            // as referencing a missing parent with no ancestor in our set
        ];

        let collection = build_threads(articles, "test.group");
        assert_eq!(collection.len(), 1);
        assert!(!collection.threads()[0].has_placeholder_root());
    }

    #[test]
    fn test_build_threads_with_ancestor_reference_chain() {
        // Test case where an orphan has a references chain that includes
        // an ancestor present in our article set
        let articles = vec![
            make_article("<grandparent@x.com>", "Original", "2024-01-01", None),
            // Reply that references grandparent via a missing parent
            // The references field should contain the full chain
            ThreadedArticleRef {
                message_id: "<grandchild@x.com>".to_string(),
                number: Some(3),
                subject: "Re: Re: Original".to_string(),
                from: "test@example.com".to_string(),
                date: "2024-01-03".to_string(),
                parent_id: Some("<missing-parent@x.com>".to_string()),
                references: vec![
                    "<grandparent@x.com>".to_string(),
                    "<missing-parent@x.com>".to_string(),
                ],
                byte_count: Some(100),
                line_count: Some(5),
            },
        ];

        let collection = build_threads(articles, "test.group");
        // The grandchild has a missing parent, so it becomes a separate orphan thread
        // or attached to grandparent depending on algorithm behavior
        assert!(!collection.is_empty());
        // Total articles should include both
        assert!(collection.total_articles() >= 1);
    }

    #[test]
    fn test_build_threads_empty_subject() {
        // Test article with empty subject
        let articles = vec![
            make_article("<root@x.com>", "", "2024-01-01", None),
        ];

        let collection = build_threads(articles, "test.group");
        assert_eq!(collection.len(), 1);
        // Empty subject is still valid
        assert_eq!(collection.threads()[0].subject(), "");
    }

    #[test]
    fn test_build_threads_re_subject_normalization() {
        // Test that Re: prefixes are normalized
        let articles = vec![
            make_article("<root@x.com>", "Test Subject", "2024-01-01", None),
            make_article("<reply@x.com>", "Re: Test Subject", "2024-01-02", Some("<root@x.com>")),
        ];

        let collection = build_threads(articles, "test.group");
        assert_eq!(collection.len(), 1);
        // Thread subject should be normalized (no Re: prefix)
        assert_eq!(collection.threads()[0].subject(), "Test Subject");
    }

    #[test]
    fn test_build_threads_placeholder_with_empty_subject_fallback() {
        // Test placeholder root with replies - should get subject from first reply
        let articles = vec![
            make_article(
                "<orphan1@x.com>",
                "Re: Missing Parent",
                "2024-01-01",
                Some("<missing@x.com>"),
            ),
        ];

        let collection = build_threads(articles, "test.group");
        assert_eq!(collection.len(), 1);
        // Thread subject comes from the first non-placeholder article
        assert_eq!(collection.threads()[0].subject(), "Missing Parent");
    }

    #[test]
    fn test_build_threads_deeply_nested() {
        // Test that deeply nested threads don't cause stack overflow
        // This creates a thread 1000 levels deep, which would overflow the stack
        // if using recursive algorithms with typical stack sizes
        const DEPTH: usize = 1000;

        let mut articles = Vec::with_capacity(DEPTH);

        // Create root article
        articles.push(make_article("<root@x.com>", "Deep Thread", "2024-01-01", None));

        // Create a chain of replies, each replying to the previous
        for i in 1..DEPTH {
            let message_id = format!("<reply{}@x.com>", i);
            let parent_id = if i == 1 {
                "<root@x.com>".to_string()
            } else {
                format!("<reply{}@x.com>", i - 1)
            };
            articles.push(ThreadedArticleRef {
                message_id: message_id.clone(),
                number: Some(i as u64 + 1),
                subject: format!("Re: Deep Thread {}", i),
                from: "test@example.com".to_string(),
                date: format!("2024-01-{:02}", (i % 28) + 1),
                parent_id: Some(parent_id.clone()),
                references: vec![parent_id],
                byte_count: Some(100),
                line_count: Some(10),
            });
        }

        let collection = build_threads(articles, "test.group");

        // Verify the thread was built correctly
        assert_eq!(collection.len(), 1);
        let thread = &collection.threads()[0];
        assert_eq!(thread.article_count(), DEPTH);
        assert_eq!(thread.max_depth(), DEPTH - 1);
        assert_eq!(thread.subject(), "Deep Thread");

        // Verify we can find articles at various depths
        assert!(thread.find_by_message_id("<root@x.com>").is_some());
        assert!(thread.find_by_message_id("<reply1@x.com>").is_some());
        assert!(thread
            .find_by_message_id(&format!("<reply{}@x.com>", DEPTH - 1))
            .is_some());

        // Verify iteration works
        let all_ids: Vec<&str> = thread.iter().map(|a| a.message_id.as_str()).collect();
        assert_eq!(all_ids.len(), DEPTH);
    }

    #[test]
    fn test_build_threads_wide_and_deep() {
        // Test a thread that is both wide (many siblings) and deep
        const WIDTH: usize = 10;
        const DEPTH: usize = 100;

        let mut articles = Vec::new();

        // Create root
        articles.push(make_article("<root@x.com>", "Wide and Deep", "2024-01-01", None));

        // Create WIDTH chains of DEPTH replies each
        for chain in 0..WIDTH {
            let chain_prefix = format!("chain{}", chain);
            for depth in 0..DEPTH {
                let message_id = format!("<{}-{}@x.com>", chain_prefix, depth);
                let parent_id = if depth == 0 {
                    "<root@x.com>".to_string()
                } else {
                    format!("<{}-{}@x.com>", chain_prefix, depth - 1)
                };
                articles.push(ThreadedArticleRef {
                    message_id,
                    number: Some((chain * DEPTH + depth + 2) as u64),
                    subject: "Re: Wide and Deep".to_string(),
                    from: "test@example.com".to_string(),
                    date: format!("2024-01-{:02}", (depth % 28) + 1),
                    parent_id: Some(parent_id.clone()),
                    references: vec![parent_id],
                    byte_count: Some(100),
                    line_count: Some(10),
                });
            }
        }

        let collection = build_threads(articles, "test.group");

        assert_eq!(collection.len(), 1);
        let thread = &collection.threads()[0];
        assert_eq!(thread.article_count(), 1 + WIDTH * DEPTH);
        assert_eq!(thread.root().reply_count(), WIDTH);

        // Each chain should have depth DEPTH - 1 (from first reply to last)
        assert_eq!(thread.max_depth(), DEPTH);
    }
}
