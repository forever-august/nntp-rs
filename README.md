# nntp-rs

[![Crates.io](https://img.shields.io/crates/v/nntp-rs.svg)](https://crates.io/crates/nntp-rs)
[![Documentation](https://docs.rs/nntp-rs/badge.svg)](https://docs.rs/nntp-rs)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Build Status](https://github.com/forever-august/nntp-rs/workflows/CI/badge.svg)](https://github.com/forever-august/nntp-rs/actions)

A modern, sans-io NNTP (Network News Transfer Protocol) client library for Rust.

## Features

- **Sans-IO Design**: Protocol logic is separated from I/O operations, allowing you to use any async runtime or transport
- **Async Runtime Agnostic**: Optional integrations with popular async runtimes (Tokio, async-std, smol)
- **Type-Safe**: Leverages Rust's type system to provide a safe and ergonomic API
- **RFC 3977 Compliant**: Implements the NNTP protocol as specified in RFC 3977
- **Extensible**: Support for NNTP extensions and custom commands

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
nntp-rs = "0.1"

# For Tokio integration
nntp-rs = { version = "0.1", features = ["tokio-runtime"] }

# For async-std integration  
nntp-rs = { version = "0.1", features = ["async-std-runtime"] }

# For smol integration
nntp-rs = { version = "0.1", features = ["smol-runtime"] }
```

## Usage

### Sans-IO Usage (Manual I/O handling)

```rust
use nntp_rs::{Client, Command, Response};

// Create a client instance
let mut client = Client::new();

// Build a command
let command = Command::Capabilities;
let request = client.encode_command(command)?;

// Send request through your I/O layer
// ... your networking code ...

// Parse response from your I/O layer  
let response = client.decode_response(&response_bytes)?;
match response {
    Response::Capabilities(caps) => {
        println!("Server capabilities: {:?}", caps);
    }
    _ => {}
}
```

### With Tokio Integration

```rust
use nntp_rs::tokio::NntpClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = NntpClient::connect("news.example.com:119").await?;
    
    let capabilities = client.capabilities().await?;
    println!("Server capabilities: {:?}", capabilities);
    
    Ok(())
}
```

## Sans-IO Design

This library follows the sans-io design pattern, which means:

1. **Protocol Logic**: The core library handles NNTP protocol parsing and generation
2. **I/O Separation**: Network I/O is handled separately, allowing integration with any async runtime
3. **Flexibility**: You can use the library with custom transports, testing frameworks, or any I/O model

## Supported NNTP Commands

- [x] CAPABILITIES
- [x] MODE READER  
- [x] AUTHINFO USER/PASS
- [x] GROUP
- [x] LISTGROUP
- [x] ARTICLE
- [x] HEAD
- [x] BODY
- [x] STAT
- [x] LIST
- [x] NEWGROUPS
- [x] NEWNEWS
- [x] POST
- [x] QUIT
- [x] HELP
- [x] DATE
- [x] LAST
- [x] NEXT
- [x] HDR
- [x] OVER
- [x] IHAVE

## Testing and Compliance

nntp-rs includes comprehensive testing infrastructure to help validate your NNTP client implementations:

### Mock Server for Testing

The library provides a mock server for testing NNTP client logic without requiring a real server:

```rust
use nntp_rs::mock::ClientMockTest;
use nntp_rs::{Command, Response};

// Define expected interactions
let interactions = vec![
    (Command::Capabilities, Response::Capabilities(vec!["VERSION 2".to_string()])),
    (Command::ModeReader, Response::ModeReader { posting_allowed: true }),
];

// Create test environment
let mut test = ClientMockTest::new(interactions);

// Test your client logic
let response = test.send_command(Command::Capabilities)?;
// ... assertions ...
```

### RFC 3977 Compliance Tests

The library includes comprehensive compliance tests based on RFC 3977 examples, covering:

- Basic connection and capabilities exchange
- Reader mode switching
- Group selection and navigation
- Article retrieval (ARTICLE, HEAD, BODY, STAT)
- Newsgroup listing
- Error handling
- Complete session workflows

Run compliance tests with:
```bash
cargo test rfc3977
```

See `examples/mock_server.rs` for a complete demonstration of testing capabilities.

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.