//! Example demonstrating basic sans-io usage of nntp-rs.
//!
//! This example shows how to use the sans-io client directly,
//! handling I/O operations manually.

use nntp_rs::{Client, Command, Response};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== NNTP Sans-IO Example ===\n");

    // Create a new sans-io client
    let mut client = Client::new();
    println!("Client created in state: {}", client.state());

    // Example 1: Encode commands
    println!("\n1. Encoding commands:");

    let capabilities_cmd = client.encode_command(Command::Capabilities)?;
    println!(
        "CAPABILITIES command: {:?}",
        std::str::from_utf8(&capabilities_cmd)?
    );

    let group_cmd = client.encode_command(Command::Group("comp.lang.rust".to_string()))?;
    println!("GROUP command: {:?}", std::str::from_utf8(&group_cmd)?);

    // Example 2: Parse responses
    println!("\n2. Parsing responses:");

    // Simulate server responses
    let capability_response =
        b"101 Capability list:\r\nVERSION 2\r\nREADER\r\nIHAVE\r\nPOST\r\n.\r\n";
    let group_response = b"211 1234 3000 4234 comp.lang.rust\r\n";

    // Feed data to client and decode
    client.feed_bytes(capability_response);
    if let Some(response) = client.decode_response()? {
        match response {
            Response::Capabilities(caps) => {
                println!("Server capabilities:");
                for cap in caps {
                    println!("  - {cap}");
                }
            }
            _ => println!("Unexpected response"),
        }
    }

    client.feed_bytes(group_response);
    if let Some(response) = client.decode_response()? {
        match response {
            Response::GroupSelected {
                count,
                first,
                last,
                name,
            } => {
                println!("Selected group '{name}': {count} articles ({first}-{last})");
                println!("Current group: {:?}", client.current_group());
            }
            _ => println!("Unexpected response"),
        }
    }

    // Example 3: Client state management
    println!("\n3. Client state:");
    println!("Is ready: {}", client.is_ready());
    println!("Is authenticated: {}", client.is_authenticated());
    println!("Current state: {}", client.state());

    println!("\n=== Sans-IO Example Complete ===");
    Ok(())
}
