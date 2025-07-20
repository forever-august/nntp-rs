//! Error types for the NNTP client library.

use std::fmt;

/// Result type used throughout the library.
pub type Result<T> = std::result::Result<T, Error>;

/// Error types that can occur when using the NNTP client.
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    /// Invalid response from server
    InvalidResponse(String),

    /// Protocol error (4xx or 5xx response codes)
    Protocol {
        /// Response code from server
        code: u16,
        /// Response message from server  
        message: String,
    },

    /// Parse error when decoding server responses
    Parse(String),

    /// Invalid command or parameters
    InvalidCommand(String),

    /// I/O error (when using runtime integrations)
    #[cfg(any(
        feature = "tokio-runtime",
        feature = "async-std-runtime",
        feature = "smol-runtime"
    ))]
    Io(String),

    /// Connection error
    #[cfg(any(
        feature = "tokio-runtime",
        feature = "async-std-runtime",
        feature = "smol-runtime"
    ))]
    Connection(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidResponse(msg) => write!(f, "Invalid response: {msg}"),
            Error::Protocol { code, message } => write!(f, "Protocol error {code}: {message}"),
            Error::Parse(msg) => write!(f, "Parse error: {msg}"),
            Error::InvalidCommand(msg) => write!(f, "Invalid command: {msg}"),
            #[cfg(any(
                feature = "tokio-runtime",
                feature = "async-std-runtime",
                feature = "smol-runtime"
            ))]
            Error::Io(msg) => write!(f, "I/O error: {msg}"),
            #[cfg(any(
                feature = "tokio-runtime",
                feature = "async-std-runtime",
                feature = "smol-runtime"
            ))]
            Error::Connection(msg) => write!(f, "Connection error: {msg}"),
        }
    }
}

impl std::error::Error for Error {}

#[cfg(any(
    feature = "tokio-runtime",
    feature = "async-std-runtime",
    feature = "smol-runtime"
))]
impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err.to_string())
    }
}
