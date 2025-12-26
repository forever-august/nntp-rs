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
            _ => Err(Error::InvalidResponse("Expected help response".to_string())),
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
            _ => Err(Error::InvalidResponse("Expected date response".to_string())),
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

/// Newsgroup creation time entry.
///
/// Represents a single entry from LIST ACTIVE.TIMES response (RFC 3977 Section 7.6.4).
/// Each entry contains the newsgroup name, creation time, and creator information.
///
/// # Format
///
/// The wire format is: `groupname timestamp creator`
/// - `groupname` - The name of the newsgroup
/// - `timestamp` - Unix timestamp (seconds since epoch) when the group was created
/// - `creator` - Email address or identifier of who created the group
///
/// # Example
///
/// ```ignore
/// let entry = ActiveTimeEntry {
///     name: "comp.lang.rust".to_string(),
///     timestamp: 1609459200,
///     creator: "admin@example.com".to_string(),
/// };
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ActiveTimeEntry {
    /// Newsgroup name.
    pub name: String,
    /// Unix timestamp when the group was created.
    pub timestamp: u64,
    /// Email address or identifier of the creator.
    pub creator: String,
}

/// List of newsgroup creation times.
///
/// Wraps a list of [`ActiveTimeEntry`] values returned by LIST ACTIVE.TIMES (215 response).
/// Each entry contains the group name, creation timestamp, and creator information.
///
/// # Example
///
/// ```ignore
/// let times: ActiveTimesList = response.try_into()?;
/// for entry in times.iter() {
///     println!("{} created at {} by {}", entry.name, entry.timestamp, entry.creator);
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ActiveTimesList(pub Vec<ActiveTimeEntry>);

impl Deref for ActiveTimesList {
    type Target = Vec<ActiveTimeEntry>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Distribution pattern entry.
///
/// Represents a single entry from LIST DISTRIB.PATS response (RFC 3977 Section 7.6.5).
/// Each entry defines a default distribution for newsgroups matching a pattern.
///
/// # Format
///
/// The wire format is: `weight:wildmat:distribution`
/// - `weight` - Numeric weight for pattern priority (higher = more specific)
/// - `wildmat` - Wildcard pattern matching newsgroup names
/// - `distribution` - Default distribution value to use
///
/// # Example
///
/// ```ignore
/// let pat = DistribPat {
///     weight: 10,
///     wildmat: "comp.*".to_string(),
///     distribution: "world".to_string(),
/// };
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct DistribPat {
    /// Weight for pattern priority (higher = more specific).
    pub weight: u32,
    /// Wildcard pattern matching newsgroup names.
    pub wildmat: String,
    /// Default distribution value.
    pub distribution: String,
}

/// List of distribution patterns.
///
/// Wraps a list of [`DistribPat`] values returned by LIST DISTRIB.PATS (215 response).
/// Used to determine default Distribution header values for new posts.
///
/// # Example
///
/// ```ignore
/// let pats: DistribPatsList = response.try_into()?;
/// for pat in pats.iter() {
///     println!("Weight {}: {} -> {}", pat.weight, pat.wildmat, pat.distribution);
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct DistribPatsList(pub Vec<DistribPat>);

impl Deref for DistribPatsList {
    type Target = Vec<DistribPat>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// List of available headers for HDR command.
///
/// Wraps a list of header/metadata field names returned by LIST HEADERS (215 response).
/// These are the fields that can be retrieved using the HDR command.
///
/// # Standard Fields
///
/// Per RFC 3977 Section 8.6, the list typically includes:
/// - Standard header names (e.g., "Subject", "From", "Date")
/// - The special token ":" indicating all standard headers
/// - Metadata items starting with ":" (e.g., ":bytes", ":lines")
///
/// # Example
///
/// ```ignore
/// let headers: HeadersList = response.try_into()?;
/// for field in headers.iter() {
///     println!("Available header: {}", field);
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct HeadersList(pub Vec<String>);

impl Deref for HeadersList {
    type Target = Vec<String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Newsgroup counts entry.
///
/// Represents a single entry from LIST COUNTS response.
/// Each entry contains newsgroup statistics including article counts.
///
/// # Format
///
/// The wire format is: `groupname count low high status`
/// - `groupname` - The name of the newsgroup
/// - `count` - Estimated number of articles
/// - `low` - Lowest article number
/// - `high` - Highest article number
/// - `status` - Posting status character ('y', 'n', 'm', etc.)
///
/// # Note
///
/// LIST COUNTS is not defined in RFC 3977 but is supported by some servers.
/// It's similar to LIST ACTIVE but may include more accurate counts.
#[derive(Debug, Clone, PartialEq)]
pub struct CountsEntry {
    /// Newsgroup name.
    pub name: String,
    /// Estimated number of articles in the group.
    pub count: u64,
    /// Lowest article number in the group.
    pub low: u64,
    /// Highest article number in the group.
    pub high: u64,
    /// Posting status ('y' = posting allowed, 'n' = no posting, 'm' = moderated).
    pub status: char,
}

/// List of newsgroup counts.
///
/// Wraps a list of [`CountsEntry`] values returned by LIST COUNTS.
///
/// # Example
///
/// ```ignore
/// let counts: CountsList = response.try_into()?;
/// for entry in counts.iter() {
///     println!("{}: {} articles ({}-{})", entry.name, entry.count, entry.low, entry.high);
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct CountsList(pub Vec<CountsEntry>);

impl Deref for CountsList {
    type Target = Vec<CountsEntry>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Moderator pattern entry.
///
/// Represents a single entry from LIST MODERATORS response.
/// Each entry defines how to find the moderator email for newsgroups matching a pattern.
///
/// # Format
///
/// The wire format is: `wildmat:template`
/// - `wildmat` - Wildcard pattern matching newsgroup names
/// - `template` - Email template with %s replaced by group name components
///
/// # Example
///
/// ```ignore
/// let mod_entry = ModeratorEntry {
///     wildmat: "comp.*".to_string(),
///     template: "%s@moderators.isc.org".to_string(),
/// };
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ModeratorEntry {
    /// Wildcard pattern matching newsgroup names.
    pub wildmat: String,
    /// Email template (%s is replaced with group name components).
    pub template: String,
}

/// List of moderator patterns.
///
/// Wraps a list of [`ModeratorEntry`] values returned by LIST MODERATORS.
///
/// # Example
///
/// ```ignore
/// let mods: ModeratorsList = response.try_into()?;
/// for entry in mods.iter() {
///     println!("{} -> {}", entry.wildmat, entry.template);
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ModeratorsList(pub Vec<ModeratorEntry>);

impl Deref for ModeratorsList {
    type Target = Vec<ModeratorEntry>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Distribution entry.
///
/// Represents a single entry from LIST DISTRIBUTIONS response.
/// Each entry describes a valid distribution value.
///
/// # Format
///
/// The wire format is: `distribution description`
/// - `distribution` - The distribution value (e.g., "world", "local")
/// - `description` - Human-readable description
///
/// # Example
///
/// ```ignore
/// let dist = DistributionEntry {
///     name: "world".to_string(),
///     description: "Worldwide distribution".to_string(),
/// };
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct DistributionEntry {
    /// Distribution value.
    pub name: String,
    /// Human-readable description.
    pub description: String,
}

/// List of valid distributions.
///
/// Wraps a list of [`DistributionEntry`] values returned by LIST DISTRIBUTIONS.
///
/// # Example
///
/// ```ignore
/// let dists: DistributionsList = response.try_into()?;
/// for entry in dists.iter() {
///     println!("{}: {}", entry.name, entry.description);
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct DistributionsList(pub Vec<DistributionEntry>);

impl Deref for DistributionsList {
    type Target = Vec<DistributionEntry>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Newsgroup description entry.
///
/// Represents a single entry from LIST NEWSGROUPS response (RFC 3977 Section 7.6.6).
/// Each entry contains a newsgroup name and its description.
///
/// # Format
///
/// The wire format is: `groupname description`
/// - `groupname` - The name of the newsgroup
/// - `description` - Human-readable description of the newsgroup
///
/// # Example
///
/// ```ignore
/// let entry = NewsgroupDesc {
///     name: "comp.lang.rust".to_string(),
///     description: "Discussion about the Rust programming language".to_string(),
/// };
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct NewsgroupDesc {
    /// Newsgroup name.
    pub name: String,
    /// Human-readable description.
    pub description: String,
}

/// List of newsgroup descriptions.
///
/// Wraps a list of [`NewsgroupDesc`] values returned by LIST NEWSGROUPS (215 response).
/// Each entry contains the group name and a human-readable description.
///
/// # Example
///
/// ```ignore
/// let descs: NewsgroupDescList = response.try_into()?;
/// for entry in descs.iter() {
///     println!("{}: {}", entry.name, entry.description);
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct NewsgroupDescList(pub Vec<NewsgroupDesc>);

impl Deref for NewsgroupDescList {
    type Target = Vec<NewsgroupDesc>;

    fn deref(&self) -> &Self::Target {
        &self.0
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
        let response = Response::OverviewFormat(vec!["Subject:".to_string(), "From:".to_string()]);
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

    // Tests for new LIST variant wrapper types

    #[test]
    fn test_active_time_entry() {
        let entry = ActiveTimeEntry {
            name: "comp.lang.rust".to_string(),
            timestamp: 1609459200,
            creator: "admin@example.com".to_string(),
        };
        assert_eq!(entry.name, "comp.lang.rust");
        assert_eq!(entry.timestamp, 1609459200);
        assert_eq!(entry.creator, "admin@example.com");
    }

    #[test]
    fn test_active_times_list_deref() {
        let times = ActiveTimesList(vec![
            ActiveTimeEntry {
                name: "comp.lang.rust".to_string(),
                timestamp: 1609459200,
                creator: "admin@example.com".to_string(),
            },
            ActiveTimeEntry {
                name: "alt.test".to_string(),
                timestamp: 1609545600,
                creator: "other@example.com".to_string(),
            },
        ]);
        assert_eq!(times.len(), 2);
        assert_eq!(times[0].name, "comp.lang.rust");
        assert_eq!(times[1].name, "alt.test");
    }

    #[test]
    fn test_distrib_pat() {
        let pat = DistribPat {
            weight: 10,
            wildmat: "comp.*".to_string(),
            distribution: "world".to_string(),
        };
        assert_eq!(pat.weight, 10);
        assert_eq!(pat.wildmat, "comp.*");
        assert_eq!(pat.distribution, "world");
    }

    #[test]
    fn test_distrib_pats_list_deref() {
        let pats = DistribPatsList(vec![
            DistribPat {
                weight: 10,
                wildmat: "comp.*".to_string(),
                distribution: "world".to_string(),
            },
            DistribPat {
                weight: 5,
                wildmat: "local.*".to_string(),
                distribution: "local".to_string(),
            },
        ]);
        assert_eq!(pats.len(), 2);
        assert_eq!(pats[0].wildmat, "comp.*");
        assert_eq!(pats[1].distribution, "local");
    }

    #[test]
    fn test_headers_list_deref() {
        let headers = HeadersList(vec![
            "Subject".to_string(),
            "From".to_string(),
            ":bytes".to_string(),
            ":lines".to_string(),
        ]);
        assert_eq!(headers.len(), 4);
        assert!(headers.contains(&"Subject".to_string()));
        assert!(headers.contains(&":bytes".to_string()));
    }

    #[test]
    fn test_counts_entry() {
        let entry = CountsEntry {
            name: "comp.lang.rust".to_string(),
            count: 1234,
            low: 1,
            high: 5000,
            status: 'y',
        };
        assert_eq!(entry.name, "comp.lang.rust");
        assert_eq!(entry.count, 1234);
        assert_eq!(entry.low, 1);
        assert_eq!(entry.high, 5000);
        assert_eq!(entry.status, 'y');
    }

    #[test]
    fn test_counts_list_deref() {
        let counts = CountsList(vec![CountsEntry {
            name: "misc.test".to_string(),
            count: 100,
            low: 1,
            high: 200,
            status: 'm',
        }]);
        assert_eq!(counts.len(), 1);
        assert_eq!(counts[0].status, 'm');
    }

    #[test]
    fn test_moderator_entry() {
        let entry = ModeratorEntry {
            wildmat: "comp.lang.*".to_string(),
            template: "%s@moderators.example.org".to_string(),
        };
        assert_eq!(entry.wildmat, "comp.lang.*");
        assert_eq!(entry.template, "%s@moderators.example.org");
    }

    #[test]
    fn test_moderators_list_deref() {
        let mods = ModeratorsList(vec![
            ModeratorEntry {
                wildmat: "comp.*".to_string(),
                template: "%s@comp-moderators.example.org".to_string(),
            },
            ModeratorEntry {
                wildmat: "*".to_string(),
                template: "%s@default-moderators.example.org".to_string(),
            },
        ]);
        assert_eq!(mods.len(), 2);
        assert_eq!(mods[0].wildmat, "comp.*");
    }

    #[test]
    fn test_distribution_entry() {
        let entry = DistributionEntry {
            name: "world".to_string(),
            description: "Worldwide distribution".to_string(),
        };
        assert_eq!(entry.name, "world");
        assert_eq!(entry.description, "Worldwide distribution");
    }

    #[test]
    fn test_distributions_list_deref() {
        let dists = DistributionsList(vec![
            DistributionEntry {
                name: "world".to_string(),
                description: "Worldwide distribution".to_string(),
            },
            DistributionEntry {
                name: "local".to_string(),
                description: "Local distribution only".to_string(),
            },
        ]);
        assert_eq!(dists.len(), 2);
        assert_eq!(dists[0].name, "world");
        assert_eq!(dists[1].name, "local");
    }

    #[test]
    fn test_newsgroup_desc() {
        let entry = NewsgroupDesc {
            name: "comp.lang.rust".to_string(),
            description: "Discussion about the Rust programming language".to_string(),
        };
        assert_eq!(entry.name, "comp.lang.rust");
        assert!(entry.description.contains("Rust"));
    }

    #[test]
    fn test_newsgroup_desc_list_deref() {
        let descs = NewsgroupDescList(vec![
            NewsgroupDesc {
                name: "comp.lang.rust".to_string(),
                description: "Rust programming".to_string(),
            },
            NewsgroupDesc {
                name: "comp.lang.c".to_string(),
                description: "C programming".to_string(),
            },
        ]);
        assert_eq!(descs.len(), 2);
        assert_eq!(descs[0].name, "comp.lang.rust");
        assert_eq!(descs[1].name, "comp.lang.c");
    }
}
