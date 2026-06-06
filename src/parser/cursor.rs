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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;

    #[test]
    fn cursor_expect_successes_and_predicates() {
        let tokens = tokenize("(define :kw ?x 12)").unwrap();
        let mut parser = Parser::new(&tokens);

        assert!(parser.at_lparen());
        parser.expect_lparen().unwrap();
        assert!(parser.at_symbol("define"));
        parser.expect_symbol_eq("define").unwrap();
        assert!(parser.at_keyword_any());
        assert_eq!(parser.expect_keyword().unwrap(), "kw");
        assert_eq!(parser.expect_variable().unwrap(), "?x");
        assert!(parser.at_number());
        assert!(parser.advance().is_ok());
        assert!(parser.at_rparen());
        parser.expect_rparen().unwrap();
        assert!(parser.advance().is_err());
    }

    #[test]
    fn cursor_expect_errors() {
        let tokens = tokenize(":kw").unwrap();
        let mut parser = Parser::new(&tokens);
        assert!(parser.expect_lparen().is_err());

        let tokens = tokenize("(").unwrap();
        let mut parser = Parser::new(&tokens);
        assert!(parser.expect_rparen().is_err());

        let tokens = tokenize("?x").unwrap();
        let mut parser = Parser::new(&tokens);
        assert!(parser.expect_symbol().is_err());

        let tokens = tokenize("actual").unwrap();
        let mut parser = Parser::new(&tokens);
        assert!(parser.expect_symbol_eq("expected").is_err());

        let tokens = tokenize("name").unwrap();
        let mut parser = Parser::new(&tokens);
        assert!(parser.expect_keyword().is_err());

        let tokens = tokenize("name").unwrap();
        let mut parser = Parser::new(&tokens);
        assert!(parser.expect_variable().is_err());

        let empty = Vec::new();
        let parser = Parser::new(&empty);
        assert_eq!(parser.current_span(), Span::new(0, 1, 1));
        assert_eq!(parser.eof_span(), Span::new(0, 1, 1));
    }
}
