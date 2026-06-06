//! Tokenizer for PDDL S-expressions.
//!
//! Converts raw PDDL source text into a flat vector of [`Token`]s, each carrying a
//! [`TokenKind`] and a [`Span`] for error reporting.
//!
//! Token conventions:
//! - `:` prefix → [`TokenKind::Keyword`] (e.g. `:requirements`, `:typing`)
//! - `?` prefix → [`TokenKind::Variable`] (e.g. `?x`, `?duration`)
//! - `;` → line comment (skipped entirely)
//! - All symbols and keywords are lowercased during tokenization.

use crate::error::{ParseError, Span};

/// The kind of a lexed PDDL token.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    /// Opening parenthesis `(`.
    LParen,
    /// Closing parenthesis `)`.
    RParen,
    /// An identifier: predicate names, type names, operators (`+`, `-`, …), etc.
    /// Stored in lowercase.
    Symbol(String),
    /// A PDDL keyword starting with `:` (e.g. `:requirements`, `:typing`).
    /// The leading `:` is stripped; stored in lowercase.
    Keyword(String),
    /// A PDDL variable starting with `?` (e.g. `?x`, `?duration`).
    /// The leading `?` is kept; stored in lowercase.
    Variable(String),
    /// An integer or floating-point numeric literal.
    Number(f64),
}

/// A single token produced by the lexer, pairing a [`TokenKind`] with its source [`Span`].
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// What kind of token this is.
    pub kind: TokenKind,
    /// Where in the source text this token appears.
    pub span: Span,
}

impl Token {
    /// Return `true` if this token is a [`TokenKind::Symbol`] matching `s` (case-insensitive).
    ///
    /// # Arguments
    ///
    /// * `s` - The symbol string to compare against (e.g. `"define"`, `"and"`)
    pub fn symbol_eq(&self, s: &str) -> bool {
        matches!(&self.kind, TokenKind::Symbol(sym) if sym.eq_ignore_ascii_case(s))
    }

    /// Return `true` if this token is a [`TokenKind::Keyword`] matching `s` (case-insensitive).
    ///
    /// # Arguments
    ///
    /// * `s` - The keyword to compare against, *without* the leading `:` (e.g. `"requirements"`)
    pub fn keyword_eq(&self, s: &str) -> bool {
        matches!(&self.kind, TokenKind::Keyword(kw) if kw.eq_ignore_ascii_case(s))
    }

    /// Return `true` if this token is a [`TokenKind::Variable`] matching `s` (case-insensitive).
    ///
    /// # Arguments
    ///
    /// * `s` - The variable to compare against, *with* the leading `?` (e.g. `"?x"`)
    pub fn variable_eq(&self, s: &str) -> bool {
        matches!(&self.kind, TokenKind::Variable(v) if v.eq_ignore_ascii_case(s))
    }
}

/// Tokenize a PDDL source string into a flat vector of [`Token`]s.
///
/// Line comments (starting with `;`) are stripped. All symbols, keywords, and
/// variables are lowercased so that later comparisons are case-insensitive.
///
/// # Arguments
///
/// * `input` - The raw PDDL source text to tokenize
///
/// # Returns
///
/// A vector of tokens in order of appearance.
///
/// # Errors
///
/// Returns [`ParseError`] if the input contains an unexpected character or
/// a malformed numeric literal.
pub fn tokenize(input: &str) -> Result<Vec<Token>, ParseError> {
    let mut tokens = Vec::new();
    let bytes = input.as_bytes();
    let mut pos = 0;
    let mut line = 1usize;
    let mut col = 1usize;

    while pos < bytes.len() {
        let b = bytes[pos];

        // Whitespace
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

        // Comment: skip to end of line
        if b == b';' {
            while pos < bytes.len() && bytes[pos] != b'\n' {
                pos += 1;
            }
            continue;
        }

        let span = Span::new(pos, line, col);

        if b == b'(' {
            tokens.push(Token {
                kind: TokenKind::LParen,
                span,
            });
            pos += 1;
            col += 1;
            continue;
        }

        if b == b')' {
            tokens.push(Token {
                kind: TokenKind::RParen,
                span,
            });
            pos += 1;
            col += 1;
            continue;
        }

        // Keyword `:something`
        if b == b':' {
            pos += 1;
            col += 1;
            let start = pos;
            while pos < bytes.len() && is_symbol_char(bytes[pos]) {
                pos += 1;
                col += 1;
            }
            let kw = std::str::from_utf8(&bytes[start..pos])
                .unwrap()
                .to_ascii_lowercase();
            tokens.push(Token {
                kind: TokenKind::Keyword(kw),
                span,
            });
            continue;
        }

        // Variable `?something`
        if b == b'?' {
            let start = pos;
            pos += 1;
            col += 1;
            while pos < bytes.len() && is_symbol_char(bytes[pos]) {
                pos += 1;
                col += 1;
            }
            let var = std::str::from_utf8(&bytes[start..pos])
                .unwrap()
                .to_lowercase();
            tokens.push(Token {
                kind: TokenKind::Variable(var),
                span,
            });
            continue;
        }

        // Number (possibly negative when preceded by a context that makes it unambiguous,
        // but we handle unary minus in the parser; here we lex positive numbers and
        // the `-` as a symbol)
        if b.is_ascii_digit()
            || (b == b'.' && pos + 1 < bytes.len() && bytes[pos + 1].is_ascii_digit())
        {
            let start = pos;
            while pos < bytes.len() && (bytes[pos].is_ascii_digit() || bytes[pos] == b'.') {
                pos += 1;
                col += 1;
            }
            let num_str = std::str::from_utf8(&bytes[start..pos]).unwrap();
            let num: f64 = num_str
                .parse()
                .map_err(|_| ParseError::new(format!("invalid number: {num_str}"), span))?;
            tokens.push(Token {
                kind: TokenKind::Number(num),
                span,
            });
            continue;
        }

        // String literal
        if b == b'"' {
            pos += 1;
            col += 1;
            let start = pos;
            while pos < bytes.len() && bytes[pos] != b'"' {
                if bytes[pos] == b'\n' {
                    line += 1;
                    col = 1;
                } else {
                    col += 1;
                }
                pos += 1;
            }
            let s = std::str::from_utf8(&bytes[start..pos]).unwrap().to_string();
            if pos < bytes.len() {
                pos += 1; // skip closing "
                col += 1;
            }
            tokens.push(Token {
                kind: TokenKind::Symbol(s),
                span,
            });
            continue;
        }

        // Symbol (identifier, operator like `+`, `-`, `*`, `/`, `=`, `<`, `>`, `<=`, `>=`)
        if is_symbol_start(b) {
            let start = pos;
            while pos < bytes.len() && is_symbol_char(bytes[pos]) {
                pos += 1;
                col += 1;
            }
            let sym = std::str::from_utf8(&bytes[start..pos])
                .unwrap()
                .to_lowercase();
            tokens.push(Token {
                kind: TokenKind::Symbol(sym),
                span,
            });
            continue;
        }

        return Err(ParseError::new(
            format!("unexpected character: '{}'", b as char),
            span,
        ));
    }

    Ok(tokens)
}

/// Return `true` if `b` can begin a PDDL symbol (letters, `_`, arithmetic operators, comparison
/// operators, `#`).
fn is_symbol_start(b: u8) -> bool {
    b.is_ascii_alphabetic()
        || b == b'_'
        || b == b'-'
        || b == b'+'
        || b == b'*'
        || b == b'/'
        || b == b'='
        || b == b'<'
        || b == b'>'
        || b == b'#'
}

/// Return `true` if `b` can appear within (i.e. continue) a PDDL symbol. Superset of
/// [`is_symbol_start`] plus digits and `.`.
fn is_symbol_char(b: u8) -> bool {
    b.is_ascii_alphanumeric()
        || b == b'_'
        || b == b'-'
        || b == b'+'
        || b == b'*'
        || b == b'/'
        || b == b'='
        || b == b'<'
        || b == b'>'
        || b == b'.'
        || b == b'#'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokens() {
        let tokens = tokenize("(define (domain test))").unwrap();
        assert_eq!(tokens.len(), 7);
        assert_eq!(tokens[0].kind, TokenKind::LParen);
        assert!(tokens[1].symbol_eq("define"));
        assert_eq!(tokens[2].kind, TokenKind::LParen);
        assert!(tokens[3].symbol_eq("domain"));
        assert!(tokens[4].symbol_eq("test"));
        assert_eq!(tokens[5].kind, TokenKind::RParen);
        assert_eq!(tokens[6].kind, TokenKind::RParen);
    }

    #[test]
    fn test_keywords_and_variables() {
        let tokens = tokenize("(:requirements :typing) (?x - type1)").unwrap();
        assert!(tokens[1].keyword_eq("requirements"));
        assert!(tokens[2].keyword_eq("typing"));
        assert!(tokens[5].variable_eq("?x"));
    }

    #[test]
    fn test_numbers() {
        // (  =  (  f  )  3.14  ) => 7 tokens
        let tokens = tokenize("(= (f) 3.57)").unwrap();
        assert_eq!(tokens.len(), 7);
        assert_eq!(tokens[5].kind, TokenKind::Number(3.57));
    }

    #[test]
    fn test_comments_skipped() {
        let tokens = tokenize("; comment\n(define)").unwrap();
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].kind, TokenKind::LParen);
    }

    #[test]
    fn test_negative_symbol() {
        let tokens = tokenize("(- 3 2)").unwrap();
        assert!(tokens[1].symbol_eq("-"));
        assert_eq!(tokens[2].kind, TokenKind::Number(3.0));
    }

    #[test]
    fn test_strings_with_newlines_symbols_hash_and_unexpected_char() {
        let tokens = tokenize("\"line one\nline two\" #tag").unwrap();
        assert_eq!(
            tokens[0].kind,
            TokenKind::Symbol("line one\nline two".to_string())
        );
        assert!(tokens[1].symbol_eq("#tag"));

        let err = tokenize("@").unwrap_err();
        assert!(err.message.contains("unexpected character"));
    }
}
