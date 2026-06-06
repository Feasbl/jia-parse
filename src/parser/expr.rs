//! Numeric expression parsing.

use crate::ast::{BinaryOp, FunctionTerm, NumericExpr};
use crate::error::ParseError;
use crate::lexer::TokenKind;

use super::cursor::Parser;
use super::terms::parse_term;
use super::utils::parse_number_literal;

fn binary_op_from_symbol(symbol: &str) -> Option<BinaryOp> {
    match symbol {
        "+" => Some(BinaryOp::Add),
        "-" => Some(BinaryOp::Sub),
        "*" => Some(BinaryOp::Mul),
        "/" => Some(BinaryOp::Div),
        _ => None,
    }
}

/// Parse a numeric expression (arithmetic, function calls, ?duration, total-time, numbers).
/// N-ary operators are folded left-associatively.
pub(super) fn parse_numeric_expr(p: &mut Parser) -> Result<NumericExpr, ParseError> {
    if p.at_lparen() {
        p.expect_lparen()?;

        let tok = p
            .peek()
            .ok_or_else(|| ParseError::new("unexpected end in numeric expression", p.eof_span()))?;

        if let TokenKind::Symbol(s) = &tok.kind {
            let s = s.clone();
            if let Some(op) = binary_op_from_symbol(&s) {
                p.advance()?;
                let first = parse_numeric_expr(p)?;
                if p.at_rparen() && s == "-" {
                    p.expect_rparen()?;
                    return Ok(NumericExpr::Negate(Box::new(first)));
                }
                let mut result = first;
                while !p.at_rparen() {
                    let next = parse_numeric_expr(p)?;
                    result = NumericExpr::BinaryOp {
                        op: op.clone(),
                        left: Box::new(result),
                        right: Box::new(next),
                    };
                }
                p.expect_rparen()?;
                return Ok(result);
            }

            // Function call
            p.advance()?;
            let mut args = Vec::new();
            while !p.at_rparen() {
                args.push(parse_term(p)?);
            }
            p.expect_rparen()?;
            return Ok(NumericExpr::FunctionCall(FunctionTerm { name: s, args }));
        }

        return Err(ParseError::new(
            format!("unexpected token in numeric expression: {:?}", tok.kind),
            tok.span,
        ));
    }

    if p.at_number() {
        return Ok(NumericExpr::Number(parse_number_literal(p)?));
    }

    if let Some(tok) = p.peek() {
        if let TokenKind::Variable(v) = &tok.kind {
            if v == "?duration" {
                p.advance()?;
                return Ok(NumericExpr::Duration);
            }
        }
        if let TokenKind::Symbol(s) = &tok.kind {
            if s == "total-time" {
                p.advance()?;
                return Ok(NumericExpr::TotalTime);
            }
            // Bare function name (0-arity)
            let name = s.clone();
            p.advance()?;
            return Ok(NumericExpr::FunctionCall(FunctionTerm {
                name,
                args: Vec::new(),
            }));
        }
    }

    Err(ParseError::new(
        "expected numeric expression",
        p.current_span(),
    ))
}

/// Parse metric expression -- similar to numeric expr but allows `total-time` at top level
pub(super) fn parse_metric_expr(p: &mut Parser) -> Result<NumericExpr, ParseError> {
    if p.at_lparen() {
        p.expect_lparen()?;

        let tok = p.peek().ok_or_else(|| {
            ParseError::new("unexpected end of input in metric expression", p.eof_span())
        })?;

        if tok.symbol_eq("total-time") {
            p.advance()?;
            p.expect_rparen()?;
            return Ok(NumericExpr::TotalTime);
        }

        // Binary op or function call
        if let TokenKind::Symbol(s) = &tok.kind {
            let s = s.clone();
            if let Some(op) = binary_op_from_symbol(&s) {
                p.advance()?;
                let first = parse_metric_expr(p)?;
                if p.at_rparen() {
                    // Unary minus
                    p.expect_rparen()?;
                    return Ok(NumericExpr::Negate(Box::new(first)));
                }
                // Fold n-ary into left-associative binary tree
                let mut result = first;
                while !p.at_rparen() {
                    let next = parse_metric_expr(p)?;
                    result = NumericExpr::BinaryOp {
                        op: op.clone(),
                        left: Box::new(result),
                        right: Box::new(next),
                    };
                }
                p.expect_rparen()?;
                return Ok(result);
            }

            // Function call
            p.advance()?;
            let mut args = Vec::new();
            while !p.at_rparen() {
                args.push(parse_term(p)?);
            }
            p.expect_rparen()?;
            return Ok(NumericExpr::FunctionCall(FunctionTerm { name: s, args }));
        }

        return Err(ParseError::new(
            format!("unexpected token in metric expression: {:?}", tok.kind),
            tok.span,
        ));
    }

    if p.at_number() {
        return Ok(NumericExpr::Number(parse_number_literal(p)?));
    }

    if p.at_symbol("total-time") {
        p.advance()?;
        return Ok(NumericExpr::TotalTime);
    }

    // Bare function name with no args
    if let Some(tok) = p.peek() {
        if let TokenKind::Symbol(s) = &tok.kind {
            let s = s.clone();
            p.advance()?;
            return Ok(NumericExpr::FunctionCall(FunctionTerm {
                name: s,
                args: Vec::new(),
            }));
        }
    }

    Err(ParseError::new(
        "expected metric expression",
        p.current_span(),
    ))
}

/// Heuristic lookahead to distinguish numeric expressions from logical conditions at an `=` comparison.
pub(super) fn is_numeric_start(p: &Parser) -> bool {
    match p.peek() {
        Some(crate::lexer::Token {
            kind: TokenKind::Number(_),
            ..
        }) => true,
        Some(crate::lexer::Token {
            kind: TokenKind::Variable(v),
            ..
        }) if v == "?duration" => true,
        Some(crate::lexer::Token {
            kind: TokenKind::LParen,
            ..
        }) => {
            // Look ahead past '(' to see if it's a numeric function or op
            if let Some(next) = p.tokens.get(p.pos + 1) {
                matches!(&next.kind, TokenKind::Symbol(s) if
                    matches!(s.as_str(), "+" | "-" | "*" | "/" | "total-time")
                    || !matches!(s.as_str(),
                        "and" | "or" | "not" | "forall" | "exists" | "imply"
                        | "at" | "over" | "preference" | "when"
                    )
                )
            } else {
                false
            }
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::cursor::Parser;

    fn parse_numeric(input: &str) -> NumericExpr {
        let tokens = tokenize(input).unwrap();
        let mut parser = Parser::new(&tokens);
        parse_numeric_expr(&mut parser).unwrap()
    }

    fn parse_metric(input: &str) -> NumericExpr {
        let tokens = tokenize(input).unwrap();
        let mut parser = Parser::new(&tokens);
        parse_metric_expr(&mut parser).unwrap()
    }

    #[test]
    fn numeric_expr_variants() {
        assert_eq!(binary_op_from_symbol("%"), None);
        assert!(matches!(parse_numeric("(- 5)"), NumericExpr::Negate(_)));
        assert!(matches!(
            parse_numeric("(/ (* 8 2) (+ 1 3))"),
            NumericExpr::BinaryOp { .. }
        ));
        assert!(matches!(
            parse_numeric("(fuel truck1)"),
            NumericExpr::FunctionCall(FunctionTerm { .. })
        ));
        assert_eq!(parse_numeric("?duration"), NumericExpr::Duration);
        assert_eq!(parse_numeric("total-time"), NumericExpr::TotalTime);
        assert!(matches!(
            parse_numeric("fuel"),
            NumericExpr::FunctionCall(FunctionTerm { .. })
        ));
    }

    #[test]
    fn numeric_expr_errors() {
        let tokens = tokenize("(42)").unwrap();
        let mut parser = Parser::new(&tokens);
        assert!(parse_numeric_expr(&mut parser).is_err());

        let tokens = tokenize("(").unwrap();
        let mut parser = Parser::new(&tokens);
        assert!(parse_numeric_expr(&mut parser).is_err());

        let tokens = tokenize("?x").unwrap();
        let mut parser = Parser::new(&tokens);
        assert!(parse_numeric_expr(&mut parser).is_err());

        let tokens = tokenize(")").unwrap();
        let mut parser = Parser::new(&tokens);
        assert!(parse_numeric_expr(&mut parser).is_err());

        let tokens = Vec::new();
        let mut parser = Parser::new(&tokens);
        assert!(parse_numeric_expr(&mut parser).is_err());
    }

    #[test]
    fn metric_expr_variants_and_errors() {
        assert_eq!(parse_metric("(total-time)"), NumericExpr::TotalTime);
        assert_eq!(parse_metric("total-time"), NumericExpr::TotalTime);
        assert!(matches!(parse_metric("(- 5)"), NumericExpr::Negate(_)));
        assert!(matches!(
            parse_metric("(+ total-time cost)"),
            NumericExpr::BinaryOp { .. }
        ));
        assert!(matches!(
            parse_metric("(distance a b)"),
            NumericExpr::FunctionCall(FunctionTerm { .. })
        ));
        assert!(matches!(
            parse_metric("cost"),
            NumericExpr::FunctionCall(FunctionTerm { .. })
        ));

        let tokens = tokenize(")").unwrap();
        let mut parser = Parser::new(&tokens);
        assert!(parse_metric_expr(&mut parser).is_err());

        let tokens = tokenize("(42)").unwrap();
        let mut parser = Parser::new(&tokens);
        assert!(parse_metric_expr(&mut parser).is_err());

        let tokens = tokenize("(").unwrap();
        let mut parser = Parser::new(&tokens);
        assert!(parse_metric_expr(&mut parser).is_err());

        let tokens = tokenize("?x").unwrap();
        let mut parser = Parser::new(&tokens);
        assert!(parse_metric_expr(&mut parser).is_err());

        let tokens = Vec::new();
        let mut parser = Parser::new(&tokens);
        assert!(parse_metric_expr(&mut parser).is_err());
    }

    #[test]
    fn numeric_start_lookahead_variants() {
        for (input, expected) in [
            ("1", true),
            ("?duration", true),
            ("(+ 1 2)", true),
            ("(fuel truck1)", true),
            ("(and (ready))", false),
            ("(", false),
            ("ready", false),
        ] {
            let tokens = tokenize(input).unwrap();
            let parser = Parser::new(&tokens);
            assert_eq!(is_numeric_start(&parser), expected, "{input}");
        }

        let tokens = Vec::new();
        let parser = Parser::new(&tokens);
        assert!(!is_numeric_start(&parser));
    }
}
