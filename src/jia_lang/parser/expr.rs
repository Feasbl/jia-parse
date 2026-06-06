//! Expression parser for the CP language using Pratt parsing.
//!
//! Precedence levels (lowest to highest):
//! 1. Additive: `+`, `-`
//! 2. Multiplicative: `*`
//! 3. Atoms: `Number`, `Ident`, `start_of(name)`, `end_of(name)`, `duration_of(name)`, `(expr)`, unary `-`

use super::Parser;
use crate::error::ParseError;
use crate::jia_lang::ast::{ArithOp, Expr};
use crate::jia_lang::lexer::TokenKind;

impl<'a> Parser<'a> {
    /// Parse an expression.
    pub fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_additive()
    }

    /// Parse additive expressions: `term (('+' | '-') term)*`
    fn parse_additive(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = match self.peek_kind() {
                Some(TokenKind::Plus) => ArithOp::Add,
                Some(TokenKind::Minus) => ArithOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            left = Expr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    /// Parse multiplicative expressions: `atom (('*' | '/') atom)*`
    fn parse_multiplicative(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_atom()?;
        loop {
            let op = match self.peek_kind() {
                Some(TokenKind::Star) => ArithOp::Mul,
                Some(TokenKind::Slash) => ArithOp::Div,
                _ => break,
            };
            self.advance();
            let right = self.parse_atom()?;
            left = Expr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    /// Parse an atom: number, identifier, function call, parenthesized expr, or unary negation.
    fn parse_atom(&mut self) -> Result<Expr, ParseError> {
        match self.peek_kind() {
            Some(TokenKind::Number(n)) => {
                self.advance();
                Ok(Expr::Number(n))
            }
            Some(TokenKind::Float(f)) => {
                self.advance();
                Ok(Expr::Float(f))
            }
            Some(TokenKind::Minus) => {
                self.advance();
                let inner = self.parse_atom()?;
                Ok(Expr::Negate(Box::new(inner)))
            }
            Some(TokenKind::LParen) => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect_token(TokenKind::RParen)?;
                Ok(expr)
            }
            Some(TokenKind::Ident(name)) => {
                // Check if it's a function call: start_of, end_of, duration_of, present_of
                match name.as_str() {
                    "start_of" | "end_of" | "duration_of" | "present_of" => {
                        let func = name.clone();
                        self.advance();
                        self.expect_token(TokenKind::LParen)?;
                        let arg = self.expect_ident()?;
                        self.expect_token(TokenKind::RParen)?;
                        match func.as_str() {
                            "start_of" => Ok(Expr::StartOf(arg)),
                            "end_of" => Ok(Expr::EndOf(arg)),
                            "duration_of" => Ok(Expr::DurationOf(arg)),
                            "present_of" => Ok(Expr::PresentOf(arg)),
                            _ => unreachable!(),
                        }
                    }
                    _ => {
                        let name = name.clone();
                        self.advance();
                        Ok(Expr::Var(name))
                    }
                }
            }
            _ => {
                let span = self.current_span();
                Err(ParseError::new("expected expression", span))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::jia_lang::ast::{ArithOp, Expr};
    use crate::jia_lang::lexer::tokenize;
    use crate::jia_lang::parser::Parser;

    fn parse_expr(input: &str) -> Expr {
        let tokens = tokenize(input).unwrap();
        let mut parser = Parser::new(&tokens);
        parser.parse_expr().unwrap()
    }

    #[test]
    fn test_number() {
        assert_eq!(parse_expr("42"), Expr::Number(42));
    }

    #[test]
    fn test_variable() {
        assert_eq!(parse_expr("makespan"), Expr::Var("makespan".to_string()));
    }

    #[test]
    fn test_start_of() {
        assert_eq!(
            parse_expr("start_of(task_a)"),
            Expr::StartOf("task_a".to_string())
        );
    }

    #[test]
    fn test_end_of() {
        assert_eq!(
            parse_expr("end_of(task_b)"),
            Expr::EndOf("task_b".to_string())
        );
    }

    #[test]
    fn test_duration_of() {
        assert_eq!(
            parse_expr("duration_of(task_a)"),
            Expr::DurationOf("task_a".to_string())
        );
    }

    #[test]
    fn test_addition() {
        assert_eq!(
            parse_expr("end_of(a) + 5"),
            Expr::BinaryOp {
                op: ArithOp::Add,
                left: Box::new(Expr::EndOf("a".to_string())),
                right: Box::new(Expr::Number(5)),
            }
        );
    }

    #[test]
    fn test_precedence_mul_before_add() {
        // 1 + 2 * 3 should be 1 + (2 * 3)
        let expr = parse_expr("1 + 2 * 3");
        assert_eq!(
            expr,
            Expr::BinaryOp {
                op: ArithOp::Add,
                left: Box::new(Expr::Number(1)),
                right: Box::new(Expr::BinaryOp {
                    op: ArithOp::Mul,
                    left: Box::new(Expr::Number(2)),
                    right: Box::new(Expr::Number(3)),
                }),
            }
        );
    }

    #[test]
    fn test_parenthesized() {
        let expr = parse_expr("(1 + 2) * 3");
        assert_eq!(
            expr,
            Expr::BinaryOp {
                op: ArithOp::Mul,
                left: Box::new(Expr::BinaryOp {
                    op: ArithOp::Add,
                    left: Box::new(Expr::Number(1)),
                    right: Box::new(Expr::Number(2)),
                }),
                right: Box::new(Expr::Number(3)),
            }
        );
    }

    #[test]
    fn test_negation() {
        assert_eq!(parse_expr("-5"), Expr::Negate(Box::new(Expr::Number(5))));
    }
}
