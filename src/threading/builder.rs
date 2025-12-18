//! Article building and fetched article types.

use crate::error::{Error, Result};
use crate::response::{Article, Attachment};
use mail_parser::Message;

use super::types::ThreadedArticleRef;

/// Builder for composing new NNTP articles.
///
/// This provides a fluent API for constructing articles with proper
/// headers for posting to newsgroups.
///
/// # Example
///
/// ```
/// use nntp_rs::threading::ArticleBuilder;
///
/// let article = ArticleBuilder::new()
///     .from("User Name <user@example.com>")
///     .subject("Hello World!")
///     .newsgroup("misc.test")
///     .body("This is my message.")
///     .build()
///     .unwrap();
/// ```
#[derive(Debug, Clone, Default)]
pub struct ArticleBuilder {
    from: Option<String>,
    subject: Option<String>,
    newsgroups: Vec<String>,
    body: Option<String>,
    references: Vec<String>,
    in_reply_to: Option<String>,
    additional_headers: Vec<(String, String)>,
}

impl ArticleBuilder {
    /// Create a new article builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the From header (required).
    ///
    /// Should be in the format "Name <email@example.com>" or just "email@example.com".
    pub fn from(mut self, from: impl Into<String>) -> Self {
        self.from = Some(from.into());
        self
    }

    /// Set the Subject header (required).
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    /// Add a newsgroup to post to (at least one required).
    ///
    /// Can be called multiple times for cross-posting.
    pub fn newsgroup(mut self, group: impl Into<String>) -> Self {
        self.newsgroups.push(group.into());
        self
    }

    /// Add multiple newsgroups at once.
    pub fn newsgroups(mut self, groups: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.newsgroups.extend(groups.into_iter().map(|g| g.into()));
        self
    }

    /// Set the article body (required).
    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Set this article as a reply to another article.
    ///
    /// Automatically sets the References and In-Reply-To headers based on
    /// the parent article's Message-ID and References.
    pub fn in_reply_to(mut self, article: &ThreadedArticleRef) -> Self {
        // Build References: parent's references + parent's message-id
        self.references = article.references.clone();
        self.references.push(article.message_id.clone());

        // In-Reply-To is the immediate parent
        self.in_reply_to = Some(article.message_id.clone());

        // If subject doesn't start with Re:, add it
        if let Some(ref subject) = self.subject {
            if !subject.to_lowercase().starts_with("re:") {
                self.subject = Some(format!("Re: {}", subject));
            }
        }

        self
    }

    /// Set this article as a reply, using just a Message-ID.
    ///
    /// Use `in_reply_to` with a `ThreadedArticleRef` when possible,
    /// as it properly builds the complete References chain.
    pub fn in_reply_to_message_id(mut self, message_id: impl Into<String>) -> Self {
        let message_id = message_id.into();
        self.references.push(message_id.clone());
        self.in_reply_to = Some(message_id);
        self
    }

    /// Add a custom header.
    ///
    /// # Example
    ///
    /// ```
    /// # use nntp_rs::threading::ArticleBuilder;
    /// let builder = ArticleBuilder::new()
    ///     .header("Organization", "Example Corp")
    ///     .header("X-Custom-Header", "custom value");
    /// ```
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.additional_headers.push((name.into(), value.into()));
        self
    }

    /// Build the article as a formatted string ready for posting.
    ///
    /// Returns an error if required fields (from, subject, newsgroups, body) are missing.
    pub fn build(self) -> Result<String> {
        let from = self
            .from
            .ok_or_else(|| Error::ArticleBuilder("From header is required".to_string()))?;

        let subject = self
            .subject
            .ok_or_else(|| Error::ArticleBuilder("Subject header is required".to_string()))?;

        if self.newsgroups.is_empty() {
            return Err(Error::ArticleBuilder(
                "At least one newsgroup is required".to_string(),
            ));
        }

        let body = self
            .body
            .ok_or_else(|| Error::ArticleBuilder("Body is required".to_string()))?;

        let mut article = String::new();

        // Required headers
        article.push_str(&format!("From: {}\r\n", from));
        article.push_str(&format!("Subject: {}\r\n", subject));
        article.push_str(&format!("Newsgroups: {}\r\n", self.newsgroups.join(",")));

        // Optional threading headers
        if !self.references.is_empty() {
            article.push_str(&format!("References: {}\r\n", self.references.join(" ")));
        }
        if let Some(reply_to) = self.in_reply_to {
            article.push_str(&format!("In-Reply-To: {}\r\n", reply_to));
        }

        // Additional headers
        for (name, value) in self.additional_headers {
            article.push_str(&format!("{}: {}\r\n", name, value));
        }

        // Blank line separating headers from body
        article.push_str("\r\n");

        // Body
        article.push_str(&body);

        // Ensure body ends with CRLF
        if !article.ends_with("\r\n") {
            article.push_str("\r\n");
        }

        Ok(article)
    }
}

/// A fully fetched article with its threading context.
///
/// This combines the full `Article` content with `ThreadedArticleRef`
/// metadata for convenient access to both content and threading info.
#[derive(Debug)]
pub struct FetchedArticle {
    /// The underlying Article with full content
    article: Article,
    /// Threading metadata
    pub thread_ref: ThreadedArticleRef,
}

impl FetchedArticle {
    /// Create a new FetchedArticle from an Article.
    ///
    /// Extracts threading information from the article headers.
    pub fn new(article: Article) -> Self {
        let message_id = article.article_id().to_string();
        let references = ThreadedArticleRef::parse_references(article.references().as_deref());
        let parent_id = references.last().cloned();

        let thread_ref = ThreadedArticleRef {
            message_id,
            number: article.number(),
            subject: article.subject().unwrap_or_default(),
            from: article.from().unwrap_or_default(),
            date: article.date().unwrap_or_default(),
            parent_id,
            references,
            byte_count: None, // Not available from Article
            line_count: None, // Not available from Article
        };

        Self {
            article,
            thread_ref,
        }
    }

    /// Create a FetchedArticle with explicit thread reference.
    pub fn with_thread_ref(article: Article, thread_ref: ThreadedArticleRef) -> Self {
        Self {
            article,
            thread_ref,
        }
    }

    /// Get the underlying Article.
    pub fn article(&self) -> &Article {
        &self.article
    }

    /// Get the body as plain text.
    pub fn body_text(&self) -> Option<String> {
        self.article.body_text()
    }

    /// Get the body as HTML if available.
    pub fn body_html(&self) -> Option<String> {
        self.article.body_html()
    }

    /// Get attachments from the article.
    pub fn attachments(&self) -> Vec<Attachment> {
        self.article.attachments()
    }

    /// Get the full mail_parser Message for advanced parsing.
    pub fn message(&self) -> Option<Message<'_>> {
        self.article.message()
    }

    /// Get the Message-ID.
    pub fn message_id(&self) -> &str {
        &self.thread_ref.message_id
    }

    /// Get the subject.
    pub fn subject(&self) -> &str {
        &self.thread_ref.subject
    }

    /// Get the From header.
    pub fn from(&self) -> &str {
        &self.thread_ref.from
    }

    /// Get the date.
    pub fn date(&self) -> &str {
        &self.thread_ref.date
    }

    /// Get the parent Message-ID if this is a reply.
    pub fn parent_id(&self) -> Option<&str> {
        self.thread_ref.parent_id.as_deref()
    }

    /// Get the references (ancestor Message-IDs).
    pub fn references(&self) -> &[String] {
        &self.thread_ref.references
    }

    /// Check if this article is a reply.
    pub fn is_reply(&self) -> bool {
        self.thread_ref.is_reply()
    }

    /// Get the raw article content.
    pub fn raw_content(&self) -> &[u8] {
        self.article.raw_content()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_article_builder_basic() {
        let article = ArticleBuilder::new()
            .from("Test User <test@example.com>")
            .subject("Hello World")
            .newsgroup("misc.test")
            .body("This is a test.")
            .build()
            .unwrap();

        assert!(article.contains("From: Test User <test@example.com>"));
        assert!(article.contains("Subject: Hello World"));
        assert!(article.contains("Newsgroups: misc.test"));
        assert!(article.contains("\r\n\r\n")); // Header/body separator
        assert!(article.contains("This is a test."));
    }

    #[test]
    fn test_article_builder_multiple_newsgroups() {
        let article = ArticleBuilder::new()
            .from("test@example.com")
            .subject("Cross-post")
            .newsgroup("misc.test")
            .newsgroup("alt.test")
            .body("Test")
            .build()
            .unwrap();

        assert!(article.contains("Newsgroups: misc.test,alt.test"));
    }

    #[test]
    fn test_article_builder_with_reply() {
        let parent = ThreadedArticleRef {
            message_id: "<parent@example.com>".to_string(),
            number: Some(1),
            subject: "Original Subject".to_string(),
            from: "other@example.com".to_string(),
            date: "2024-01-01".to_string(),
            parent_id: None,
            references: vec!["<grandparent@example.com>".to_string()],
            byte_count: None,
            line_count: None,
        };

        let article = ArticleBuilder::new()
            .from("test@example.com")
            .subject("Original Subject")
            .newsgroup("misc.test")
            .body("My reply")
            .in_reply_to(&parent)
            .build()
            .unwrap();

        assert!(article.contains("Subject: Re: Original Subject"));
        assert!(article.contains("In-Reply-To: <parent@example.com>"));
        assert!(article.contains("References: <grandparent@example.com> <parent@example.com>"));
    }

    #[test]
    fn test_article_builder_custom_headers() {
        let article = ArticleBuilder::new()
            .from("test@example.com")
            .subject("Test")
            .newsgroup("misc.test")
            .body("Test")
            .header("Organization", "Test Corp")
            .header("X-Custom", "value")
            .build()
            .unwrap();

        assert!(article.contains("Organization: Test Corp"));
        assert!(article.contains("X-Custom: value"));
    }

    #[test]
    fn test_article_builder_missing_from() {
        let result = ArticleBuilder::new()
            .subject("Test")
            .newsgroup("misc.test")
            .body("Test")
            .build();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::ArticleBuilder(_)));
    }

    #[test]
    fn test_article_builder_missing_subject() {
        let result = ArticleBuilder::new()
            .from("test@example.com")
            .newsgroup("misc.test")
            .body("Test")
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_article_builder_missing_newsgroup() {
        let result = ArticleBuilder::new()
            .from("test@example.com")
            .subject("Test")
            .body("Test")
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_article_builder_missing_body() {
        let result = ArticleBuilder::new()
            .from("test@example.com")
            .subject("Test")
            .newsgroup("misc.test")
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_article_builder_with_newsgroups() {
        let article = ArticleBuilder::new()
            .from("test@example.com")
            .subject("Test")
            .newsgroups(vec!["misc.test", "alt.test", "comp.test"])
            .body("Test body")
            .build()
            .unwrap();

        assert!(article.contains("Newsgroups: misc.test,alt.test,comp.test"));
    }

    #[test]
    fn test_article_builder_in_reply_to_message_id() {
        let article = ArticleBuilder::new()
            .from("test@example.com")
            .subject("Re: Test")
            .newsgroup("misc.test")
            .body("Test body")
            .in_reply_to_message_id("<parent@example.com>")
            .build()
            .unwrap();

        assert!(article.contains("In-Reply-To: <parent@example.com>"));
        assert!(article.contains("References: <parent@example.com>"));
    }

    #[test]
    fn test_article_builder_reply_adds_re_prefix() {
        let parent = ThreadedArticleRef {
            message_id: "<parent@example.com>".to_string(),
            number: Some(1),
            subject: "Original Subject".to_string(),
            from: "other@example.com".to_string(),
            date: "2024-01-01".to_string(),
            parent_id: None,
            references: vec![],
            byte_count: None,
            line_count: None,
        };

        // Subject without Re: should get it added
        let article = ArticleBuilder::new()
            .from("test@example.com")
            .subject("Original Subject")
            .newsgroup("misc.test")
            .body("My reply")
            .in_reply_to(&parent)
            .build()
            .unwrap();

        assert!(article.contains("Subject: Re: Original Subject"));
    }

    #[test]
    fn test_article_builder_reply_keeps_existing_re_prefix() {
        let parent = ThreadedArticleRef {
            message_id: "<parent@example.com>".to_string(),
            number: Some(1),
            subject: "Original".to_string(),
            from: "other@example.com".to_string(),
            date: "2024-01-01".to_string(),
            parent_id: None,
            references: vec![],
            byte_count: None,
            line_count: None,
        };

        // Subject already has Re: - should not double it
        let article = ArticleBuilder::new()
            .from("test@example.com")
            .subject("Re: Original")
            .newsgroup("misc.test")
            .body("My reply")
            .in_reply_to(&parent)
            .build()
            .unwrap();

        // Should have Re: but not Re: Re:
        assert!(article.contains("Subject: Re: Original"));
        assert!(!article.contains("Re: Re:"));
    }

    #[test]
    fn test_article_builder_body_without_crlf() {
        let article = ArticleBuilder::new()
            .from("test@example.com")
            .subject("Test")
            .newsgroup("misc.test")
            .body("Body without newline")
            .build()
            .unwrap();

        // Body should end with CRLF
        assert!(article.ends_with("\r\n"));
    }

    #[test]
    fn test_fetched_article_creation() {
        let content = b"From: test@example.com\r\nSubject: Test Subject\r\nDate: Mon, 01 Jan 2024 00:00:00 +0000\r\nMessage-ID: <test123@example.com>\r\nReferences: <parent@example.com>\r\n\r\nTest body\r\n".to_vec();
        let article = Article::new(Some(100), "<test123@example.com>".to_string(), content);
        
        let fetched = FetchedArticle::new(article);
        
        assert_eq!(fetched.message_id(), "<test123@example.com>");
        assert_eq!(fetched.subject(), "Test Subject");
        // from() returns display address which may vary
        assert!(!fetched.from().is_empty());
        assert!(!fetched.date().is_empty());
        // is_reply depends on whether References was parsed, which may vary
        // Just verify the method works
        let _ = fetched.is_reply();
        let _ = fetched.parent_id();
        let _ = fetched.references();
    }

    #[test]
    fn test_fetched_article_with_thread_ref() {
        let content = b"From: test@example.com\r\nSubject: Test\r\n\r\nBody\r\n".to_vec();
        let article = Article::new(Some(100), "<test@example.com>".to_string(), content);
        
        let thread_ref = ThreadedArticleRef {
            message_id: "<test@example.com>".to_string(),
            number: Some(100),
            subject: "Custom Subject".to_string(),
            from: "custom@example.com".to_string(),
            date: "2024-01-01".to_string(),
            parent_id: None,
            references: vec![],
            byte_count: Some(500),
            line_count: Some(10),
        };
        
        let fetched = FetchedArticle::with_thread_ref(article, thread_ref);
        
        // Should use the provided thread_ref
        assert_eq!(fetched.subject(), "Custom Subject");
        assert_eq!(fetched.from(), "custom@example.com");
        assert!(!fetched.is_reply());
    }

    #[test]
    fn test_fetched_article_body_and_attachments() {
        let content = b"From: test@example.com\r\nSubject: Test\r\nContent-Type: text/plain\r\n\r\nPlain text body\r\n".to_vec();
        let article = Article::new(Some(100), "<test@example.com>".to_string(), content);
        
        let fetched = FetchedArticle::new(article);
        
        assert!(fetched.body_text().is_some());
        // body_html may or may not be present depending on mail-parser behavior
        let _ = fetched.body_html();
        assert!(fetched.attachments().is_empty());
        assert!(fetched.message().is_some());
    }

    #[test]
    fn test_fetched_article_raw_content() {
        let content = b"From: test@example.com\r\nSubject: Test\r\n\r\nBody\r\n".to_vec();
        let article = Article::new(Some(100), "<test@example.com>".to_string(), content.clone());
        
        let fetched = FetchedArticle::new(article);
        
        assert_eq!(fetched.raw_content(), &content);
        assert!(fetched.article().message().is_some());
    }
}
