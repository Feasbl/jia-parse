//! Domains block parser for the CP language.
//!
//! Parses domain statements like:
//! ```text
//! domains {
//!   duration(a, b) = 3
//!   start(a) in 0..10
//!   optional(x, y)
//!   makespan in 0..100
//!   machine1 = {a, b, c}
//!   demand(a, res1) = 2
//! }
//! ```

use super::Parser;
use crate::error::ParseError;
use crate::jia_lang::ast::{Domain, DomainStmt};
use crate::jia_lang::lexer::TokenKind;

impl<'a> Parser<'a> {
    /// Parse a `domains { ... }` block.
    pub fn parse_domains_block(&mut self) -> Result<Vec<DomainStmt>, ParseError> {
        self.expect_ident_matching("domains")?;
        self.expect_token(TokenKind::LBrace)?;

        let mut stmts = Vec::new();
        while self.peek_kind() != Some(TokenKind::RBrace) {
            stmts.push(self.parse_domain_stmt()?);
        }
        self.expect_token(TokenKind::RBrace)?;
        Ok(stmts)
    }

    /// Parse a single domain statement. Dispatches based on the first token.
    fn parse_domain_stmt(&mut self) -> Result<DomainStmt, ParseError> {
        let name = match self.peek_kind() {
            Some(TokenKind::Ident(s)) => s,
            _ => {
                return Err(ParseError::new(
                    "expected domain statement",
                    self.current_span(),
                ))
            }
        };

        match name.as_str() {
            "duration" | "start" | "end" => self.parse_interval_attr_stmt(),
            "optional" => self.parse_optional_stmt(),
            "demand" => self.parse_demand_stmt(),
            _ => {
                // Could be: `name in ...` (integer domain) or `name = {...}` (set domain)
                let ident = self.expect_ident()?;
                match self.peek_kind() {
                    Some(TokenKind::Ident(kw)) if kw.as_str() == "in" => {
                        self.parse_integer_domain(ident)
                    }
                    Some(TokenKind::Eq) => self.parse_set_domain(ident),
                    _ => Err(ParseError::new(
                        format!("expected 'in' or '=' after '{ident}'"),
                        self.current_span(),
                    )),
                }
            }
        }
    }

    /// Parse an interval attribute statement: `duration(a, b) = 3` or `start(a) in 0..10`
    fn parse_interval_attr_stmt(&mut self) -> Result<DomainStmt, ParseError> {
        let attr = self.expect_ident()?;
        self.expect_token(TokenKind::LParen)?;
        let intervals = self.parse_ident_list()?;
        self.expect_token(TokenKind::RParen)?;
        let domain = self.parse_domain_spec()?;

        match attr.as_str() {
            "duration" => Ok(DomainStmt::IntervalDuration { intervals, domain }),
            "start" => Ok(DomainStmt::IntervalStart { intervals, domain }),
            "end" => Ok(DomainStmt::IntervalEnd { intervals, domain }),
            _ => unreachable!(),
        }
    }

    /// Parse `optional(x, y, z)`
    fn parse_optional_stmt(&mut self) -> Result<DomainStmt, ParseError> {
        self.expect_ident_matching("optional")?;
        self.expect_token(TokenKind::LParen)?;
        let intervals = self.parse_ident_list()?;
        self.expect_token(TokenKind::RParen)?;
        Ok(DomainStmt::IntervalOptional { intervals })
    }

    /// Parse `demand(interval, set) = value`
    fn parse_demand_stmt(&mut self) -> Result<DomainStmt, ParseError> {
        self.expect_ident_matching("demand")?;
        self.expect_token(TokenKind::LParen)?;
        let interval = self.expect_ident()?;
        self.expect_token(TokenKind::Comma)?;
        let set = self.expect_ident()?;
        self.expect_token(TokenKind::RParen)?;
        self.expect_token(TokenKind::Eq)?;
        let value = self.expect_number()?;
        Ok(DomainStmt::Demand {
            interval,
            set,
            value,
        })
    }

    /// Parse an integer domain: `name in 0..100` or `name in {1, 3, 5}`
    /// (The identifier has already been consumed.)
    fn parse_integer_domain(&mut self, name: String) -> Result<DomainStmt, ParseError> {
        self.expect_ident_matching("in")?;
        let domain = self.parse_domain_value()?;
        Ok(DomainStmt::IntegerDomain { name, domain })
    }

    /// Parse a set domain: `name = {a, b, c}`
    /// (The identifier has already been consumed.)
    fn parse_set_domain(&mut self, name: String) -> Result<DomainStmt, ParseError> {
        self.expect_token(TokenKind::Eq)?;
        self.expect_token(TokenKind::LBrace)?;
        let members = self.parse_ident_list()?;
        self.expect_token(TokenKind::RBrace)?;
        Ok(DomainStmt::SetDomain { name, members })
    }

    /// Parse a domain specification after an interval attribute: `= 3`, `= 3.14`, `in 0..10`, or `in {1, 3}`
    fn parse_domain_spec(&mut self) -> Result<Domain, ParseError> {
        match self.peek_kind() {
            Some(TokenKind::Eq) => {
                self.advance();
                match self.peek_kind() {
                    Some(TokenKind::Number(n)) => {
                        self.advance();
                        Ok(Domain::Fixed(n))
                    }
                    Some(TokenKind::Float(f)) => {
                        self.advance();
                        Ok(Domain::RealFixed(f))
                    }
                    _ => Err(ParseError::new(
                        "expected number after '='",
                        self.current_span(),
                    )),
                }
            }
            Some(TokenKind::Ident(kw)) if kw.as_str() == "in" => {
                self.advance();
                self.parse_domain_value()
            }
            _ => Err(ParseError::new(
                "expected '=' or 'in' in domain specification",
                self.current_span(),
            )),
        }
    }

    /// Parse a numeric bound: integer, float, or `inf`/`-inf`.
    fn parse_bound(&mut self) -> Result<f64, ParseError> {
        // Handle optional leading minus for -inf or negative numbers
        let negate = if self.peek_kind() == Some(TokenKind::Minus) {
            self.advance();
            true
        } else {
            false
        };
        match self.peek_kind() {
            Some(TokenKind::Number(n)) => {
                self.advance();
                Ok(if negate { -(n as f64) } else { n as f64 })
            }
            Some(TokenKind::Float(f)) => {
                self.advance();
                Ok(if negate { -f } else { f })
            }
            Some(TokenKind::Ident(s)) if s == "inf" => {
                self.advance();
                Ok(if negate {
                    f64::NEG_INFINITY
                } else {
                    f64::INFINITY
                })
            }
            _ => Err(ParseError::new(
                "expected number, float, or 'inf'",
                self.current_span(),
            )),
        }
    }

    /// Parse a domain value after `in`: `0..100`, `0.0..inf`, `{1, 3, 5}`
    fn parse_domain_value(&mut self) -> Result<Domain, ParseError> {
        match self.peek_kind() {
            Some(TokenKind::LBrace) => {
                // Enumerated: {1, 3, 5}
                self.advance();
                let mut values = vec![self.expect_number()?];
                while self.peek_kind() == Some(TokenKind::Comma) {
                    self.advance();
                    values.push(self.expect_number()?);
                }
                self.expect_token(TokenKind::RBrace)?;
                Ok(Domain::Enumerated(values))
            }
            Some(TokenKind::Number(_)) | Some(TokenKind::Float(_)) | Some(TokenKind::Minus) => {
                // Range: 0..100 or 0.0..inf or -inf..inf
                let min = self.parse_bound()?;
                self.expect_token(TokenKind::DotDot)?;
                let max = self.parse_bound()?;
                // If both bounds are exact integers, use integer Range
                if min.fract() == 0.0 && max.fract() == 0.0 && min.is_finite() && max.is_finite() {
                    Ok(Domain::Range {
                        min: min as i64,
                        max: max as i64,
                    })
                } else {
                    Ok(Domain::RealRange { min, max })
                }
            }
            Some(TokenKind::Ident(s)) if s == "inf" || s == "free" => {
                let kw = s.clone();
                if kw == "free" {
                    self.advance();
                    return Ok(Domain::RealRange {
                        min: f64::NEG_INFINITY,
                        max: f64::INFINITY,
                    });
                }
                // inf..something or just inf as range start
                let min = self.parse_bound()?;
                self.expect_token(TokenKind::DotDot)?;
                let max = self.parse_bound()?;
                Ok(Domain::RealRange { min, max })
            }
            _ => Err(ParseError::new(
                "expected range (min..max), enumeration ({v1, v2, ...}), or 'free'",
                self.current_span(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::jia_lang::ast::{Domain, DomainStmt};
    use crate::jia_lang::lexer::tokenize;
    use crate::jia_lang::parser::Parser;

    fn parse_domains(input: &str) -> Vec<DomainStmt> {
        let tokens = tokenize(input).unwrap();
        let mut parser = Parser::new(&tokens);
        parser.parse_domains_block().unwrap()
    }

    #[test]
    fn test_fixed_duration() {
        let stmts = parse_domains("domains { duration(a) = 3 }");
        assert_eq!(
            stmts[0],
            DomainStmt::IntervalDuration {
                intervals: vec!["a".to_string()],
                domain: Domain::Fixed(3),
            }
        );
    }

    #[test]
    fn test_range_duration() {
        let stmts = parse_domains("domains { duration(a) in 1..4 }");
        assert_eq!(
            stmts[0],
            DomainStmt::IntervalDuration {
                intervals: vec!["a".to_string()],
                domain: Domain::Range { min: 1, max: 4 },
            }
        );
    }

    #[test]
    fn test_enumerated_duration() {
        let stmts = parse_domains("domains { duration(a) in {1, 3, 5} }");
        assert_eq!(
            stmts[0],
            DomainStmt::IntervalDuration {
                intervals: vec!["a".to_string()],
                domain: Domain::Enumerated(vec![1, 3, 5]),
            }
        );
    }

    #[test]
    fn test_multi_interval_attr() {
        let stmts = parse_domains("domains { start(a, b) in 0..20 }");
        assert_eq!(
            stmts[0],
            DomainStmt::IntervalStart {
                intervals: vec!["a".to_string(), "b".to_string()],
                domain: Domain::Range { min: 0, max: 20 },
            }
        );
    }

    #[test]
    fn test_optional() {
        let stmts = parse_domains("domains { optional(x, y, z) }");
        assert_eq!(
            stmts[0],
            DomainStmt::IntervalOptional {
                intervals: vec!["x".to_string(), "y".to_string(), "z".to_string()],
            }
        );
    }

    #[test]
    fn test_integer_domain() {
        let stmts = parse_domains("domains { makespan in 0..100 }");
        assert_eq!(
            stmts[0],
            DomainStmt::IntegerDomain {
                name: "makespan".to_string(),
                domain: Domain::Range { min: 0, max: 100 },
            }
        );
    }

    #[test]
    fn test_set_domain() {
        let stmts = parse_domains("domains { machine1 = {a, b, c} }");
        assert_eq!(
            stmts[0],
            DomainStmt::SetDomain {
                name: "machine1".to_string(),
                members: vec!["a".to_string(), "b".to_string(), "c".to_string()],
            }
        );
    }

    #[test]
    fn test_demand() {
        let stmts = parse_domains("domains { demand(a1, res1) = 2 }");
        assert_eq!(
            stmts[0],
            DomainStmt::Demand {
                interval: "a1".to_string(),
                set: "res1".to_string(),
                value: 2,
            }
        );
    }

    #[test]
    fn test_domain_error_branches() {
        for input in [
            "domains { 1 }",
            "domains { x }",
            "domains { duration(a) }",
            "domains { duration(a) = x }",
            "domains { x in nope }",
            "domains { x in 0..nope }",
            "domains { x in 0.. }",
        ] {
            let tokens = tokenize(input).unwrap();
            let mut parser = Parser::new(&tokens);
            assert!(parser.parse_domains_block().is_err(), "{input}");
        }
    }

    #[test]
    fn test_domain_spec_inf_range_start() {
        let stmts = parse_domains("domains { x in inf..10 }");
        assert!(matches!(
            stmts[0],
            DomainStmt::IntegerDomain {
                domain: Domain::RealRange { min, max },
                ..
            } if min.is_infinite() && min.is_sign_positive() && max == 10.0
        ));
    }
}
