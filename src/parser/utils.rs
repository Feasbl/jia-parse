//! Parser utilities: skip helpers, comparison operators, number literals.

use crate::ast::CompareOp;
use crate::error::ParseError;
use crate::lexer::TokenKind;

use super::cursor::Parser;

/// Skip any S-expressions before `(define ...)` (e.g. `(in-package "PDDL")`)
pub(super) fn skip_to_define(p: &mut Parser) -> Result<(), ParseError> {
    loop {
        p.expect_lparen()?;
        if p.at_symbol("define") {
            p.advance()?;
            return Ok(());
        }
        // Not `define` -- skip the rest of this sexp and try next
        let mut depth = 1u32;
        while depth > 0 {
            let tok = p.advance()?;
            match tok.kind {
                TokenKind::LParen => depth += 1,
                TokenKind::RParen => depth -= 1,
                _ => {}
            }
        }
    }
}

/// Skip a balanced S-expression (used to skip unknown sections).
pub(super) fn skip_sexp(p: &mut Parser) -> Result<(), ParseError> {
    p.expect_lparen()?;
    let mut depth = 1u32;
    while depth > 0 {
        let tok = p.advance()?;
        match tok.kind {
            TokenKind::LParen => depth += 1,
            TokenKind::RParen => depth -= 1,
            _ => {}
        }
    }
    Ok(())
}

/// Parse a comparison operator token.
pub(super) fn parse_compare_op(p: &mut Parser) -> Result<CompareOp, ParseError> {
    let tok = p.advance()?;
    match &tok.kind {
        TokenKind::Symbol(s) => Ok(parse_compare_op_from_str(s)),
        _ => Err(ParseError::new(
            format!("expected comparison operator, got {:?}", tok.kind),
            tok.span,
        )),
    }
}

/// Convert a string to a `CompareOp`.
pub(super) fn parse_compare_op_from_str(s: &str) -> CompareOp {
    match s {
        "<" => CompareOp::Lt,
        "<=" => CompareOp::Lte,
        ">" => CompareOp::Gt,
        ">=" => CompareOp::Gte,
        "=" => CompareOp::Eq,
        _ => CompareOp::Eq, // fallback
    }
}

/// Parse a numeric literal token.
pub(super) fn parse_number_literal(p: &mut Parser) -> Result<f64, ParseError> {
    let tok = p.advance()?;
    match &tok.kind {
        TokenKind::Number(n) => Ok(*n),
        _ => Err(ParseError::new(
            format!("expected number, got {:?}", tok.kind),
            tok.span,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::cursor::Parser;

    #[test]
    fn skip_helpers_skip_balanced_forms() {
        let tokens = tokenize("(metadata (nested value)) (define (domain d))").unwrap();
        let mut parser = Parser::new(&tokens);
        skip_to_define(&mut parser).unwrap();
        assert!(parser.at_lparen());

        let tokens = tokenize("(unknown (nested value)) next").unwrap();
        let mut parser = Parser::new(&tokens);
        skip_sexp(&mut parser).unwrap();
        assert!(parser.at_symbol("next"));
    }

    #[test]
    fn comparison_and_number_error_branches() {
        let tokens = tokenize("name").unwrap();
        let mut parser = Parser::new(&tokens);
        assert_eq!(parse_compare_op(&mut parser).unwrap(), CompareOp::Eq);

        let tokens = tokenize(":kw").unwrap();
        let mut parser = Parser::new(&tokens);
        assert!(parse_compare_op(&mut parser).is_err());

        let tokens = tokenize("name").unwrap();
        let mut parser = Parser::new(&tokens);
        assert!(parse_number_literal(&mut parser).is_err());
    }
}
