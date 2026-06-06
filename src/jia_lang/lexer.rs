//! Tokenizer for the Jia modelling language.
//!
//! Converts raw `.jia` source text into a flat vector of [`Token`]s, each carrying
//! a [`TokenKind`] and a [`Span`] for error reporting.
//!
//! Token conventions:
//! - `//` → line comment (skipped entirely)
//! - Two-char operators: `<=`, `>=`, `==`, `!=`, `..`
//! - Identifiers are case-sensitive (types are capitalized, user names lowercase)
//! - `@model` is tokenized as `At` + `Ident("model")`
//! - Numbers: integer `42` or float `3.14`

use crate::error::{ParseError, Span};

/// The kind of a lexed CP token.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `{`
    LBrace,
    /// `}`
    RBrace,
    /// `[`
    LBracket,
    /// `]`
    RBracket,
    /// `,`
    Comma,
    /// `:`
    Colon,
    /// `..`
    DotDot,
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `@`
    At,
    /// `<=`
    Le,
    /// `>=`
    Ge,
    /// `<`
    Lt,
    /// `>`
    Gt,
    /// `==`
    EqEq,
    /// `!=`
    Ne,
    /// `=`
    Eq,
    /// An integer literal.
    Number(i64),
    /// A floating-point literal.
    Float(f64),
    /// An identifier or keyword.
    Ident(String),
}

/// A single token produced by the CP lexer.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// What kind of token this is.
    pub kind: TokenKind,
    /// Where in the source text this token appears.
    pub span: Span,
}

/// Tokenize a CP source string into a flat vector of [`Token`]s.
///
/// Line comments (starting with `//`) are stripped. Whitespace is consumed
/// between tokens. Returns an error for any character that doesn't belong
/// to a valid token.
pub fn tokenize(input: &str) -> Result<Vec<Token>, ParseError> {
    let mut tokens = Vec::new();
    let bytes = input.as_bytes();
    let mut pos = 0;
    let mut line = 1usize;
    let mut col = 1usize;

    while pos < bytes.len() {
        let b = bytes[pos];

        // Skip whitespace
        if b.is_ascii_whitespace() {
            if b == b'\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
            pos += 1;
            continue;
        }

        // Line comments (must check before `/` as single-char operator)
        if b == b'/' && pos + 1 < bytes.len() && bytes[pos + 1] == b'/' {
            // Skip to end of line
            while pos < bytes.len() && bytes[pos] != b'\n' {
                pos += 1;
            }
            continue;
        }

        let span = Span::new(pos, line, col);

        // Two-char operators first
        if pos + 1 < bytes.len() {
            let next = bytes[pos + 1];
            let two_char = match (b, next) {
                (b'<', b'=') => Some(TokenKind::Le),
                (b'>', b'=') => Some(TokenKind::Ge),
                (b'=', b'=') => Some(TokenKind::EqEq),
                (b'!', b'=') => Some(TokenKind::Ne),
                (b'.', b'.') => Some(TokenKind::DotDot),
                _ => None,
            };
            if let Some(kind) = two_char {
                tokens.push(Token { kind, span });
                pos += 2;
                col += 2;
                continue;
            }
        }

        // Single-char tokens
        let single = match b {
            b'(' => Some(TokenKind::LParen),
            b')' => Some(TokenKind::RParen),
            b'{' => Some(TokenKind::LBrace),
            b'}' => Some(TokenKind::RBrace),
            b'[' => Some(TokenKind::LBracket),
            b']' => Some(TokenKind::RBracket),
            b',' => Some(TokenKind::Comma),
            b':' => Some(TokenKind::Colon),
            b'+' => Some(TokenKind::Plus),
            b'-' => Some(TokenKind::Minus),
            b'*' => Some(TokenKind::Star),
            b'/' => Some(TokenKind::Slash),
            b'@' => Some(TokenKind::At),
            b'<' => Some(TokenKind::Lt),
            b'>' => Some(TokenKind::Gt),
            b'=' => Some(TokenKind::Eq),
            _ => None,
        };
        if let Some(kind) = single {
            tokens.push(Token { kind, span });
            pos += 1;
            col += 1;
            continue;
        }

        // Numbers (integer or float)
        if b.is_ascii_digit() {
            let start = pos;
            while pos < bytes.len() && bytes[pos].is_ascii_digit() {
                pos += 1;
            }
            // Check for decimal point: must be `.` followed by a digit (not `..` range)
            let is_float = pos < bytes.len()
                && bytes[pos] == b'.'
                && pos + 1 < bytes.len()
                && bytes[pos + 1].is_ascii_digit();
            if is_float {
                pos += 1; // consume '.'
                while pos < bytes.len() && bytes[pos].is_ascii_digit() {
                    pos += 1;
                }
                let text = &input[start..pos];
                let value: f64 = match text.parse() {
                    Ok(value) => value,
                    Err(_) => {
                        return Err(ParseError::new(
                            format!("invalid float literal: {text}"),
                            span,
                        ));
                    }
                };
                tokens.push(Token {
                    kind: TokenKind::Float(value),
                    span,
                });
            } else {
                let text = &input[start..pos];
                let value: i64 = text.parse().map_err(|_| {
                    ParseError::new(format!("invalid number literal: {text}"), span)
                })?;
                tokens.push(Token {
                    kind: TokenKind::Number(value),
                    span,
                });
            }
            col += pos - start;
            continue;
        }

        // Identifiers (start with letter or underscore)
        if b.is_ascii_alphabetic() || b == b'_' {
            let start = pos;
            while pos < bytes.len() && (bytes[pos].is_ascii_alphanumeric() || bytes[pos] == b'_') {
                pos += 1;
            }
            let text = input[start..pos].to_string();
            tokens.push(Token {
                kind: TokenKind::Ident(text),
                span,
            });
            col += pos - start;
            continue;
        }

        return Err(ParseError::new(
            format!("unexpected character: '{}'", b as char),
            span,
        ));
    }

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokens() {
        let tokens = tokenize("model foo").unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].kind, TokenKind::Ident("model".to_string()));
        assert_eq!(tokens[1].kind, TokenKind::Ident("foo".to_string()));
    }

    #[test]
    fn test_operators() {
        let tokens = tokenize("<= >= == != < > = ..").unwrap();
        let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                &TokenKind::Le,
                &TokenKind::Ge,
                &TokenKind::EqEq,
                &TokenKind::Ne,
                &TokenKind::Lt,
                &TokenKind::Gt,
                &TokenKind::Eq,
                &TokenKind::DotDot,
            ]
        );
    }

    #[test]
    fn test_punctuation() {
        let tokens = tokenize("( ) { } [ ] , :").unwrap();
        assert_eq!(tokens.len(), 8);
        assert_eq!(tokens[0].kind, TokenKind::LParen);
        assert_eq!(tokens[1].kind, TokenKind::RParen);
        assert_eq!(tokens[2].kind, TokenKind::LBrace);
        assert_eq!(tokens[3].kind, TokenKind::RBrace);
        assert_eq!(tokens[4].kind, TokenKind::LBracket);
        assert_eq!(tokens[5].kind, TokenKind::RBracket);
        assert_eq!(tokens[6].kind, TokenKind::Comma);
        assert_eq!(tokens[7].kind, TokenKind::Colon);
    }

    #[test]
    fn test_numbers() {
        let tokens = tokenize("42 0 100").unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Number(42));
        assert_eq!(tokens[1].kind, TokenKind::Number(0));
        assert_eq!(tokens[2].kind, TokenKind::Number(100));
    }

    #[test]
    fn test_line_comment() {
        let tokens = tokenize("model foo // this is a comment\nvariables").unwrap();
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].kind, TokenKind::Ident("model".to_string()));
        assert_eq!(tokens[1].kind, TokenKind::Ident("foo".to_string()));
        assert_eq!(tokens[2].kind, TokenKind::Ident("variables".to_string()));
    }

    #[test]
    fn test_span_tracking() {
        let tokens = tokenize("a\nb").unwrap();
        assert_eq!(tokens[0].span, Span::new(0, 1, 1));
        assert_eq!(tokens[1].span, Span::new(2, 2, 1));
    }

    #[test]
    fn test_arithmetic_operators() {
        let tokens = tokenize("+ - *").unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Plus);
        assert_eq!(tokens[1].kind, TokenKind::Minus);
        assert_eq!(tokens[2].kind, TokenKind::Star);
    }

    #[test]
    fn test_unexpected_char() {
        let err = tokenize("model #foo").unwrap_err();
        assert!(err.message.contains("unexpected character"));
    }

    #[test]
    fn test_float_literals() {
        let tokens = tokenize("3.25 0.5 100.0").unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Float(3.25));
        assert_eq!(tokens[1].kind, TokenKind::Float(0.5));
        assert_eq!(tokens[2].kind, TokenKind::Float(100.0));
    }

    #[test]
    fn test_integer_followed_by_dotdot_not_float() {
        // "0..10" should be Number(0), DotDot, Number(10) — not Float
        let tokens = tokenize("0..10").unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Number(0));
        assert_eq!(tokens[1].kind, TokenKind::DotDot);
        assert_eq!(tokens[2].kind, TokenKind::Number(10));
    }

    #[test]
    fn test_slash_operator() {
        let tokens = tokenize("x / 2").unwrap();
        assert_eq!(tokens[1].kind, TokenKind::Slash);
    }

    #[test]
    fn test_slash_vs_comment() {
        // "/" alone is Slash; "//" is a comment
        let tokens = tokenize("x / y // comment").unwrap();
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[1].kind, TokenKind::Slash);
    }

    #[test]
    fn test_at_token() {
        let tokens = tokenize("@model lp").unwrap();
        assert_eq!(tokens[0].kind, TokenKind::At);
        assert_eq!(tokens[1].kind, TokenKind::Ident("model".to_string()));
        assert_eq!(tokens[2].kind, TokenKind::Ident("lp".to_string()));
    }

    #[test]
    fn test_integer_overflow_reports_error() {
        let err = tokenize("999999999999999999999999999999999999").unwrap_err();
        assert!(err.message.contains("invalid number literal"));
    }
}
