//! Condition and goal parsing.

use crate::ast::{AtomicFormula, Condition};
use crate::error::ParseError;
use crate::lexer::TokenKind;

use super::cursor::Parser;
use super::expr::{is_numeric_start, parse_numeric_expr};
use super::terms::{parse_term, parse_typed_list_vars};
use super::utils::{parse_compare_op_from_str, parse_number_literal};

/// Parse a PDDL condition/goal description. Handles logical connectives, quantifiers, temporal
/// wrappers (`at start`/`at end`/`over all`), predicates (disambiguating `at` and `over` from
/// temporal keywords), numeric comparisons, equality, preferences, and PDDL3 trajectory constraints.
pub(super) fn parse_condition(p: &mut Parser) -> Result<Condition, ParseError> {
    if p.at_lparen() {
        p.expect_lparen()?;

        let tok = p
            .peek()
            .ok_or_else(|| ParseError::new("unexpected end of input in condition", p.eof_span()))?;

        match &tok.kind {
            TokenKind::Symbol(s) => {
                let s = s.clone();
                match s.as_str() {
                    "and" => {
                        p.advance()?;
                        let mut conds = Vec::new();
                        while !p.at_rparen() {
                            conds.push(parse_condition(p)?);
                        }
                        p.expect_rparen()?;
                        return Ok(Condition::And(conds));
                    }
                    "or" => {
                        p.advance()?;
                        let mut conds = Vec::new();
                        while !p.at_rparen() {
                            conds.push(parse_condition(p)?);
                        }
                        p.expect_rparen()?;
                        return Ok(Condition::Or(conds));
                    }
                    "not" => {
                        p.advance()?;
                        let inner = parse_condition(p)?;
                        p.expect_rparen()?;
                        return Ok(Condition::Not(Box::new(inner)));
                    }
                    "imply" => {
                        p.advance()?;
                        let a = parse_condition(p)?;
                        let b = parse_condition(p)?;
                        p.expect_rparen()?;
                        return Ok(Condition::Imply(Box::new(a), Box::new(b)));
                    }
                    "forall" => {
                        p.advance()?;
                        p.expect_lparen()?;
                        let variables = parse_typed_list_vars(p)?;
                        p.expect_rparen()?;
                        let condition = parse_condition(p)?;
                        p.expect_rparen()?;
                        return Ok(Condition::Forall {
                            variables,
                            condition: Box::new(condition),
                        });
                    }
                    "exists" => {
                        p.advance()?;
                        p.expect_lparen()?;
                        let variables = parse_typed_list_vars(p)?;
                        p.expect_rparen()?;
                        let condition = parse_condition(p)?;
                        p.expect_rparen()?;
                        return Ok(Condition::Exists {
                            variables,
                            condition: Box::new(condition),
                        });
                    }
                    "preference" => {
                        p.advance()?;
                        // Optional name
                        let name = if !p.at_lparen() && !p.at_rparen() {
                            Some(p.expect_symbol()?)
                        } else {
                            None
                        };
                        let condition = parse_condition(p)?;
                        p.expect_rparen()?;
                        return Ok(Condition::Preference {
                            name,
                            condition: Box::new(condition),
                        });
                    }
                    "at" => {
                        p.advance()?;
                        let tok2 = p.peek().ok_or_else(|| {
                            ParseError::new("unexpected end after 'at'", p.eof_span())
                        })?;
                        if tok2.symbol_eq("start") {
                            p.advance()?;
                            let inner = parse_condition(p)?;
                            p.expect_rparen()?;
                            return Ok(Condition::AtStart(Box::new(inner)));
                        } else if tok2.symbol_eq("end") {
                            p.advance()?;
                            let inner = parse_condition(p)?;
                            p.expect_rparen()?;
                            return Ok(Condition::AtEnd(Box::new(inner)));
                        } else {
                            // `at` is a regular predicate name, e.g. (at ?truck ?loc)
                            let mut args = Vec::new();
                            while !p.at_rparen() {
                                args.push(parse_term(p)?);
                            }
                            p.expect_rparen()?;
                            return Ok(Condition::Predicate(AtomicFormula { name: s, args }));
                        }
                    }
                    "over" => {
                        p.advance()?;
                        let next = p.peek().ok_or_else(|| {
                            ParseError::new("unexpected end after 'over'", p.eof_span())
                        })?;
                        if next.symbol_eq("all") {
                            p.advance()?;
                            let inner = parse_condition(p)?;
                            p.expect_rparen()?;
                            return Ok(Condition::OverAll(Box::new(inner)));
                        } else {
                            // `over` is a regular predicate name
                            let mut args = Vec::new();
                            while !p.at_rparen() {
                                args.push(parse_term(p)?);
                            }
                            p.expect_rparen()?;
                            return Ok(Condition::Predicate(AtomicFormula { name: s, args }));
                        }
                    }
                    "always" => {
                        p.advance()?;
                        let inner = parse_condition(p)?;
                        p.expect_rparen()?;
                        return Ok(Condition::Always(Box::new(inner)));
                    }
                    "sometime" => {
                        p.advance()?;
                        let inner = parse_condition(p)?;
                        p.expect_rparen()?;
                        return Ok(Condition::Sometime(Box::new(inner)));
                    }
                    "at-most-once" => {
                        p.advance()?;
                        let inner = parse_condition(p)?;
                        p.expect_rparen()?;
                        return Ok(Condition::AtMostOnce(Box::new(inner)));
                    }
                    "within" => {
                        p.advance()?;
                        let t = parse_number_literal(p)?;
                        let inner = parse_condition(p)?;
                        p.expect_rparen()?;
                        return Ok(Condition::Within(t, Box::new(inner)));
                    }
                    "sometime-before" => {
                        p.advance()?;
                        let a = parse_condition(p)?;
                        let b = parse_condition(p)?;
                        p.expect_rparen()?;
                        return Ok(Condition::SometimeBefore(Box::new(a), Box::new(b)));
                    }
                    "sometime-after" => {
                        p.advance()?;
                        let a = parse_condition(p)?;
                        let b = parse_condition(p)?;
                        p.expect_rparen()?;
                        return Ok(Condition::SometimeAfter(Box::new(a), Box::new(b)));
                    }
                    "always-within" => {
                        p.advance()?;
                        let t = parse_number_literal(p)?;
                        let a = parse_condition(p)?;
                        let b = parse_condition(p)?;
                        p.expect_rparen()?;
                        return Ok(Condition::AlwaysWithin(t, Box::new(a), Box::new(b)));
                    }
                    "hold-during" => {
                        p.advance()?;
                        let t1 = parse_number_literal(p)?;
                        let t2 = parse_number_literal(p)?;
                        let inner = parse_condition(p)?;
                        p.expect_rparen()?;
                        return Ok(Condition::HoldDuring(t1, t2, Box::new(inner)));
                    }
                    "hold-after" => {
                        p.advance()?;
                        let t = parse_number_literal(p)?;
                        let inner = parse_condition(p)?;
                        p.expect_rparen()?;
                        return Ok(Condition::HoldAfter(t, Box::new(inner)));
                    }
                    "=" => {
                        p.advance()?;
                        // Could be equality of terms or numeric comparison
                        // Peek ahead: if next is '(' or a number, it's numeric
                        if is_numeric_start(p) {
                            let left = parse_numeric_expr(p)?;
                            let right = parse_numeric_expr(p)?;
                            p.expect_rparen()?;
                            return Ok(Condition::NumericComparison {
                                op: crate::ast::CompareOp::Eq,
                                left,
                                right,
                            });
                        }
                        let a = parse_term(p)?;
                        let b = parse_term(p)?;
                        p.expect_rparen()?;
                        return Ok(Condition::Equals(a, b));
                    }
                    "<" | "<=" | ">" | ">=" => {
                        let op = parse_compare_op_from_str(&s);
                        p.advance()?;
                        let left = parse_numeric_expr(p)?;
                        let right = parse_numeric_expr(p)?;
                        p.expect_rparen()?;
                        return Ok(Condition::NumericComparison { op, left, right });
                    }
                    _ => {
                        // Predicate application: (name term*)
                        p.advance()?;
                        let mut args = Vec::new();
                        while !p.at_rparen() {
                            args.push(parse_term(p)?);
                        }
                        p.expect_rparen()?;
                        return Ok(Condition::Predicate(AtomicFormula { name: s, args }));
                    }
                }
            }
            _ => {
                return Err(ParseError::new(
                    format!("unexpected token in condition: {:?}", tok.kind),
                    tok.span,
                ));
            }
        }
    }

    Err(ParseError::new(
        "expected condition (must start with '(')",
        p.current_span(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{CompareOp, Term};
    use crate::lexer::tokenize;

    fn parse(input: &str) -> Result<Condition, ParseError> {
        let tokens = tokenize(input)?;
        let mut parser = Parser::new(&tokens);
        parse_condition(&mut parser)
    }

    #[test]
    fn parses_named_preference_and_temporal_conditions() {
        let condition =
            parse("(and (preference prefer-ready (ready a)) (at end (done a)))").unwrap();

        let Condition::And(conditions) = condition else {
            return;
        };

        assert!(matches!(
            &conditions[0],
            Condition::Preference {
                name: Some(name),
                ..
            } if name == "prefer-ready"
        ));
        assert!(matches!(&conditions[1], Condition::AtEnd(_)));
    }

    #[test]
    fn parses_all_numeric_comparison_operators() {
        for (input, expected) in [
            ("(< (cost) 5)", CompareOp::Lt),
            ("(<= (cost) 5)", CompareOp::Lte),
            ("(> (cost) 5)", CompareOp::Gt),
            ("(>= (cost) 5)", CompareOp::Gte),
        ] {
            let condition = parse(input).unwrap();
            assert!(matches!(
                condition,
                Condition::NumericComparison { op, .. } if op == expected
            ));
        }
    }

    #[test]
    fn parses_term_equality_with_variables() {
        let condition = parse("(= ?x target)").unwrap();

        assert!(matches!(
            condition,
            Condition::Equals(Term::Variable(variable), Term::Name(name))
                if variable == "?x" && name == "target"
        ));
    }

    #[test]
    fn reports_condition_edge_errors() {
        for input in [
            "(",
            "(and (ready a)",
            "(preference name)",
            "(at",
            "(over",
            "(<=)",
            "(ready 1)",
            "ready",
        ] {
            assert!(parse(input).is_err(), "{input}");
        }
    }
}
