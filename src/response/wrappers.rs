//! Newtype wrappers for NNTP response data.
//!
//! These wrapper types provide type-safe access to response data extracted from
//! [`Response`](super::Response) variants. Each wrapper implements [`Deref`](std::ops::Deref)
//! to its inner type for ergonomic access to the underlying data.
//!
//! These types are used as return types for `TryFrom<Response>` conversions,
//! allowing type-safe extraction of specific response data.

use std::ops::Deref;

use super::{Article, HeaderEntry, NewsGroup, OverviewEntry, Response};
use crate::Error;

/// Server capabilities list.
///
/// Wraps the list of capability strings returned by the CAPABILITIES command (101 response).
/// Each string represents a capability the server supports (e.g., "VERSION 2", "READER", "POST").
///
/// # Example
///
/// ```ignore
/// let caps: Capabilities = response.try_into()?;
/// for capability in caps.iter() {
///     println!("Server supports: {}", capability);
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Capabilities(pub Vec<String>);

impl Deref for Capabilities {
    type Target = Vec<String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TryFrom<Response> for Capabilities {
    type Error = Error;

    fn try_from(response: Response) -> Result<Self, Self::Error> {
        match response {
            Response::Capabilities(caps) => Ok(Capabilities(caps)),
            _ => Err(Error::InvalidResponse(
                "Expected capabilities response".to_string(),
            )),
        }
    }
}

/// Help text from server.
///
/// Wraps the help text lines returned by the HELP command (100 response).
/// Each string is a line of help text describing available commands.
///
/// # Example
///
/// ```ignore
/// let help: HelpText = response.try_into()?;
/// for line in help.iter() {
///     println!("{}", line);
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct HelpText(pub Vec<String>);

impl Deref for HelpText {
    type Target = Vec<String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TryFrom<Response> for HelpText {
    type Error = Error;

    fn try_from(response: Response) -> Result<Self, Self::Error> {
        match response {
            Response::Help(lines) => Ok(HelpText(lines)),
            _ => Err(Error::InvalidResponse(
                "Expected help response".to_string(),
            )),
        }
    }
}

/// List of newsgroups.
///
/// Wraps a list of [`NewsGroup`] entries returned by LIST, NEWGROUPS, or similar commands
/// (215/231 responses). Each entry contains the group name, article range, and posting status.
///
/// # Example
///
/// ```ignore
/// let groups: NewsgroupList = response.try_into()?;
/// for group in groups.iter() {
///     println!("{}: {} articles", group.name, group.last - group.first + 1);
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct NewsgroupList(pub Vec<NewsGroup>);

impl Deref for NewsgroupList {
    type Target = Vec<NewsGroup>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TryFrom<Response> for NewsgroupList {
    type Error = Error;

    fn try_from(response: Response) -> Result<Self, Self::Error> {
        match response {
            Response::NewsgroupList(groups) | Response::NewNewsgroups(groups) => {
                Ok(NewsgroupList(groups))
            }
            _ => Err(Error::InvalidResponse(
                "Expected newsgroup list response".to_string(),
            )),
        }
    }
}

/// List of message IDs.
///
/// Wraps a list of message ID strings returned by the NEWNEWS command (230 response).
/// Each string is a message ID in angle bracket format (e.g., "<abc123@example.com>").
///
/// # Example
///
/// ```ignore
/// let ids: MessageIdList = response.try_into()?;
/// for id in ids.iter() {
///     println!("New article: {}", id);
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct MessageIdList(pub Vec<String>);

impl Deref for MessageIdList {
    type Target = Vec<String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TryFrom<Response> for MessageIdList {
    type Error = Error;

    fn try_from(response: Response) -> Result<Self, Self::Error> {
        match response {
            Response::NewArticles(ids) => Ok(MessageIdList(ids)),
            _ => Err(Error::InvalidResponse(
                "Expected new articles response".to_string(),
            )),
        }
    }
}

/// List of article numbers.
///
/// Wraps a list of article numbers returned by the LISTGROUP command (211 response with listing).
/// Each number is a valid article number within the currently selected group.
///
/// # Example
///
/// ```ignore
/// let articles: ArticleNumbers = response.try_into()?;
/// println!("Group has {} articles", articles.len());
/// for num in articles.iter() {
///     println!("Article: {}", num);
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ArticleNumbers(pub Vec<u64>);

impl Deref for ArticleNumbers {
    type Target = Vec<u64>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TryFrom<Response> for ArticleNumbers {
    type Error = Error;

    fn try_from(response: Response) -> Result<Self, Self::Error> {
        match response {
            Response::ArticleListing(numbers) => Ok(ArticleNumbers(numbers)),
            _ => Err(Error::InvalidResponse(
                "Expected article listing response".to_string(),
            )),
        }
    }
}

/// Newsgroup statistics (count, first, last).
///
/// Contains statistics about a newsgroup returned by the GROUP command (211 response).
/// The `count` is an estimate and may not equal `last - first + 1` due to expired articles.
///
/// # Fields
///
/// * `count` - Estimated number of articles in the group
/// * `first` - Lowest article number in the group
/// * `last` - Highest article number in the group
///
/// # Example
///
/// ```ignore
/// let stats: GroupStats = response.try_into()?;
/// println!("Articles {}-{} (approx {} total)", stats.first, stats.last, stats.count);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroupStats {
    /// Estimated number of articles in the group.
    pub count: u64,
    /// First (lowest) article number in the group.
    pub first: u64,
    /// Last (highest) article number in the group.
    pub last: u64,
}

impl TryFrom<Response> for GroupStats {
    type Error = Error;

    fn try_from(response: Response) -> Result<Self, Self::Error> {
        match response {
            Response::GroupSelected {
                count, first, last, ..
            } => Ok(GroupStats { count, first, last }),
            _ => Err(Error::InvalidResponse(
                "Expected group selected response".to_string(),
            )),
        }
    }
}

/// Posting status (true = allowed).
///
/// Indicates whether posting is allowed on the server, returned by MODE READER (200/201 responses).
/// Dereferences to `bool` where `true` means posting is allowed.
///
/// # Example
///
/// ```ignore
/// let status: PostingStatus = response.try_into()?;
/// if *status {
///     println!("Posting is allowed");
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PostingStatus(pub bool);

impl Deref for PostingStatus {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TryFrom<Response> for PostingStatus {
    type Error = Error;

    fn try_from(response: Response) -> Result<Self, Self::Error> {
        match response {
            Response::ModeReader { posting_allowed } => Ok(PostingStatus(posting_allowed)),
            _ => Err(Error::InvalidResponse(
                "Expected mode reader response".to_string(),
            )),
        }
    }
}

/// Server date/time string.
///
/// Wraps the date/time string returned by the DATE command (111 response).
/// The format is typically "YYYYMMDDhhmmss" in UTC.
///
/// # Example
///
/// ```ignore
/// let date: ServerDate = response.try_into()?;
/// println!("Server time: {}", *date);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ServerDate(pub String);

impl Deref for ServerDate {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TryFrom<Response> for ServerDate {
    type Error = Error;

    fn try_from(response: Response) -> Result<Self, Self::Error> {
        match response {
            Response::Date(date) => Ok(ServerDate(date)),
            _ => Err(Error::InvalidResponse(
                "Expected date response".to_string(),
            )),
        }
    }
}

/// Article pointer information (number and message ID).
///
/// Contains the article number and message ID returned by STAT, NEXT, or LAST commands
/// (223 response). Used to identify an article without retrieving its content.
///
/// # Fields
///
/// * `number` - Article number within the current group
/// * `message_id` - Globally unique message ID
///
/// # Example
///
/// ```ignore
/// let pointer: ArticlePointer = response.try_into()?;
/// println!("Article {} has ID {}", pointer.number, pointer.message_id);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ArticlePointer {
    /// Article number within the current group.
    pub number: u64,
    /// Message ID (globally unique identifier).
    pub message_id: String,
}

impl TryFrom<Response> for ArticlePointer {
    type Error = Error;

    fn try_from(response: Response) -> Result<Self, Self::Error> {
        match response {
            Response::ArticleStatus { number, message_id } => {
                Ok(ArticlePointer { number, message_id })
            }
            _ => Err(Error::InvalidResponse(
                "Expected article status response".to_string(),
            )),
        }
    }
}

/// Header data entries.
///
/// Wraps a list of [`HeaderEntry`] values returned by the HDR command (225 response).
/// Each entry contains an article identifier and the requested header field value.
///
/// # Example
///
/// ```ignore
/// let headers: HeaderData = response.try_into()?;
/// for entry in headers.iter() {
///     println!("{}: {}", entry.article, entry.value);
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct HeaderData(pub Vec<HeaderEntry>);

impl Deref for HeaderData {
    type Target = Vec<HeaderEntry>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TryFrom<Response> for HeaderData {
    type Error = Error;

    fn try_from(response: Response) -> Result<Self, Self::Error> {
        match response {
            Response::HeaderData(entries) => Ok(HeaderData(entries)),
            _ => Err(Error::InvalidResponse(
                "Expected header data response".to_string(),
            )),
        }
    }
}

/// Overview data entries.
///
/// Wraps a list of [`OverviewEntry`] values returned by the OVER command (224 response).
/// Each entry contains tab-separated fields with article metadata (subject, from, date, etc.).
///
/// # Example
///
/// ```ignore
/// let overview: OverviewData = response.try_into()?;
/// for entry in overview.iter() {
///     if let Some(subject) = entry.subject() {
///         println!("Subject: {}", subject);
///     }
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct OverviewData(pub Vec<OverviewEntry>);

impl Deref for OverviewData {
    type Target = Vec<OverviewEntry>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TryFrom<Response> for OverviewData {
    type Error = Error;

    fn try_from(response: Response) -> Result<Self, Self::Error> {
        match response {
            Response::OverviewData(entries) => Ok(OverviewData(entries)),
            _ => Err(Error::InvalidResponse(
                "Expected overview data response".to_string(),
            )),
        }
    }
}

/// Overview format field names.
///
/// Wraps the list of field names returned by LIST OVERVIEW.FMT (215 response).
/// Describes the order of fields in OVER command responses.
///
/// # Example
///
/// ```ignore
/// let format: OverviewFormat = response.try_into()?;
/// for (i, field) in format.iter().enumerate() {
///     println!("Field {}: {}", i, field);
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct OverviewFormat(pub Vec<String>);

impl Deref for OverviewFormat {
    type Target = Vec<String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TryFrom<Response> for OverviewFormat {
    type Error = Error;

    fn try_from(response: Response) -> Result<Self, Self::Error> {
        match response {
            Response::OverviewFormat(fields) => Ok(OverviewFormat(fields)),
            _ => Err(Error::InvalidResponse(
                "Expected overview format response".to_string(),
            )),
        }
    }
}

// TryFrom<Response> for Article
impl TryFrom<Response> for Article {
    type Error = Error;

    fn try_from(response: Response) -> Result<Self, Self::Error> {
        match response {
            Response::Article {
                number,
                message_id,
                content,
            } => Ok(Article::new(number, message_id, content)),
            _ => Err(Error::InvalidResponse(
                "Expected article response".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capabilities_deref() {
        let caps = Capabilities(vec!["VERSION 2".to_string(), "READER".to_string()]);
        assert_eq!(caps.len(), 2);
        assert_eq!(caps[0], "VERSION 2");
        assert!(caps.contains(&"READER".to_string()));
    }

    #[test]
    fn test_help_text_deref() {
        let help = HelpText(vec!["HELP".to_string(), "GROUP".to_string()]);
        assert_eq!(help.len(), 2);
        assert_eq!(help.first(), Some(&"HELP".to_string()));
    }

    #[test]
    fn test_newsgroup_list_deref() {
        let groups = NewsgroupList(vec![NewsGroup {
            name: "misc.test".to_string(),
            first: 1,
            last: 100,
            posting_status: 'y',
        }]);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "misc.test");
    }

    #[test]
    fn test_message_id_list_deref() {
        let ids = MessageIdList(vec!["<abc@example.com>".to_string()]);
        assert_eq!(ids.len(), 1);
        assert!(ids[0].starts_with('<'));
    }

    #[test]
    fn test_article_numbers_deref() {
        let nums = ArticleNumbers(vec![1, 2, 3, 5, 8]);
        assert_eq!(nums.len(), 5);
        assert_eq!(nums.iter().sum::<u64>(), 19);
    }

    #[test]
    fn test_group_stats() {
        let stats = GroupStats {
            count: 100,
            first: 1,
            last: 150,
        };
        assert_eq!(stats.count, 100);
        assert_eq!(stats.first, 1);
        assert_eq!(stats.last, 150);
    }

    #[test]
    fn test_posting_status_deref() {
        let allowed = PostingStatus(true);
        let denied = PostingStatus(false);
        assert!(*allowed);
        assert!(!*denied);
    }

    #[test]
    fn test_server_date_deref() {
        let date = ServerDate("20231106123456".to_string());
        assert!(date.starts_with("2023"));
        assert_eq!(date.len(), 14);
    }

    #[test]
    fn test_article_pointer() {
        let pointer = ArticlePointer {
            number: 12345,
            message_id: "<abc@example.com>".to_string(),
        };
        assert_eq!(pointer.number, 12345);
        assert!(pointer.message_id.contains('@'));
    }

    #[test]
    fn test_header_data_deref() {
        let headers = HeaderData(vec![HeaderEntry {
            article: "1234".to_string(),
            value: "Test Subject".to_string(),
        }]);
        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].value, "Test Subject");
    }

    #[test]
    fn test_overview_data_deref() {
        let overview = OverviewData(vec![OverviewEntry {
            fields: vec!["1234".to_string(), "Test Subject".to_string()],
        }]);
        assert_eq!(overview.len(), 1);
        assert_eq!(overview[0].subject(), Some("Test Subject"));
    }

    #[test]
    fn test_overview_format_deref() {
        let format = OverviewFormat(vec![
            "Subject:".to_string(),
            "From:".to_string(),
            "Date:".to_string(),
        ]);
        assert_eq!(format.len(), 3);
        assert!(format.contains(&"Subject:".to_string()));
    }

    // TryFrom tests

    #[test]
    fn test_capabilities_try_from_success() {
        let response = Response::Capabilities(vec!["VERSION 2".to_string(), "READER".to_string()]);
        let caps: Capabilities = response.try_into().unwrap();
        assert_eq!(caps.len(), 2);
        assert_eq!(caps[0], "VERSION 2");
    }

    #[test]
    fn test_capabilities_try_from_error() {
        let response = Response::Quit;
        let result: Result<Capabilities, _> = response.try_into();
        assert!(result.is_err());
        match result.unwrap_err() {
            Error::InvalidResponse(msg) => assert!(msg.contains("capabilities")),
            _ => panic!("Expected InvalidResponse error"),
        }
    }

    #[test]
    fn test_help_text_try_from_success() {
        let response = Response::Help(vec!["HELP".to_string(), "GROUP".to_string()]);
        let help: HelpText = response.try_into().unwrap();
        assert_eq!(help.len(), 2);
    }

    #[test]
    fn test_help_text_try_from_error() {
        let response = Response::Quit;
        let result: Result<HelpText, _> = response.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn test_newsgroup_list_try_from_newsgroup_list() {
        let response = Response::NewsgroupList(vec![NewsGroup {
            name: "misc.test".to_string(),
            first: 1,
            last: 100,
            posting_status: 'y',
        }]);
        let list: NewsgroupList = response.try_into().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "misc.test");
    }

    #[test]
    fn test_newsgroup_list_try_from_new_newsgroups() {
        let response = Response::NewNewsgroups(vec![NewsGroup {
            name: "comp.new".to_string(),
            first: 1,
            last: 50,
            posting_status: 'n',
        }]);
        let list: NewsgroupList = response.try_into().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "comp.new");
    }

    #[test]
    fn test_newsgroup_list_try_from_error() {
        let response = Response::Quit;
        let result: Result<NewsgroupList, _> = response.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn test_message_id_list_try_from_success() {
        let response = Response::NewArticles(vec![
            "<abc@example.com>".to_string(),
            "<def@example.com>".to_string(),
        ]);
        let ids: MessageIdList = response.try_into().unwrap();
        assert_eq!(ids.len(), 2);
    }

    #[test]
    fn test_message_id_list_try_from_error() {
        let response = Response::Quit;
        let result: Result<MessageIdList, _> = response.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn test_article_numbers_try_from_success() {
        let response = Response::ArticleListing(vec![1, 2, 3, 5, 8]);
        let nums: ArticleNumbers = response.try_into().unwrap();
        assert_eq!(nums.len(), 5);
    }

    #[test]
    fn test_article_numbers_try_from_error() {
        let response = Response::Quit;
        let result: Result<ArticleNumbers, _> = response.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn test_group_stats_try_from_success() {
        let response = Response::GroupSelected {
            count: 100,
            first: 1,
            last: 150,
            name: "misc.test".to_string(),
        };
        let stats: GroupStats = response.try_into().unwrap();
        assert_eq!(stats.count, 100);
        assert_eq!(stats.first, 1);
        assert_eq!(stats.last, 150);
    }

    #[test]
    fn test_group_stats_try_from_error() {
        let response = Response::Quit;
        let result: Result<GroupStats, _> = response.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn test_posting_status_try_from_success() {
        let response = Response::ModeReader {
            posting_allowed: true,
        };
        let status: PostingStatus = response.try_into().unwrap();
        assert!(*status);

        let response = Response::ModeReader {
            posting_allowed: false,
        };
        let status: PostingStatus = response.try_into().unwrap();
        assert!(!*status);
    }

    #[test]
    fn test_posting_status_try_from_error() {
        let response = Response::Quit;
        let result: Result<PostingStatus, _> = response.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn test_server_date_try_from_success() {
        let response = Response::Date("20231106123456".to_string());
        let date: ServerDate = response.try_into().unwrap();
        assert_eq!(*date, "20231106123456");
    }

    #[test]
    fn test_server_date_try_from_error() {
        let response = Response::Quit;
        let result: Result<ServerDate, _> = response.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn test_article_pointer_try_from_success() {
        let response = Response::ArticleStatus {
            number: 12345,
            message_id: "<abc@example.com>".to_string(),
        };
        let pointer: ArticlePointer = response.try_into().unwrap();
        assert_eq!(pointer.number, 12345);
        assert_eq!(pointer.message_id, "<abc@example.com>");
    }

    #[test]
    fn test_article_pointer_try_from_error() {
        let response = Response::Quit;
        let result: Result<ArticlePointer, _> = response.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn test_header_data_try_from_success() {
        let response = Response::HeaderData(vec![HeaderEntry {
            article: "1234".to_string(),
            value: "Test Subject".to_string(),
        }]);
        let headers: HeaderData = response.try_into().unwrap();
        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].value, "Test Subject");
    }

    #[test]
    fn test_header_data_try_from_error() {
        let response = Response::Quit;
        let result: Result<HeaderData, _> = response.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn test_overview_data_try_from_success() {
        let response = Response::OverviewData(vec![OverviewEntry {
            fields: vec!["1234".to_string(), "Test Subject".to_string()],
        }]);
        let overview: OverviewData = response.try_into().unwrap();
        assert_eq!(overview.len(), 1);
    }

    #[test]
    fn test_overview_data_try_from_error() {
        let response = Response::Quit;
        let result: Result<OverviewData, _> = response.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn test_overview_format_try_from_success() {
        let response = Response::OverviewFormat(vec![
            "Subject:".to_string(),
            "From:".to_string(),
        ]);
        let format: OverviewFormat = response.try_into().unwrap();
        assert_eq!(format.len(), 2);
    }

    #[test]
    fn test_overview_format_try_from_error() {
        let response = Response::Quit;
        let result: Result<OverviewFormat, _> = response.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn test_article_try_from_success() {
        let content = b"From: test@example.com\r\nSubject: Test\r\n\r\nBody".to_vec();
        let response = Response::Article {
            number: Some(100),
            message_id: "<test@example.com>".to_string(),
            content: content.clone(),
        };
        let article: Article = response.try_into().unwrap();
        assert_eq!(article.number(), Some(100));
        assert_eq!(article.article_id(), "<test@example.com>");
        assert_eq!(article.raw_content(), &content);
    }

    #[test]
    fn test_article_try_from_error() {
        let response = Response::Quit;
        let result: Result<Article, _> = response.try_into();
        assert!(result.is_err());
        match result.unwrap_err() {
            Error::InvalidResponse(msg) => assert!(msg.contains("article")),
            _ => panic!("Expected InvalidResponse error"),
        }
    }
}
