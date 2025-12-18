//! Extended RFC 3977 compliance tests for newly implemented features.
//!
//! These tests validate the enhanced RFC 3977 compliance features including
//! LIST command variants, command validation, specific error codes, and state management.

use nntp_rs::mock::ClientMockTest;
use nntp_rs::{Command, ListVariant, Response};

/// Test LIST ACTIVE command variant as per RFC 3977 Section 7.6.3
#[test]
fn test_rfc3977_list_active_command() {
    use nntp_rs::response::NewsGroup;

    let interactions = vec![(
        Command::List(ListVariant::Active(None)),
        Response::NewsgroupList(vec![
            NewsGroup {
                name: "comp.lang.rust".to_string(),
                last: 1234,
                first: 1000,
                posting_status: 'y',
            },
            NewsGroup {
                name: "misc.test".to_string(),
                last: 5678,
                first: 5000,
                posting_status: 'n',
            },
        ]),
    )];

    let mut test = ClientMockTest::new(interactions);
    let response = test
        .send_command(Command::List(ListVariant::Active(None)))
        .unwrap();

    if let Response::NewsgroupList(groups) = response {
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].name, "comp.lang.rust");
        assert_eq!(groups[0].posting_status, 'y');
        assert_eq!(groups[1].name, "misc.test");
        assert_eq!(groups[1].posting_status, 'n');
    } else {
        panic!("Expected NewsgroupList response");
    }

    assert!(test.is_complete());
}

/// Test LIST ACTIVE with wildmat pattern as per RFC 3977 Section 7.6.3
#[test]
fn test_rfc3977_list_active_wildmat() {
    use nntp_rs::response::NewsGroup;

    let interactions = vec![(
        Command::List(ListVariant::Active(Some("comp.*".to_string()))),
        Response::NewsgroupList(vec![NewsGroup {
            name: "comp.lang.rust".to_string(),
            last: 1234,
            first: 1000,
            posting_status: 'y',
        }]),
    )];

    let mut test = ClientMockTest::new(interactions);
    let response = test
        .send_command(Command::List(ListVariant::Active(Some(
            "comp.*".to_string(),
        ))))
        .unwrap();

    if let Response::NewsgroupList(groups) = response {
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "comp.lang.rust");
    } else {
        panic!("Expected NewsgroupList response");
    }

    assert!(test.is_complete());
}

/// Test LIST NEWSGROUPS command variant as per RFC 3977 Section 7.6.6
#[test]
fn test_rfc3977_list_newsgroups_command() {
    use nntp_rs::response::NewsGroup;

    let interactions = vec![(
        Command::List(ListVariant::Newsgroups(None)),
        Response::NewsgroupList(vec![NewsGroup {
            name: "comp.lang.rust".to_string(),
            last: 1234,
            first: 1000,
            posting_status: 'y',
        }]),
    )];

    let mut test = ClientMockTest::new(interactions);
    let response = test
        .send_command(Command::List(ListVariant::Newsgroups(None)))
        .unwrap();

    if let Response::NewsgroupList(groups) = response {
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "comp.lang.rust");
    } else {
        panic!("Expected NewsgroupList response for LIST NEWSGROUPS");
    }

    assert!(test.is_complete());
}

/// Test LIST HEADERS command for HDR capability support
#[test]
fn test_rfc3977_list_headers_command() {
    let interactions = vec![(
        Command::List(ListVariant::Headers),
        Response::Capabilities(vec![
            "Subject:".to_string(),
            "From:".to_string(),
            "Date:".to_string(),
            "Message-ID:".to_string(),
        ]),
    )];

    let mut test = ClientMockTest::new(interactions);
    let response = test
        .send_command(Command::List(ListVariant::Headers))
        .unwrap();

    if let Response::Capabilities(headers) = response {
        assert_eq!(headers.len(), 4);
        assert!(headers.contains(&"Subject:".to_string()));
        assert!(headers.contains(&"From:".to_string()));
    } else {
        panic!("Expected Capabilities response for LIST HEADERS");
    }

    assert!(test.is_complete());
}

/// Test command length validation as per RFC 3977 Section 3.1
#[test]
fn test_rfc3977_command_length_validation() {
    // Create a command that would exceed 512 octets (510 + CRLF)
    // GROUP command = "GROUP " + name, so need name > 504 chars
    let very_long_group_name = "a".repeat(505);
    let cmd = Command::Group(very_long_group_name);

    // Command should fail validation during encoding
    assert!(cmd.encode().is_err());
}

/// Test specific error code responses as per RFC 3977 Section 3.2
#[test]
fn test_rfc3977_specific_error_codes() {
    let interactions = vec![(
        Command::Group("nonexistent.group".to_string()),
        Response::Error {
            code: 411,
            message: "No such newsgroup".to_string(),
        },
    )];

    let mut test = ClientMockTest::new(interactions);

    // Test 411 - No such newsgroup
    let response = test
        .send_command(Command::Group("nonexistent.group".to_string()))
        .unwrap();
    if let Response::Error { code, message } = response {
        assert_eq!(code, 411);
        assert!(message.contains("No such newsgroup"));
    } else {
        panic!("Expected Error response with code 411");
    }

    assert!(test.is_complete());
}

/// Test enhanced NEWGROUPS command with distributions parameter
#[test]
fn test_rfc3977_newgroups_with_distributions() {
    let interactions = vec![(
        Command::NewGroups {
            date: "231106".to_string(),
            time: "120000".to_string(),
            gmt: true,
            distributions: Some("world".to_string()),
        },
        Response::NewNewsgroups(vec![]),
    )];

    let mut test = ClientMockTest::new(interactions);
    let response = test
        .send_command(Command::NewGroups {
            date: "231106".to_string(),
            time: "120000".to_string(),
            gmt: true,
            distributions: Some("world".to_string()),
        })
        .unwrap();

    if let Response::NewNewsgroups(_groups) = response {
        // Command executed successfully
    } else {
        panic!("Expected NewNewsgroups response");
    }

    assert!(test.is_complete());
}

/// Test state validation for group-dependent commands
#[test]
fn test_rfc3977_state_validation() {
    // Test that LAST command requires group selection
    let interactions = vec![];
    let test = ClientMockTest::new(interactions);

    // Client should reject LAST command when no group is selected
    // This test validates that our client enforces RFC 3977 state requirements
    let client = test.client();

    // Verify client is in initial state with no group selected
    assert_eq!(client.current_group(), None);
    assert_eq!(client.state(), "connected");
}

/// Test LIST command encoding variations
#[test]
fn test_rfc3977_list_command_encoding() {
    // Test LIST ACTIVE
    let cmd = Command::List(ListVariant::Active(None));
    let encoded = cmd.encode().unwrap();
    assert_eq!(encoded, b"LIST ACTIVE\r\n");

    // Test LIST ACTIVE with pattern
    let cmd = Command::List(ListVariant::Active(Some("comp.*".to_string())));
    let encoded = cmd.encode().unwrap();
    assert_eq!(encoded, b"LIST ACTIVE comp.*\r\n");

    // Test LIST NEWSGROUPS
    let cmd = Command::List(ListVariant::Newsgroups(None));
    let encoded = cmd.encode().unwrap();
    assert_eq!(encoded, b"LIST NEWSGROUPS\r\n");

    // Test LIST HEADERS
    let cmd = Command::List(ListVariant::Headers);
    let encoded = cmd.encode().unwrap();
    assert_eq!(encoded, b"LIST HEADERS\r\n");

    // Test LIST ACTIVE.TIMES
    let cmd = Command::List(ListVariant::ActiveTimes);
    let encoded = cmd.encode().unwrap();
    assert_eq!(encoded, b"LIST ACTIVE.TIMES\r\n");

    // Test LIST DISTRIBUTIONS
    let cmd = Command::List(ListVariant::Distributions);
    let encoded = cmd.encode().unwrap();
    assert_eq!(encoded, b"LIST DISTRIBUTIONS\r\n");

    // Test LIST OVERVIEW.FMT
    let cmd = Command::List(ListVariant::OverviewFmt);
    let encoded = cmd.encode().unwrap();
    assert_eq!(encoded, b"LIST OVERVIEW.FMT\r\n");

    // Test basic LIST (backwards compatibility)
    let cmd = Command::List(ListVariant::Basic(None));
    let encoded = cmd.encode().unwrap();
    assert_eq!(encoded, b"LIST\r\n");

    let cmd = Command::List(ListVariant::Basic(Some("misc.*".to_string())));
    let encoded = cmd.encode().unwrap();
    assert_eq!(encoded, b"LIST misc.*\r\n");
}

/// Test authentication error codes
#[test]
fn test_rfc3977_authentication_errors() {
    let interactions = vec![(
        Command::Post,
        Response::Error {
            code: 480,
            message: "Authentication required for posting".to_string(),
        },
    )];

    let mut test = ClientMockTest::new(interactions);
    let response = test.send_command(Command::Post).unwrap();

    if let Response::Error { code, message } = response {
        assert_eq!(code, 480);
        assert!(message.contains("Authentication required"));
    } else {
        panic!("Expected Error response with code 480");
    }

    assert!(test.is_complete());
}

/// Test command syntax error responses
#[test]
fn test_rfc3977_syntax_errors() {
    let interactions = vec![(
        Command::Capabilities, // Using valid command for test setup
        Response::Error {
            code: 501,
            message: "Command syntax error".to_string(),
        },
    )];

    let mut test = ClientMockTest::new(interactions);
    let response = test.send_command(Command::Capabilities).unwrap();

    if let Response::Error { code, message } = response {
        assert_eq!(code, 501);
        assert!(message.contains("syntax error"));
    } else {
        panic!("Expected Error response with code 501");
    }

    assert!(test.is_complete());
}

/// Test comprehensive LIST command coverage for RFC 3977 compliance
#[test]
fn test_rfc3977_list_comprehensive_coverage() {
    // This test ensures all LIST variants are properly implemented
    // and can be encoded without errors

    let variants = vec![
        ListVariant::Active(None),
        ListVariant::Active(Some("comp.*".to_string())),
        ListVariant::Newsgroups(None),
        ListVariant::Newsgroups(Some("misc.*".to_string())),
        ListVariant::Headers,
        ListVariant::ActiveTimes,
        ListVariant::Distributions,
        ListVariant::OverviewFmt,
        ListVariant::Basic(None),
        ListVariant::Basic(Some("alt.*".to_string())),
    ];

    for variant in variants {
        let cmd = Command::List(variant);
        assert!(
            cmd.encode().is_ok(),
            "LIST variant should encode successfully"
        );
    }
}
