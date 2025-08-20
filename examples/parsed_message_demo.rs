//! Example demonstrating the new parsed message functionality in runtime integrations.
//!
//! This example shows how to use the new `article_parsed()`, `head_parsed()`, and `body_parsed()`
//! methods as well as the convenience methods like `article_subject()`, `article_from()`,
//! and `article_body_text()` in the async runtime integrations.

use nntp_rs::Response;

#[cfg(feature = "tokio-runtime")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Parsed Message Demo");
    println!("==================");

    // This example demonstrates the new parsing capabilities without requiring
    // an actual NNTP server connection by using mock responses.

    // Create a sample article response for demonstration
    let sample_email = b"Subject: I am just a test article\r\nFrom: nobody@example.com\r\nDate: 6 Oct 1998 04:38:40 GMT\r\n\r\nThis is just a test article body.\r\nWith multiple lines of content.\r\n";

    let article_response = Response::Article {
        number: Some(3000),
        message_id: "<45223423@example.com>".to_string(),
        content: sample_email.to_vec(),
    };

    // Traditional raw content access (backwards compatibility)
    if let Response::Article { content, .. } = &article_response {
        let content_str = String::from_utf8_lossy(content);
        println!("Raw content access:");
        println!("{}", content_str);
        println!();
    }

    // New parsed message functionality
    println!("Parsed content access:");

    // Get parsed subject
    if let Some(subject) = article_response.article_subject() {
        println!("Subject: {}", subject);
    }

    // Get sender email
    if let Some(from) = article_response.article_from() {
        println!("From: {}", from);
    }

    // Get body text
    if let Some(body) = article_response.article_body() {
        println!("Body: {}", body);
    }

    // Access full parsed message for advanced use cases
    if let Some(message) = article_response.parsed_message() {
        println!("\nAdvanced parsing:");
        println!("Message-ID: {:?}", message.message_id());
        if let Some(date) = message.date() {
            println!("Date: {:?}", date);
        }
    }

    println!("\nRuntime Integration Demo:");
    println!("The runtime integrations (tokio, async_std, smol) now provide:");
    println!("- article_parsed() -> Returns Response with parsing methods");
    println!("- article_subject() -> Returns Option<String> with subject");
    println!("- article_from() -> Returns Option<String> with sender email");
    println!("- article_body_text() -> Returns Option<String> with body text");
    println!("- head_parsed() -> Returns Response for headers-only parsing");
    println!("- body_parsed() -> Returns Response for body-only parsing");

    Ok(())
}

#[cfg(not(feature = "tokio-runtime"))]
fn main() {
    println!("This example requires the tokio-runtime feature.");
    println!("Run with: cargo run --example parsed_message_demo --features tokio-runtime");
}
