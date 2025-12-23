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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_invalid_response() {
        let err = Error::InvalidResponse("test message".to_string());
        assert_eq!(format!("{}", err), "Invalid response: test message");
    }

    #[test]
    fn test_error_display_protocol() {
        let err = Error::Protocol {
            code: 411,
            message: "No such newsgroup".to_string(),
        };
        assert_eq!(format!("{}", err), "Protocol error 411: No such newsgroup");
    }

    #[test]
    fn test_error_display_parse() {
        let err = Error::Parse("invalid format".to_string());
        assert_eq!(format!("{}", err), "Parse error: invalid format");
    }

    #[test]
    fn test_error_display_invalid_command() {
        let err = Error::InvalidCommand("bad command".to_string());
        assert_eq!(format!("{}", err), "Invalid command: bad command");
    }

    #[cfg(any(
        feature = "tokio-runtime",
        feature = "async-std-runtime",
        feature = "smol-runtime"
    ))]
    #[test]
    fn test_error_display_io() {
        let err = Error::Io("connection refused".to_string());
        assert_eq!(format!("{}", err), "I/O error: connection refused");
    }

    #[cfg(any(
        feature = "tokio-runtime",
        feature = "async-std-runtime",
        feature = "smol-runtime"
    ))]
    #[test]
    fn test_error_display_connection() {
        let err = Error::Connection("timeout".to_string());
        assert_eq!(format!("{}", err), "Connection error: timeout");
    }

    #[cfg(any(
        feature = "tokio-runtime",
        feature = "async-std-runtime",
        feature = "smol-runtime"
    ))]
    #[test]
    fn test_error_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: Error = io_err.into();
        assert!(matches!(err, Error::Io(_)));
        assert!(format!("{}", err).contains("file not found"));
    }

    #[test]
    fn test_error_is_std_error() {
        let err: &dyn std::error::Error = &Error::Parse("test".to_string());
        assert!(err.to_string().contains("Parse error"));
    }
}
