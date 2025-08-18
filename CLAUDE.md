# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

### Build and Test
```bash
# Standard build
cargo build

# Build with specific runtime features
cargo build --features tokio-runtime
cargo build --features async-std-runtime  
cargo build --features smol-runtime
cargo build --features all-runtimes

# Run all tests
cargo test

# Run tests with specific features
cargo test --features tokio-runtime
cargo test --features all-runtimes

# Run RFC 3977 compliance tests specifically
cargo test rfc3977
```

### Code Quality
```bash
# Format code
cargo fmt

# Check formatting
cargo fmt --all -- --check

# Run clippy (linting)
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --features all-runtimes -- -D warnings

# Build documentation
cargo doc --no-deps --all-features

# Coverage analysis (requires cargo-tarpaulin)
cargo tarpaulin --verbose --all-features --workspace --timeout 120
```

### Examples
```bash
# Run examples
cargo run --example tokio_client --features tokio-runtime
cargo run --example sans_io
cargo run --example mock_server
```

## Architecture Overview

This is a sans-io NNTP (Network News Transfer Protocol) client library following a modular design:

### Core Sans-IO Design
- **Protocol Logic Separation**: Core NNTP protocol implementation (`client.rs`) handles parsing and generation without performing I/O
- **Runtime Agnostic**: Optional async runtime integrations (`tokio.rs`, `async_std.rs`, `smol.rs`) provide high-level interfaces
- **Stateful Client**: The `Client` struct maintains connection state and protocol compliance

### Key Components

- **`client.rs`**: Core sans-io client with state management (`Client` struct)
  - Encodes commands to bytes for transmission
  - Decodes server responses from received bytes
  - Maintains connection state (Connected, Reader, Authenticated, GroupSelected, etc.)
  - Handles both single-line and multi-line NNTP responses

- **`command.rs`**: NNTP command types and encoding
  - `Command` enum covering all supported NNTP commands
  - `ArticleSpec` for specifying articles by number or message-ID
  - Command serialization to protocol format

- **`response.rs`**: NNTP response types and parsing
  - `Response` enum for all server response types
  - Handles multi-line responses (capabilities, article listings, etc.)
  - Response parsing from raw protocol data

- **`error.rs`**: Error types for protocol and I/O failures

- **`mock.rs`**: Testing infrastructure with `ClientMockTest` for unit testing NNTP client logic

### Runtime Integrations
Optional async runtime modules provide high-level client interfaces:
- Each runtime module (`tokio`, `async_std`, `smol`) wraps the core sans-io client
- Provides `connect()` methods and async command execution
- Handles network I/O using the respective async runtime

### State Management
The client maintains protocol state through `ClientState` enum:
- Tracks authentication status
- Manages group selection
- Enforces command sequencing per NNTP protocol

### Testing Strategy
- RFC 3977 compliance tests in `tests/rfc3977_compliance.rs`
- Extended RFC 3977 compliance tests in `tests/rfc3977_extended_compliance.rs`
- Mock server testing via `ClientMockTest`
- Examples demonstrating both sans-io and runtime-integrated usage

## Usage Examples

### LIST Command Variants
```rust
use nntp_rs::{Command, ListVariant};

// List active newsgroups
let cmd = Command::List(ListVariant::Active(None));

// List active newsgroups matching pattern
let cmd = Command::List(ListVariant::Active(Some("comp.*".to_string())));

// List newsgroup descriptions
let cmd = Command::List(ListVariant::Newsgroups(None));

// List available header fields for HDR command
let cmd = Command::List(ListVariant::Headers);

// List overview format
let cmd = Command::List(ListVariant::OverviewFmt);
```

### Enhanced NEWGROUPS Command
```rust
use nntp_rs::Command;

// NEWGROUPS with distributions parameter
let cmd = Command::NewGroups {
    date: "231106".to_string(),
    time: "120000".to_string(),
    gmt: true,
    distributions: Some("world".to_string()),
};
```

## Feature Flags
- `tokio-runtime`: Enables Tokio integration
- `async-std-runtime`: Enables async-std integration  
- `smol-runtime`: Enables smol integration
- `all-runtimes`: Enables all runtime integrations

## NNTP Protocol Support
Implements comprehensive NNTP command set with **enhanced RFC 3977 compliance**:

### Core Commands
- Connection management (CAPABILITIES, MODE READER, QUIT)
- Authentication (AUTHINFO USER/PASS)
- Group operations (GROUP, LISTGROUP)
- Article retrieval (ARTICLE, HEAD, BODY, STAT)
- Navigation (LAST, NEXT)
- Metadata (HDR, OVER, DATE, HELP)
- Posting (POST, IHAVE)
- News discovery (NEWGROUPS, NEWNEWS)

### Enhanced LIST Command Support (RFC 3977 Section 7.6)
- `LIST ACTIVE [wildmat]` - Active newsgroups with posting status
- `LIST NEWSGROUPS [wildmat]` - Newsgroup descriptions  
- `LIST HEADERS` - Available header fields for HDR command
- `LIST ACTIVE.TIMES` - Newsgroup creation times
- `LIST DISTRIBUTIONS` - Distribution values
- `LIST OVERVIEW.FMT` - Overview format specification
- `LIST [wildmat]` - Basic list (backwards compatibility)

### RFC 3977 Compliance Features
- **Command Length Validation**: Enforces 512-octet limit (RFC 3977 Section 3.1)
- **Specific Error Codes**: Proper handling of standardized error responses:
  - 400 Service discontinued
  - 411 No such newsgroup
  - 412 No newsgroup selected
  - 420 No current article
  - 421/422 No next/previous article
  - 430 No such article
  - 480 Authentication required
  - 500 Command not recognized
  - 501 Command syntax error
  - 502 Access denied
  - 503 Program fault
- **Protocol State Management**: Validates command prerequisites (e.g., group selection for LAST/NEXT)
- **Enhanced NEWGROUPS**: Support for distributions parameter