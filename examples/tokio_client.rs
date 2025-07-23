//! Example demonstrating Tokio integration usage of nntp-rs.
//!
//! This example shows how to use the high-level Tokio client
//! to connect to an NNTP server.
//!
//! Note: This example requires an NNTP server to connect to.
//! Uncomment and modify the server address to test with a real server.

#[cfg(feature = "tokio-runtime")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[allow(unused_imports)]
    use nntp_rs::tokio::NntpClient;

    println!("=== NNTP Tokio Example ===\n");

    // Note: Replace with a real NNTP server for testing
    // let server = "news.example.com:119";

    println!("This example requires a real NNTP server to connect to.");
    println!("To test with a real server:");
    println!("1. Uncomment the connection code below");
    println!("2. Replace 'news.example.com:119' with a real server");
    println!("3. Run: cargo run --example tokio_client --features tokio-runtime");

    /*
    println!("Connecting to NNTP server at {}...", server);
    let mut client = NntpClient::connect(server).await?;

    println!("Connected! Requesting capabilities...");
    let capabilities = client.capabilities().await?;
    println!("Server capabilities:");
    for cap in capabilities {
        println!("  - {}", cap);
    }

    println!("\nSwitching to reader mode...");
    let posting_allowed = client.mode_reader().await?;
    println!("Reader mode active. Posting allowed: {}", posting_allowed);

    // Example: Select a newsgroup
    println!("\nSelecting newsgroup 'comp.lang.rust'...");
    match client.group("comp.lang.rust").await {
        Ok((count, first, last)) => {
            println!("Selected group: {} articles, range {}-{}", count, first, last);
        }
        Err(e) => {
            println!("Failed to select group: {}", e);
        }
    }

    println!("\nClosing connection...");
    client.quit().await?;
    */

    println!("\n=== Tokio Example Complete ===");
    Ok(())
}

#[cfg(not(feature = "tokio-runtime"))]
fn main() {
    println!("This example requires the 'tokio-runtime' feature.");
    println!("Run with: cargo run --example tokio_client --features tokio-runtime");
}
