//! Example demonstrating the mock server for testing NNTP client implementations.
//!
//! This example shows how to use the mock server to simulate NNTP server responses
//! and test client behavior in isolation.

use nntp_rs::mock::ClientMockTest;
use nntp_rs::{ArticleSpec, Command, Response};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== NNTP Mock Server Example ===\n");

    // Define a sequence of expected request/response pairs
    let interactions = vec![
        // Client requests capabilities
        (
            Command::Capabilities,
            Response::Capabilities(vec![
                "VERSION 2".to_string(),
                "READER".to_string(),
                "POST".to_string(),
            ]),
        ),
        // Client switches to reader mode
        (
            Command::ModeReader,
            Response::ModeReader {
                posting_allowed: true,
            },
        ),
        // Client selects a newsgroup
        (
            Command::Group("comp.lang.rust".to_string()),
            Response::GroupSelected {
                count: 42,
                first: 1,
                last: 42,
                name: "comp.lang.rust".to_string(),
            },
        ),
        // Client retrieves an article
        (
            Command::Article(ArticleSpec::Number(1)),
            Response::Article {
                number: Some(1),
                message_id: "<test@example.com>".to_string(),
                content: b"From: user@example.com\r\nSubject: Hello World\r\n\r\nThis is a test message.\r\n".to_vec(),
            },
        ),
    ];

    // Create a test environment with the mock server
    let mut test = ClientMockTest::new(interactions);

    println!("1. Testing capabilities exchange:");
    let response = test.send_command(Command::Capabilities)?;
    match response {
        Response::Capabilities(caps) => {
            println!("Server capabilities: {caps:?}");
        }
        _ => println!("Unexpected response"),
    }

    println!("\n2. Testing mode reader:");
    let response = test.send_command(Command::ModeReader)?;
    match response {
        Response::ModeReader { posting_allowed } => {
            println!("Reader mode enabled, posting allowed: {posting_allowed}");
        }
        _ => println!("Unexpected response"),
    }

    println!("Client state: {}", test.client().state());

    println!("\n3. Testing group selection:");
    let response = test.send_command(Command::Group("comp.lang.rust".to_string()))?;
    match response {
        Response::GroupSelected {
            name,
            count,
            first,
            last,
        } => {
            println!("Selected group '{name}': {count} articles ({first}-{last})");
        }
        _ => println!("Unexpected response"),
    }

    println!("Current group: {:?}", test.client().current_group());

    println!("\n4. Testing article retrieval:");
    let response = test.send_command(Command::Article(ArticleSpec::Number(1)))?;
    match response {
        Response::Article {
            number,
            message_id,
            content,
        } => {
            println!("Retrieved article {} ({})", number.unwrap_or(0), message_id);
            let content_str = String::from_utf8_lossy(&content);
            println!(
                "Content preview: {}",
                content_str.lines().next().unwrap_or("")
            );
        }
        _ => println!("Unexpected response"),
    }

    println!("\n5. Verifying all interactions completed:");
    if test.is_complete() {
        println!("✓ All expected interactions completed successfully!");
    } else {
        println!("✗ {} interactions remaining", test.remaining_interactions());
    }

    println!("\n=== Mock Server Example Complete ===");
    Ok(())
}
