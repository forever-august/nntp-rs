//! RFC3977 compliance tests for NNTP client implementation.
//!
//! These tests validate the client's behavior against examples from RFC3977
//! using the mock server infrastructure.

use nntp_rs::mock::ClientMockTest;
use nntp_rs::{ArticleSpec, Command, Response};

/// Test basic connection and capabilities exchange as per RFC3977 Section 5.1
#[test]
fn test_rfc3977_basic_capabilities() {
    let interactions = vec![(
        Command::Capabilities,
        Response::Capabilities(vec![
            "VERSION 2".to_string(),
            "READER".to_string(),
            "IHAVE".to_string(),
            "POST".to_string(),
            "NEWNEWS".to_string(),
            "HDR".to_string(),
        ]),
    )];

    let mut test = ClientMockTest::new(interactions);

    // Test capabilities request
    let response = test.send_command(Command::Capabilities).unwrap();
    if let Response::Capabilities(caps) = response {
        assert!(caps.contains(&"VERSION 2".to_string()));
        assert!(caps.contains(&"READER".to_string()));
        assert!(caps.len() >= 2);
    } else {
        panic!("Expected Capabilities response");
    }

    assert!(test.is_complete());
}

/// Test mode reader command as per RFC3977 Section 5.3
#[test]
fn test_rfc3977_mode_reader() {
    let interactions = vec![(
        Command::ModeReader,
        Response::ModeReader {
            posting_allowed: true,
        },
    )];

    let mut test = ClientMockTest::new(interactions);

    let response = test.send_command(Command::ModeReader).unwrap();
    if let Response::ModeReader { posting_allowed } = response {
        assert!(posting_allowed);
    } else {
        panic!("Expected ModeReader response");
    }

    assert_eq!(test.client().state(), "reader");
    assert!(test.is_complete());
}

/// Test group selection as per RFC3977 Section 6.1.1
#[test]
fn test_rfc3977_group_selection() {
    let interactions = vec![(
        Command::Group("misc.test".to_string()),
        Response::GroupSelected {
            count: 3000,
            first: 3000,
            last: 3002,
            name: "misc.test".to_string(),
        },
    )];

    let mut test = ClientMockTest::new(interactions);

    let response = test
        .send_command(Command::Group("misc.test".to_string()))
        .unwrap();

    if let Response::GroupSelected {
        name,
        count,
        first,
        last,
    } = response
    {
        assert_eq!(name, "misc.test");
        assert_eq!(count, 3000);
        assert_eq!(first, 3000);
        assert_eq!(last, 3002);
    } else {
        panic!("Expected GroupSelected response");
    }

    assert_eq!(test.client().state(), "group_selected");
    assert_eq!(test.client().current_group(), Some("misc.test"));
    assert!(test.is_complete());
}

/// Test article retrieval by number as per RFC3977 Section 6.2.1
#[test]
fn test_rfc3977_article_by_number() {
    let interactions = vec![
        // First select a group
        (
            Command::Group("misc.test".to_string()),
            Response::GroupSelected {
                count: 3000,
                first: 3000,
                last: 3002,
                name: "misc.test".to_string(),
            },
        ),
        // Then retrieve an article
        (
            Command::Article(ArticleSpec::Number(3000)),
            Response::Article {
                number: Some(3000),
                message_id: "<45223423@example.com>".to_string(),
                content: b"From: \"Demo User\" <nobody@example.com>\r\nNewsgroups: misc.test\r\nSubject: I am just a test article\r\nDate: 6 Oct 1998 04:38:40 -0500\r\nOrganization: An Example Net\r\n\r\nThis is just a test article.\r\n".to_vec(),
            },
        ),
    ];

    let mut test = ClientMockTest::new(interactions);

    // Select group
    test.send_command(Command::Group("misc.test".to_string()))
        .unwrap();

    // Retrieve article
    let response = test
        .send_command(Command::Article(ArticleSpec::Number(3000)))
        .unwrap();

    if let Response::Article {
        number,
        message_id,
        content,
    } = response
    {
        assert_eq!(number, Some(3000));
        assert_eq!(message_id, "<45223423@example.com>");
        assert!(!content.is_empty());
        let content_str = String::from_utf8_lossy(&content);
        assert!(content_str.contains("Subject: I am just a test article"));
    } else {
        panic!("Expected Article response");
    }

    assert!(test.is_complete());
}

/// Test article retrieval by message-id as per RFC3977 Section 6.2.1
#[test]
fn test_rfc3977_article_by_message_id() {
    let interactions = vec![
        (
            Command::Article(ArticleSpec::MessageId("<45223423@example.com>".to_string())),
            Response::Article {
                number: Some(3000),
                message_id: "<45223423@example.com>".to_string(),
                content: b"From: \"Demo User\" <nobody@example.com>\r\nNewsgroups: misc.test\r\nSubject: I am just a test article\r\n\r\nThis is just a test article.\r\n".to_vec(),
            },
        ),
    ];

    let mut test = ClientMockTest::new(interactions);

    let response = test
        .send_command(Command::Article(ArticleSpec::MessageId(
            "<45223423@example.com>".to_string(),
        )))
        .unwrap();

    if let Response::Article {
        message_id,
        content,
        ..
    } = response
    {
        assert_eq!(message_id, "<45223423@example.com>");
        let content_str = String::from_utf8_lossy(&content);
        assert!(content_str.contains("Subject: I am just a test article"));
    } else {
        panic!("Expected Article response");
    }

    assert!(test.is_complete());
}

/// Test HEAD command as per RFC3977 Section 6.2.2
#[test]
fn test_rfc3977_head_command() {
    let interactions = vec![
        (
            Command::Group("misc.test".to_string()),
            Response::GroupSelected {
                count: 3000,
                first: 3000,
                last: 3002,
                name: "misc.test".to_string(),
            },
        ),
        (
            Command::Head(ArticleSpec::Number(3000)),
            Response::Article {
                number: Some(3000),
                message_id: "<45223423@example.com>".to_string(),
                content: b"From: \"Demo User\" <nobody@example.com>\r\nNewsgroups: misc.test\r\nSubject: I am just a test article\r\nDate: 6 Oct 1998 04:38:40 -0500\r\n".to_vec(),
            },
        ),
    ];

    let mut test = ClientMockTest::new(interactions);

    // Select group
    test.send_command(Command::Group("misc.test".to_string()))
        .unwrap();

    // Get headers
    let response = test
        .send_command(Command::Head(ArticleSpec::Number(3000)))
        .unwrap();

    if let Response::Article { content, .. } = response {
        let content_str = String::from_utf8_lossy(&content);
        assert!(content_str.contains("From: \"Demo User\""));
        assert!(content_str.contains("Subject: I am just a test article"));
        // Headers only, should not contain article body
        assert!(!content_str.contains("This is just a test article"));
    } else {
        panic!("Expected Article response");
    }

    assert!(test.is_complete());
}

/// Test BODY command as per RFC3977 Section 6.2.3
#[test]
fn test_rfc3977_body_command() {
    let interactions = vec![
        (
            Command::Group("misc.test".to_string()),
            Response::GroupSelected {
                count: 3000,
                first: 3000,
                last: 3002,
                name: "misc.test".to_string(),
            },
        ),
        (
            Command::Body(ArticleSpec::Number(3000)),
            Response::Article {
                number: Some(3000),
                message_id: "<45223423@example.com>".to_string(),
                content: b"This is just a test article.\r\n".to_vec(),
            },
        ),
    ];

    let mut test = ClientMockTest::new(interactions);

    // Select group
    test.send_command(Command::Group("misc.test".to_string()))
        .unwrap();

    // Get body
    let response = test
        .send_command(Command::Body(ArticleSpec::Number(3000)))
        .unwrap();

    if let Response::Article { content, .. } = response {
        let content_str = String::from_utf8_lossy(&content);
        assert!(content_str.contains("This is just a test article"));
        // Body only, should not contain headers
        assert!(!content_str.contains("From:"));
        assert!(!content_str.contains("Subject:"));
    } else {
        panic!("Expected Article response");
    }

    assert!(test.is_complete());
}

/// Test STAT command as per RFC3977 Section 6.2.4
#[test]
fn test_rfc3977_stat_command() {
    let interactions = vec![
        (
            Command::Group("misc.test".to_string()),
            Response::GroupSelected {
                count: 3000,
                first: 3000,
                last: 3002,
                name: "misc.test".to_string(),
            },
        ),
        (
            Command::Stat(ArticleSpec::Number(3000)),
            Response::ArticleStatus {
                number: 3000,
                message_id: "<45223423@example.com>".to_string(),
            },
        ),
    ];

    let mut test = ClientMockTest::new(interactions);

    // Select group
    test.send_command(Command::Group("misc.test".to_string()))
        .unwrap();

    // Get status
    let response = test
        .send_command(Command::Stat(ArticleSpec::Number(3000)))
        .unwrap();

    if let Response::ArticleStatus { number, message_id } = response {
        assert_eq!(number, 3000);
        assert_eq!(message_id, "<45223423@example.com>");
    } else {
        panic!("Expected ArticleStatus response");
    }

    assert!(test.is_complete());
}

/// Test LIST command as per RFC3977 Section 7.6.1
#[test]
fn test_rfc3977_list_command() {
    use nntp_rs::response::NewsGroup;

    let interactions = vec![(
        Command::List(None),
        Response::NewsgroupList(vec![
            NewsGroup {
                name: "misc.test".to_string(),
                last: 3002,
                first: 3000,
                posting_status: 'y',
            },
            NewsGroup {
                name: "comp.risks".to_string(),
                last: 442418,
                first: 1,
                posting_status: 'm',
            },
            NewsGroup {
                name: "alt.rfc-writers.recovery".to_string(),
                last: 4,
                first: 1,
                posting_status: 'y',
            },
        ]),
    )];

    let mut test = ClientMockTest::new(interactions);

    let response = test.send_command(Command::List(None)).unwrap();

    if let Response::NewsgroupList(groups) = response {
        assert_eq!(groups.len(), 3);

        // Check first group
        assert_eq!(groups[0].name, "misc.test");
        assert_eq!(groups[0].last, 3002);
        assert_eq!(groups[0].first, 3000);
        assert_eq!(groups[0].posting_status, 'y');

        // Check second group
        assert_eq!(groups[1].name, "comp.risks");
        assert_eq!(groups[1].posting_status, 'm');
    } else {
        panic!("Expected NewsgroupList response");
    }

    assert!(test.is_complete());
}

/// Test POST command sequence as per RFC3977 Section 6.3.1
#[test]
fn test_rfc3977_post_sequence() {
    let interactions = vec![
        (Command::Post, Response::PostAccepted),
        // Note: In a real scenario, the client would send article content here
        // but since this is testing the protocol sequence, we simulate success
    ];

    let mut test = ClientMockTest::new(interactions);

    let response = test.send_command(Command::Post).unwrap();

    if let Response::PostAccepted = response {
        // Expected
    } else {
        panic!("Expected PostAccepted response");
    }

    assert_eq!(test.client().state(), "posting");
    assert!(test.is_complete());
}

/// Test error responses as per RFC3977 Section 3.2
#[test]
fn test_rfc3977_error_responses() {
    let interactions = vec![
        (
            Command::Group("nonexistent.group".to_string()),
            Response::Error {
                code: 411,
                message: "No such newsgroup".to_string(),
            },
        ),
        (
            Command::Article(ArticleSpec::Number(999999)),
            Response::Error {
                code: 423,
                message: "No article with that number".to_string(),
            },
        ),
    ];

    let mut test = ClientMockTest::new(interactions);

    // Test nonexistent group
    let response = test
        .send_command(Command::Group("nonexistent.group".to_string()))
        .unwrap();

    if let Response::Error { code, message } = response {
        assert_eq!(code, 411);
        assert!(message.contains("No such newsgroup"));
    } else {
        panic!("Expected Error response");
    }

    // Test nonexistent article
    let response = test
        .send_command(Command::Article(ArticleSpec::Number(999999)))
        .unwrap();

    if let Response::Error { code, message } = response {
        assert_eq!(code, 423);
        assert!(message.contains("No article with that number"));
    } else {
        panic!("Expected Error response");
    }

    assert!(test.is_complete());
}

/// Test IHAVE command as per RFC 3977 Section 6.3.2
#[test]
fn test_rfc3977_ihave_command() {
    let interactions = vec![
        // Offer article
        (
            Command::Ihave {
                message_id: "<new_article@example.com>".to_string(),
            },
            Response::ArticleWanted,
        ),
        // In a real scenario, article content would be sent here followed by success
    ];

    let mut test = ClientMockTest::new(interactions);

    let response = test
        .send_command(Command::Ihave {
            message_id: "<new_article@example.com>".to_string(),
        })
        .unwrap();

    if let Response::ArticleWanted = response {
        // Expected - server wants the article
    } else {
        panic!("Expected ArticleWanted response");
    }

    assert!(test.is_complete());
}

/// Test IHAVE rejection as per RFC 3977 Section 6.3.2
#[test]
fn test_rfc3977_ihave_rejection() {
    let interactions = vec![(
        Command::Ihave {
            message_id: "<existing_article@example.com>".to_string(),
        },
        Response::ArticleNotWanted,
    )];

    let mut test = ClientMockTest::new(interactions);

    let response = test
        .send_command(Command::Ihave {
            message_id: "<existing_article@example.com>".to_string(),
        })
        .unwrap();

    if let Response::ArticleNotWanted = response {
        // Expected - server doesn't want the article
    } else {
        panic!("Expected ArticleNotWanted response");
    }

    assert!(test.is_complete());
}

/// Test OVER command as per RFC 3977 Section 8.3
#[test]
fn test_rfc3977_over_command() {
    use nntp_rs::OverviewEntry;

    let interactions = vec![
        // First select a group
        (
            Command::Group("misc.test".to_string()),
            Response::GroupSelected {
                count: 3000,
                first: 3000,
                last: 3002,
                name: "misc.test".to_string(),
            },
        ),
        // Get overview data for range
        (
            Command::Over {
                range: Some("3000-3001".to_string()),
            },
            Response::OverviewData(vec![
                OverviewEntry {
                    number: 3000,
                    subject: "I am just a test article".to_string(),
                    from: "\"Demo User\" <nobody@example.com>".to_string(),
                    date: "6 Oct 1998 04:38:40 -0500".to_string(),
                    message_id: "<45223423@example.com>".to_string(),
                    references: "".to_string(),
                    byte_count: 1234,
                    line_count: 42,
                },
                OverviewEntry {
                    number: 3001,
                    subject: "Another test article".to_string(),
                    from: "\"Another User\" <user@example.com>".to_string(),
                    date: "7 Oct 1998 05:39:41 -0500".to_string(),
                    message_id: "<45223424@example.com>".to_string(),
                    references: "<45223423@example.com>".to_string(),
                    byte_count: 2345,
                    line_count: 56,
                },
            ]),
        ),
    ];

    let mut test = ClientMockTest::new(interactions);

    // Select group first
    test.send_command(Command::Group("misc.test".to_string()))
        .unwrap();

    // Test OVER command
    let response = test
        .send_command(Command::Over {
            range: Some("3000-3001".to_string()),
        })
        .unwrap();

    if let Response::OverviewData(overview) = response {
        assert_eq!(overview.len(), 2);
        assert_eq!(overview[0].number, 3000);
        assert_eq!(overview[0].subject, "I am just a test article");
        assert_eq!(overview[0].message_id, "<45223423@example.com>");
        assert_eq!(overview[1].number, 3001);
        assert_eq!(overview[1].subject, "Another test article");
        assert_eq!(overview[1].references, "<45223423@example.com>");
    } else {
        panic!("Expected OverviewData response");
    }

    assert!(test.is_complete());
}

/// Test HDR command as per RFC 3977 Section 8.5
#[test]
fn test_rfc3977_hdr_command() {
    use nntp_rs::HeaderEntry;

    let interactions = vec![
        // First select a group
        (
            Command::Group("misc.test".to_string()),
            Response::GroupSelected {
                count: 3000,
                first: 3000,
                last: 3002,
                name: "misc.test".to_string(),
            },
        ),
        // Get Subject headers for range
        (
            Command::Hdr {
                field: "Subject".to_string(),
                range: Some("3000-3002".to_string()),
            },
            Response::HeaderData(vec![
                HeaderEntry {
                    article: "3000".to_string(),
                    value: "I am just a test article".to_string(),
                },
                HeaderEntry {
                    article: "3001".to_string(),
                    value: "Another test article".to_string(),
                },
                HeaderEntry {
                    article: "3002".to_string(),
                    value: "Yet another test".to_string(),
                },
            ]),
        ),
    ];

    let mut test = ClientMockTest::new(interactions);

    // Select group first
    test.send_command(Command::Group("misc.test".to_string()))
        .unwrap();

    // Test HDR command
    let response = test
        .send_command(Command::Hdr {
            field: "Subject".to_string(),
            range: Some("3000-3002".to_string()),
        })
        .unwrap();

    if let Response::HeaderData(headers) = response {
        assert_eq!(headers.len(), 3);
        assert_eq!(headers[0].article, "3000");
        assert_eq!(headers[0].value, "I am just a test article");
        assert_eq!(headers[1].article, "3001");
        assert_eq!(headers[1].value, "Another test article");
        assert_eq!(headers[2].article, "3002");
        assert_eq!(headers[2].value, "Yet another test");
    } else {
        panic!("Expected HeaderData response");
    }

    assert!(test.is_complete());
}

/// Test LAST command as per RFC 3977 Section 6.1.4
#[test]
fn test_rfc3977_last_command() {
    let interactions = vec![
        // First select a group
        (
            Command::Group("misc.test".to_string()),
            Response::GroupSelected {
                count: 3000,
                first: 3000,
                last: 3002,
                name: "misc.test".to_string(),
            },
        ),
        // Move to last article
        (
            Command::Last,
            Response::ArticleStatus {
                number: 2999,
                message_id: "<previous@example.com>".to_string(),
            },
        ),
    ];

    let mut test = ClientMockTest::new(interactions);

    // Select group first
    test.send_command(Command::Group("misc.test".to_string()))
        .unwrap();

    // Test LAST command
    let response = test.send_command(Command::Last).unwrap();
    if let Response::ArticleStatus { number, message_id } = response {
        assert_eq!(number, 2999);
        assert_eq!(message_id, "<previous@example.com>");
    } else {
        panic!("Expected ArticleStatus response");
    }

    assert!(test.is_complete());
}

/// Test NEXT command as per RFC 3977 Section 6.1.3
#[test]
fn test_rfc3977_next_command() {
    let interactions = vec![
        // First select a group
        (
            Command::Group("misc.test".to_string()),
            Response::GroupSelected {
                count: 3000,
                first: 3000,
                last: 3002,
                name: "misc.test".to_string(),
            },
        ),
        // Move to next article
        (
            Command::Next,
            Response::ArticleStatus {
                number: 3001,
                message_id: "<next@example.com>".to_string(),
            },
        ),
    ];

    let mut test = ClientMockTest::new(interactions);

    // Select group first
    test.send_command(Command::Group("misc.test".to_string()))
        .unwrap();

    // Test NEXT command
    let response = test.send_command(Command::Next).unwrap();
    if let Response::ArticleStatus { number, message_id } = response {
        assert_eq!(number, 3001);
        assert_eq!(message_id, "<next@example.com>");
    } else {
        panic!("Expected ArticleStatus response");
    }

    assert!(test.is_complete());
}

/// Test DATE command as per RFC 3977 Section 7.1
#[test]
fn test_rfc3977_date_command() {
    let interactions = vec![(
        Command::Date,
        Response::Date("20231106123456".to_string()),
    )];

    let mut test = ClientMockTest::new(interactions);

    let response = test.send_command(Command::Date).unwrap();
    if let Response::Date(date) = response {
        assert_eq!(date.len(), 14); // YYYYMMDDHHMMSS format
        assert!(date.chars().all(|c| c.is_ascii_digit()));
    } else {
        panic!("Expected Date response");
    }

    assert!(test.is_complete());
}

/// Test HELP command as per RFC 3977 Section 7.4.1
#[test]
fn test_rfc3977_help_command() {
    let interactions = vec![(
        Command::Help,
        Response::Help(vec![
            "This server accepts the following commands:".to_string(),
            "ARTICLE BODY CAPABILITIES GROUP HEAD HELP".to_string(),
            "IHAVE LAST LIST LISTGROUP MODE NEWGROUPS".to_string(),
            "NEWNEWS NEXT POST QUIT STAT".to_string(),
            "".to_string(),
            "For more information, consult RFC 3977.".to_string(),
        ]),
    )];

    let mut test = ClientMockTest::new(interactions);

    let response = test.send_command(Command::Help).unwrap();
    if let Response::Help(help_lines) = response {
        assert!(!help_lines.is_empty());
        assert!(help_lines[0].contains("server accepts"));
        assert!(help_lines.iter().any(|line| line.contains("CAPABILITIES")));
        assert!(help_lines.iter().any(|line| line.contains("RFC 3977")));
    } else {
        panic!("Expected Help response");
    }

    assert!(test.is_complete());
}

/// Test complete session workflow as per RFC3977 examples
#[test]
fn test_rfc3977_complete_session() {
    let interactions = vec![
        // Initial capabilities
        (
            Command::Capabilities,
            Response::Capabilities(vec![
                "VERSION 2".to_string(),
                "READER".to_string(),
                "POST".to_string(),
            ]),
        ),
        // Switch to reader mode
        (
            Command::ModeReader,
            Response::ModeReader {
                posting_allowed: true,
            },
        ),
        // Select a group
        (
            Command::Group("misc.test".to_string()),
            Response::GroupSelected {
                count: 3000,
                first: 3000,
                last: 3002,
                name: "misc.test".to_string(),
            },
        ),
        // Get article status
        (
            Command::Stat(ArticleSpec::Number(3000)),
            Response::ArticleStatus {
                number: 3000,
                message_id: "<45223423@example.com>".to_string(),
            },
        ),
        // Retrieve article headers
        (
            Command::Head(ArticleSpec::Number(3000)),
            Response::Article {
                number: Some(3000),
                message_id: "<45223423@example.com>".to_string(),
                content: b"From: \"Demo User\" <nobody@example.com>\r\nSubject: I am just a test article\r\n".to_vec(),
            },
        ),
        // Quit
        (
            Command::Quit,
            Response::Quit,
        ),
    ];

    let mut test = ClientMockTest::new(interactions);

    // Test complete workflow
    test.send_command(Command::Capabilities).unwrap();
    assert_eq!(test.client().state(), "reader");

    test.send_command(Command::ModeReader).unwrap();
    assert_eq!(test.client().state(), "reader");

    test.send_command(Command::Group("misc.test".to_string()))
        .unwrap();
    assert_eq!(test.client().state(), "group_selected");

    test.send_command(Command::Stat(ArticleSpec::Number(3000)))
        .unwrap();

    test.send_command(Command::Head(ArticleSpec::Number(3000)))
        .unwrap();

    test.send_command(Command::Quit).unwrap();
    assert_eq!(test.client().state(), "closed");

    assert!(test.is_complete());
}
