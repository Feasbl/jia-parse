//! Constraints block parser for the CP language.
//!
//! Parses constraints like:
//! ```text
//! constraints {
//!   no_overlap(machine1)
//!   cumulative(res1, 4)
//!   span(project, phases)
//!   alternative(j1_o1, j1_o1_alts)
//!   end_of(a) <= start_of(b)
//! }
//! ```

use super::Parser;
use crate::error::ParseError;
use crate::jia_lang::ast::{CmpOp, Constraint};
use crate::jia_lang::lexer::TokenKind;

impl<'a> Parser<'a> {
    /// Parse a `constraints { ... }` block.
    pub fn parse_constraints_block(&mut self) -> Result<Vec<Constraint>, ParseError> {
        self.expect_ident_matching("constraints")?;
        self.expect_token(TokenKind::LBrace)?;

        let mut constraints = Vec::new();
        while self.peek_kind() != Some(TokenKind::RBrace) {
            constraints.push(self.parse_constraint_stmt()?);
        }
        self.expect_token(TokenKind::RBrace)?;
        Ok(constraints)
    }

    /// Parse a single constraint statement.
    fn parse_constraint_stmt(&mut self) -> Result<Constraint, ParseError> {
        // Dispatch on first ident
        if let Some(TokenKind::Ident(name)) = self.peek_kind() {
            match name.as_str() {
                "no_overlap" => return self.parse_no_overlap(),
                "cumulative" => return self.parse_cumulative(),
                "span" => return self.parse_span(),
                "alternative" => return self.parse_alternative(),
                _ => {}
            }
        }

        // Otherwise it's a comparison: expr op expr
        self.parse_comparison()
    }

    /// Parse `no_overlap(name, ...)`
    fn parse_no_overlap(&mut self) -> Result<Constraint, ParseError> {
        self.expect_ident_matching("no_overlap")?;
        self.expect_token(TokenKind::LParen)?;
        let intervals = self.parse_ident_list()?;
        self.expect_token(TokenKind::RParen)?;
        Ok(Constraint::NoOverlap { intervals })
    }

    /// Parse `cumulative(set, capacity_expr)`
    fn parse_cumulative(&mut self) -> Result<Constraint, ParseError> {
        self.expect_ident_matching("cumulative")?;
        self.expect_token(TokenKind::LParen)?;
        let set = self.expect_ident()?;
        self.expect_token(TokenKind::Comma)?;
        let capacity = self.parse_expr()?;
        self.expect_token(TokenKind::RParen)?;
        Ok(Constraint::Cumulative { set, capacity })
    }

    /// Parse `span(parent, set)`
    fn parse_span(&mut self) -> Result<Constraint, ParseError> {
        self.expect_ident_matching("span")?;
        self.expect_token(TokenKind::LParen)?;
        let parent = self.expect_ident()?;
        self.expect_token(TokenKind::Comma)?;
        let set = self.expect_ident()?;
        self.expect_token(TokenKind::RParen)?;
        Ok(Constraint::Span { parent, set })
    }

    /// Parse `alternative(parent, set)`
    fn parse_alternative(&mut self) -> Result<Constraint, ParseError> {
        self.expect_ident_matching("alternative")?;
        self.expect_token(TokenKind::LParen)?;
        let parent = self.expect_ident()?;
        self.expect_token(TokenKind::Comma)?;
        let set = self.expect_ident()?;
        self.expect_token(TokenKind::RParen)?;
        Ok(Constraint::Alternative { parent, set })
    }

    /// Parse a comparison constraint: `expr op expr`
    fn parse_comparison(&mut self) -> Result<Constraint, ParseError> {
        let left = self.parse_expr()?;
        let op = self.parse_cmp_op()?;
        let right = self.parse_expr()?;
        Ok(Constraint::Comparison { left, op, right })
    }

    /// Parse a comparison operator.
    fn parse_cmp_op(&mut self) -> Result<CmpOp, ParseError> {
        let span = self.current_span();
        match self.peek_kind() {
            Some(TokenKind::Le) => {
                self.advance();
                Ok(CmpOp::Le)
            }
            Some(TokenKind::Ge) => {
                self.advance();
                Ok(CmpOp::Ge)
            }
            Some(TokenKind::Lt) => {
                self.advance();
                Ok(CmpOp::Lt)
            }
            Some(TokenKind::Gt) => {
                self.advance();
                Ok(CmpOp::Gt)
            }
            Some(TokenKind::EqEq) | Some(TokenKind::Eq) => {
                self.advance();
                Ok(CmpOp::Eq)
            }
            Some(TokenKind::Ne) => {
                self.advance();
                Ok(CmpOp::Ne)
            }
            _ => Err(ParseError::new(
                "expected comparison operator (<=, >=, <, >, ==, !=, =)",
                span,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::jia_lang::ast::{CmpOp, Constraint, Expr};
    use crate::jia_lang::lexer::tokenize;
    use crate::jia_lang::parser::Parser;

    fn parse_constraints(input: &str) -> Vec<Constraint> {
        let tokens = tokenize(input).unwrap();
        let mut parser = Parser::new(&tokens);
        parser.parse_constraints_block().unwrap()
    }

    #[test]
    fn test_no_overlap() {
        let cs = parse_constraints("constraints { no_overlap(machine1) }");
        assert_eq!(
            cs[0],
            Constraint::NoOverlap {
                intervals: vec!["machine1".to_string()]
            }
        );
    }

    #[test]
    fn test_no_overlap_multiple() {
        let cs = parse_constraints("constraints { no_overlap(a, b, c) }");
        assert_eq!(
            cs[0],
            Constraint::NoOverlap {
                intervals: vec!["a".to_string(), "b".to_string(), "c".to_string()]
            }
        );
    }

    #[test]
    fn test_cumulative() {
        let cs = parse_constraints("constraints { cumulative(res1, 4) }");
        assert_eq!(
            cs[0],
            Constraint::Cumulative {
                set: "res1".to_string(),
                capacity: Expr::Number(4),
            }
        );
    }

    #[test]
    fn test_span() {
        let cs = parse_constraints("constraints { span(project, phases) }");
        assert_eq!(
            cs[0],
            Constraint::Span {
                parent: "project".to_string(),
                set: "phases".to_string(),
            }
        );
    }

    #[test]
    fn test_alternative() {
        let cs = parse_constraints("constraints { alternative(j1_o1, j1_o1_alts) }");
        assert_eq!(
            cs[0],
            Constraint::Alternative {
                parent: "j1_o1".to_string(),
                set: "j1_o1_alts".to_string(),
            }
        );
    }

    #[test]
    fn test_comparison() {
        let cs = parse_constraints("constraints { end_of(a) <= start_of(b) }");
        assert_eq!(
            cs[0],
            Constraint::Comparison {
                left: Expr::EndOf("a".to_string()),
                op: CmpOp::Le,
                right: Expr::StartOf("b".to_string()),
            }
        );
    }

    #[test]
    fn test_comparison_with_arithmetic() {
        let cs = parse_constraints("constraints { start_of(b) >= end_of(a) + gap }");
        assert!(matches!(
            &cs[0],
            Constraint::Comparison { op, .. } if *op == CmpOp::Ge
        ));
    }

    #[test]
    fn test_lt_gt_ne_and_missing_comparator_errors() {
        for (input, expected) in [
            ("constraints { x < 1 }", CmpOp::Lt),
            ("constraints { x > 1 }", CmpOp::Gt),
            ("constraints { x != 1 }", CmpOp::Ne),
        ] {
            let cs = parse_constraints(input);
            assert!(matches!(
                &cs[0],
                Constraint::Comparison { op, .. } if *op == expected
            ));
        }

        let tokens = tokenize("constraints { x + 1 }").unwrap();
        let mut parser = Parser::new(&tokens);
        assert!(parser.parse_constraints_block().is_err());
    }
}
