//! Parser cursor: a position-tracking view over a token stream.

use crate::error::{ParseError, Span};
use crate::lexer::{Token, TokenKind};

/// A cursor over a token slice with position tracking for the recursive-descent parser.
pub(super) struct Parser<'a> {
    pub(super) tokens: &'a [Token],
    pub(super) pos: usize,
}

impl<'a> Parser<'a> {
    pub(super) fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, pos: 0 }
    }

    pub(super) fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    pub(super) fn advance(&mut self) -> Result<&Token, ParseError> {
        let tok = self
            .tokens
            .get(self.pos)
            .ok_or_else(|| ParseError::new("unexpected end of input", self.eof_span()))?;
        self.pos += 1;
        Ok(tok)
    }

    pub(super) fn eof_span(&self) -> Span {
        if let Some(last) = self.tokens.last() {
            Span::new(last.span.offset + 1, last.span.line, last.span.col + 1)
        } else {
            Span::new(0, 1, 1)
        }
    }

    pub(super) fn current_span(&self) -> Span {
        self.peek()
            .map(|t| t.span)
            .unwrap_or_else(|| self.eof_span())
    }

    pub(super) fn expect_lparen(&mut self) -> Result<Span, ParseError> {
        let tok = self.advance()?;
        if tok.kind == TokenKind::LParen {
            Ok(tok.span)
        } else {
            Err(ParseError::new(
                format!("expected '(', got {:?}", tok.kind),
                tok.span,
            ))
        }
    }

    pub(super) fn expect_rparen(&mut self) -> Result<Span, ParseError> {
        let tok = self.advance()?;
        if tok.kind == TokenKind::RParen {
            Ok(tok.span)
        } else {
            Err(ParseError::new(
                format!("expected ')', got {:?}", tok.kind),
                tok.span,
            ))
        }
    }

    pub(super) fn expect_symbol(&mut self) -> Result<String, ParseError> {
        let tok = self.advance()?;
        if let TokenKind::Symbol(s) = &tok.kind {
            Ok(s.clone())
        } else {
            Err(ParseError::new(
                format!("expected symbol, got {:?}", tok.kind),
                tok.span,
            ))
        }
    }

    pub(super) fn expect_symbol_eq(&mut self, expected: &str) -> Result<(), ParseError> {
        let tok = self.advance()?;
        if tok.symbol_eq(expected) {
            Ok(())
        } else {
            Err(ParseError::new(
                format!("expected '{expected}', got {:?}", tok.kind),
                tok.span,
            ))
        }
    }

    pub(super) fn expect_keyword(&mut self) -> Result<String, ParseError> {
        let tok = self.advance()?;
        if let TokenKind::Keyword(k) = &tok.kind {
            Ok(k.clone())
        } else {
            Err(ParseError::new(
                format!("expected keyword, got {:?}", tok.kind),
                tok.span,
            ))
        }
    }

    pub(super) fn expect_variable(&mut self) -> Result<String, ParseError> {
        let tok = self.advance()?;
        if let TokenKind::Variable(v) = &tok.kind {
            Ok(v.clone())
        } else {
            Err(ParseError::new(
                format!("expected variable, got {:?}", tok.kind),
                tok.span,
            ))
        }
    }

    pub(super) fn at_lparen(&self) -> bool {
        matches!(
            self.peek(),
            Some(Token {
                kind: TokenKind::LParen,
                ..
            })
        )
    }

    pub(super) fn at_rparen(&self) -> bool {
        matches!(
            self.peek(),
            Some(Token {
                kind: TokenKind::RParen,
                ..
            })
        )
    }

    pub(super) fn at_symbol(&self, s: &str) -> bool {
        matches!(self.peek(), Some(tok) if tok.symbol_eq(s))
    }

    pub(super) fn at_number(&self) -> bool {
        matches!(
            self.peek(),
            Some(Token {
                kind: TokenKind::Number(_),
                ..
            })
        )
    }

    pub(super) fn at_keyword_any(&self) -> bool {
        matches!(
            self.peek(),
            Some(Token {
                kind: TokenKind::Keyword(_),
                ..
            })
        )
    }
}
