//! Stream abstraction layer for async runtime integration.
//!
//! This module provides a unified `AsyncStream` trait that abstracts over different
//! async runtime TCP stream implementations. Each runtime has a feature-gated newtype
//! wrapper that implements this trait, allowing the library to work seamlessly with
//! tokio, async-std, or smol.
//!
//! # Example
//!
//! ```rust,no_run
//! # #[cfg(feature = "tokio-runtime")]
//! # {
//! use nntp_rs::runtime::stream::{AsyncStream, TokioStream};
//!
//! # #[tokio::main]
//! # async fn main() -> std::io::Result<()> {
//! let mut stream = TokioStream::connect("news.example.com:119").await?;
//! stream.write_all(b"CAPABILITIES\r\n").await?;
//! let mut buf = [0u8; 1024];
//! let n = stream.read(&mut buf).await?;
//! # Ok(())
//! # }
//! # }
//! ```

use async_trait::async_trait;

/// A unified trait for async TCP streams across different runtimes.
///
/// This trait provides a common interface for connecting to servers and performing
/// async read/write operations, abstracting over the differences between tokio,
/// async-std, and smol TCP stream implementations.
///
/// # Bounds
///
/// Implementations must be `Send + Unpin + 'static` to ensure they can be used
/// safely across async task boundaries and with common async patterns.
#[async_trait]
pub trait AsyncStream: Send + Unpin + 'static {
    /// Establishes a TCP connection to the specified address.
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to connect to, in the format "host:port"
    ///
    /// # Returns
    ///
    /// Returns `Ok(Self)` on successful connection, or an `io::Error` on failure.
    async fn connect(addr: &str) -> std::io::Result<Self>
    where
        Self: Sized;

    /// Reads data from the stream into the provided buffer.
    ///
    /// # Arguments
    ///
    /// * `buf` - The buffer to read data into
    ///
    /// # Returns
    ///
    /// Returns `Ok(n)` where `n` is the number of bytes read, or an `io::Error` on failure.
    /// Returns `Ok(0)` when EOF is reached.
    async fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize>;

    /// Writes all data from the buffer to the stream.
    ///
    /// This method will continue writing until all bytes are written or an error occurs.
    ///
    /// # Arguments
    ///
    /// * `buf` - The data to write
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an `io::Error` on failure.
    async fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()>;

    /// Shuts down the stream.
    ///
    /// This signals that no more data will be written to the stream.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an `io::Error` on failure.
    async fn shutdown(&mut self) -> std::io::Result<()>;
}

// ============================================================================
// Tokio Runtime Implementation
// ============================================================================

/// A newtype wrapper around `tokio::net::TcpStream`.
///
/// This wrapper implements the `AsyncStream` trait for use with the tokio runtime.
#[cfg(feature = "tokio-runtime")]
#[cfg_attr(docsrs, doc(cfg(feature = "tokio-runtime")))]
pub struct TokioStream(pub tokio::net::TcpStream);

#[cfg(feature = "tokio-runtime")]
#[cfg_attr(docsrs, doc(cfg(feature = "tokio-runtime")))]
#[async_trait]
impl AsyncStream for TokioStream {
    async fn connect(addr: &str) -> std::io::Result<Self>
    where
        Self: Sized,
    {
        let stream = tokio::net::TcpStream::connect(addr).await?;
        Ok(TokioStream(stream))
    }

    async fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        use tokio::io::AsyncReadExt;
        self.0.read(buf).await
    }

    async fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        tokio::io::AsyncWriteExt::write_all(&mut self.0, buf).await
    }

    async fn shutdown(&mut self) -> std::io::Result<()> {
        tokio::io::AsyncWriteExt::shutdown(&mut self.0).await
    }
}

// ============================================================================
// Async-std Runtime Implementation
// ============================================================================

/// A newtype wrapper around `async_std::net::TcpStream`.
///
/// This wrapper implements the `AsyncStream` trait for use with the async-std runtime.
#[cfg(feature = "async-std-runtime")]
#[cfg_attr(docsrs, doc(cfg(feature = "async-std-runtime")))]
pub struct AsyncStdStream(pub async_std::net::TcpStream);

#[cfg(feature = "async-std-runtime")]
#[cfg_attr(docsrs, doc(cfg(feature = "async-std-runtime")))]
#[async_trait]
impl AsyncStream for AsyncStdStream {
    async fn connect(addr: &str) -> std::io::Result<Self>
    where
        Self: Sized,
    {
        let stream = async_std::net::TcpStream::connect(addr).await?;
        Ok(AsyncStdStream(stream))
    }

    async fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        async_std::io::ReadExt::read(&mut self.0, buf).await
    }

    async fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        async_std::io::WriteExt::write_all(&mut self.0, buf).await
    }

    async fn shutdown(&mut self) -> std::io::Result<()> {
        // async-std TcpStream has shutdown method
        self.0.shutdown(std::net::Shutdown::Write)?;
        Ok(())
    }
}

// ============================================================================
// Smol Runtime Implementation
// ============================================================================

/// A newtype wrapper around `smol::net::TcpStream`.
///
/// This wrapper implements the `AsyncStream` trait for use with the smol runtime.
#[cfg(feature = "smol-runtime")]
#[cfg_attr(docsrs, doc(cfg(feature = "smol-runtime")))]
pub struct SmolStream(pub smol::net::TcpStream);

#[cfg(feature = "smol-runtime")]
#[cfg_attr(docsrs, doc(cfg(feature = "smol-runtime")))]
#[async_trait]
impl AsyncStream for SmolStream {
    async fn connect(addr: &str) -> std::io::Result<Self>
    where
        Self: Sized,
    {
        let stream = smol::net::TcpStream::connect(addr).await?;
        Ok(SmolStream(stream))
    }

    async fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        use smol::io::AsyncReadExt;
        self.0.read(buf).await
    }

    async fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        smol::io::AsyncWriteExt::write_all(&mut self.0, buf).await
    }

    async fn shutdown(&mut self) -> std::io::Result<()> {
        // smol uses close() for shutdown
        smol::io::AsyncWriteExt::close(&mut self.0).await
    }
}
