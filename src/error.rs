//! Error types for the PDDL parser.
//!
//! Provides [`ParseError`] with a human-readable message and a [`Span`] that pinpoints
//! the exact location (byte offset, line, column) in the source text where the error occurred.

use std::fmt;

/// A source location in the PDDL input text.
///
/// Used throughout the parser and lexer to attach position information to tokens
/// and error messages. Both `line` and `col` are 1-based.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct Span {
    /// Byte offset from the start of the input.
    pub offset: usize,
    /// 1-based line number.
    pub line: usize,
    /// 1-based column number.
    pub col: usize,
}

impl Span {
    /// Create a new source span.
    ///
    /// # Arguments
    ///
    /// * `offset` - Byte offset from the start of the input
    /// * `line` - 1-based line number
    /// * `col` - 1-based column number
    pub fn new(offset: usize, line: usize, col: usize) -> Self {
        Self { offset, line, col }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

/// A parse error with a human-readable message and source location.
///
/// Displayed as `"parse error at <line>:<col>: <message>"`.
/// Implements `std::error::Error` for integration with `anyhow` / `?` chains.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// What went wrong.
    pub message: String,
    /// Where in the source text it went wrong.
    pub span: Span,
}

impl ParseError {
    /// Create a new parse error.
    ///
    /// # Arguments
    ///
    /// * `message` - Human-readable description of the error
    /// * `span` - Source location where the error was detected
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "parse error at {}: {}", self.span, self.message)
    }
}

impl std::error::Error for ParseError {}
