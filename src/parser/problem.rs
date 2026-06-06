//! Problem parsing.

use crate::ast::*;
use crate::error::ParseError;

use super::condition::parse_condition;
use super::cursor::Parser;
use super::expr::parse_metric_expr;
use super::terms::{
    parse_atomic_formula, parse_function_term, parse_term_name_only, parse_typed_list_names,
};
use super::utils::{parse_number_literal, skip_sexp, skip_to_define};

/// Internal: parse the body of a `(define (problem ...))` form.
pub(super) fn parse_problem_def(p: &mut Parser) -> Result<Problem, ParseError> {
    skip_to_define(p)?;

    p.expect_lparen()?; // (
    p.expect_symbol_eq("problem")?; // problem
    let name = p.expect_symbol()?; // <name>
    p.expect_rparen()?; // )

    let mut problem = Problem {
        name,
        domain_name: String::new(),
        requirements: Vec::new(),
        objects: Vec::new(),
        init: Vec::new(),
        goal: Condition::And(Vec::new()),
        metric: None,
        constraints: None,
    };

    while p.at_lparen() {
        let save = p.pos;
        p.expect_lparen()?;
        let tok = p.advance()?;

        match &tok.kind {
            crate::lexer::TokenKind::Keyword(kw) => {
                let kw = kw.clone();
                match kw.as_str() {
                    "domain" => {
                        problem.domain_name = p.expect_symbol()?;
                    }
                    "requirements" => {
                        problem.requirements = super::domain::parse_requirements(p)?;
                    }
                    "objects" => {
                        problem.objects = parse_typed_list_names(p)?;
                    }
                    "init" => {
                        problem.init = parse_init(p)?;
                    }
                    "goal" => {
                        problem.goal = parse_condition(p)?;
                    }
                    "metric" => {
                        problem.metric = Some(parse_metric(p)?);
                    }
                    "constraints" => {
                        problem.constraints = Some(parse_condition(p)?);
                    }
                    _ => {
                        p.pos = save;
                        skip_sexp(p)?;
                        continue;
                    }
                }
            }
            _ => {
                p.pos = save;
                skip_sexp(p)?;
                continue;
            }
        }

        p.expect_rparen()?;
    }

    p.expect_rparen()?;

    Ok(problem)
}

/// Parse the `:init` section of a problem.
fn parse_init(p: &mut Parser) -> Result<Vec<InitElement>, ParseError> {
    let mut elements = Vec::new();
    while !p.at_rparen() {
        elements.push(parse_init_element(p)?);
    }
    Ok(elements)
}

/// Parse a single init element (predicate, numeric assignment, negation, or timed literal).
/// Disambiguates `at` as a timed-literal keyword vs. predicate name by lookahead.
fn parse_init_element(p: &mut Parser) -> Result<InitElement, ParseError> {
    p.expect_lparen()?;

    // Check for `=` (numeric assignment)
    if p.at_symbol("=") {
        p.advance()?;
        let func = parse_function_term(p)?;
        let val = parse_number_literal(p)?;
        p.expect_rparen()?;
        return Ok(InitElement::NumericAssignment(func, val));
    }

    // Check for `not`
    if p.at_symbol("not") {
        p.advance()?;
        let af = parse_atomic_formula(p)?;
        p.expect_rparen()?;
        return Ok(InitElement::NotPredicate(af));
    }

    // Check for `at` (timed initial literal)
    if p.at_symbol("at") {
        let save = p.pos;
        p.advance()?; // consume `at`

        // If followed by a number, this is a timed initial literal
        if p.at_number() {
            let time = parse_number_literal(p)?;
            let inner = parse_init_element(p)?;
            p.expect_rparen()?;
            return Ok(InitElement::At(time, Box::new(inner)));
        }

        // Otherwise, `at` is a predicate name -- backtrack
        p.pos = save;
    }

    // Regular predicate
    let name = p.expect_symbol()?;
    let mut args = Vec::new();
    while !p.at_rparen() {
        args.push(parse_term_name_only(p)?);
    }
    p.expect_rparen()?;

    Ok(InitElement::Predicate(AtomicFormula { name, args }))
}

/// Parse the `:metric` specification (minimize/maximize + expression).
fn parse_metric(p: &mut Parser) -> Result<MetricSpec, ParseError> {
    let opt_sym = p.expect_symbol()?;
    let optimization = match opt_sym.as_str() {
        "minimize" => Optimization::Minimize,
        "maximize" => Optimization::Maximize,
        other => {
            return Err(ParseError::new(
                format!("expected 'minimize' or 'maximize', got '{other}'"),
                p.current_span(),
            ));
        }
    };
    let expr = parse_metric_expr(p)?;
    Ok(MetricSpec { optimization, expr })
}
