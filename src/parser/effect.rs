//! Effect parsing.

use crate::ast::{AssignOp, AtomicFormula, Effect};
use crate::error::ParseError;
use crate::lexer::TokenKind;

use super::condition::parse_condition;
use super::cursor::Parser;
use super::expr::parse_numeric_expr;
use super::terms::{parse_atomic_formula, parse_function_term, parse_term};

fn assign_op_from_symbol(symbol: &str) -> Option<AssignOp> {
    match symbol {
        "assign" => Some(AssignOp::Assign),
        "increase" => Some(AssignOp::Increase),
        "decrease" => Some(AssignOp::Decrease),
        "scale-up" => Some(AssignOp::ScaleUp),
        "scale-down" => Some(AssignOp::ScaleDown),
        _ => None,
    }
}

/// Parse a PDDL effect. Handles conjunction, negation, forall, when (conditional), numeric
/// assignment, temporal wrappers, and predicates (disambiguating `at` from temporal keyword).
pub(super) fn parse_effect(p: &mut Parser) -> Result<Effect, ParseError> {
    p.expect_lparen()?;

    let tok = p
        .peek()
        .ok_or_else(|| ParseError::new("unexpected end of input in effect", p.eof_span()))?;

    match &tok.kind {
        TokenKind::Symbol(s) => {
            let s = s.clone();
            match s.as_str() {
                "and" => {
                    p.advance()?;
                    let mut effects = Vec::new();
                    while !p.at_rparen() {
                        effects.push(parse_effect(p)?);
                    }
                    p.expect_rparen()?;
                    Ok(Effect::And(effects))
                }
                "not" => {
                    p.advance()?;
                    let af = parse_atomic_formula(p)?;
                    p.expect_rparen()?;
                    Ok(Effect::NotPredicate(af))
                }
                "forall" => {
                    p.advance()?;
                    p.expect_lparen()?;
                    let variables = super::terms::parse_typed_list_vars(p)?;
                    p.expect_rparen()?;
                    let effect = parse_effect(p)?;
                    p.expect_rparen()?;
                    Ok(Effect::Forall {
                        variables,
                        effect: Box::new(effect),
                    })
                }
                "when" => {
                    p.advance()?;
                    let condition = parse_condition(p)?;
                    let effect = parse_effect(p)?;
                    p.expect_rparen()?;
                    Ok(Effect::When {
                        condition,
                        effect: Box::new(effect),
                    })
                }
                "at" => {
                    p.advance()?;
                    let tok2 = p.peek().ok_or_else(|| {
                        ParseError::new("unexpected end after 'at' in effect", p.eof_span())
                    })?;
                    if tok2.symbol_eq("start") {
                        p.advance()?;
                        let inner = parse_effect(p)?;
                        p.expect_rparen()?;
                        Ok(Effect::AtStart(Box::new(inner)))
                    } else if tok2.symbol_eq("end") {
                        p.advance()?;
                        let inner = parse_effect(p)?;
                        p.expect_rparen()?;
                        Ok(Effect::AtEnd(Box::new(inner)))
                    } else {
                        // `at` is a regular predicate name
                        let mut args = Vec::new();
                        while !p.at_rparen() {
                            args.push(parse_term(p)?);
                        }
                        p.expect_rparen()?;
                        Ok(Effect::Predicate(AtomicFormula { name: s, args }))
                    }
                }
                _ => {
                    if let Some(op) = assign_op_from_symbol(&s) {
                        p.advance()?;
                        let function = parse_function_term(p)?;
                        let expr = parse_numeric_expr(p)?;
                        p.expect_rparen()?;
                        return Ok(Effect::NumericAssign { op, function, expr });
                    }

                    // Positive predicate effect
                    p.advance()?;
                    let mut args = Vec::new();
                    while !p.at_rparen() {
                        args.push(parse_term(p)?);
                    }
                    p.expect_rparen()?;
                    Ok(Effect::Predicate(AtomicFormula { name: s, args }))
                }
            }
        }
        _ => Err(ParseError::new(
            format!("unexpected token in effect: {:?}", tok.kind),
            tok.span,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;

    fn parse(input: &str) -> Result<Effect, ParseError> {
        let tokens = tokenize(input)?;
        let mut parser = Parser::new(&tokens);
        parse_effect(&mut parser)
    }

    #[test]
    fn parses_quantified_negated_and_temporal_effects() {
        let effect = parse(
            "(and (forall (?x - obj) (not (ready ?x))) (at start (ready a)) (at end (done a)))",
        )
        .unwrap();

        let Effect::And(effects) = effect else {
            return;
        };

        assert!(matches!(&effects[0], Effect::Forall { .. }));
        assert!(matches!(&effects[1], Effect::AtStart(_)));
        assert!(matches!(&effects[2], Effect::AtEnd(_)));
    }

    #[test]
    fn parses_all_numeric_assignment_ops() {
        for input in [
            "(assign (cost) 1)",
            "(increase (cost) 1)",
            "(decrease (cost) 1)",
            "(scale-up (cost) 1)",
            "(scale-down (cost) 1)",
        ] {
            assert!(matches!(
                parse(input).unwrap(),
                Effect::NumericAssign { .. }
            ));
        }
    }

    #[test]
    fn reports_effect_edge_errors() {
        for input in [
            "(",
            "(not)",
            "(forall (?x - obj))",
            "(when (ready a))",
            "(at",
            "(ready 1)",
            "ready",
        ] {
            assert!(parse(input).is_err(), "{input}");
        }
    }
}
