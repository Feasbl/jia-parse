//! Recursive-descent parser for PDDL domain and problem files.
//!
//! Consumes a token stream from the [`crate::lexer`] and produces the AST defined in
//! [`crate::ast`]. The parser handles all PDDL features encountered in IPC benchmarks
//! (1998--2014), including:
//!
//! - Typing and type hierarchies (`:types`, `either`)
//! - Durative actions with temporal conditions/effects
//! - Numeric fluents, action costs, and metrics
//! - ADL quantifiers (`forall`, `exists`) and conditional effects (`when`)
//! - PDDL3 trajectory constraints and preferences
//! - Legacy PDDL 1.2 `:vars` in actions
//!
//! ## Entry points
//!
//! | Function | Input | Output |
//! |---|---|---|
//! | [`parse_domain_str`] | raw PDDL string | [`Domain`] |
//! | [`parse_problem_str`] | raw PDDL string | [`Problem`] |
//! | [`parse_domain`] | pre-tokenized `&[Token]` | [`Domain`] |
//! | [`parse_problem`] | pre-tokenized `&[Token]` | [`Problem`] |

mod condition;
mod cursor;
mod domain;
mod effect;
mod expr;
mod problem;
mod terms;
mod utils;

use crate::ast::{Domain, Problem};
use crate::error::ParseError;
use crate::lexer::Token;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse a pre-tokenized PDDL domain.
///
/// Prefer [`parse_domain_str`] unless you need to inspect or reuse the token stream.
///
/// # Arguments
///
/// * `tokens` - Token stream produced by [`crate::lexer::tokenize`]
///
/// # Returns
///
/// A [`Domain`] AST on success.
///
/// # Errors
///
/// Returns [`ParseError`] if the token stream does not represent a valid PDDL domain.
pub fn parse_domain(tokens: &[Token]) -> Result<Domain, ParseError> {
    let mut p = cursor::Parser::new(tokens);
    let mut domain = domain::parse_domain_def(&mut p)?;
    domain.sort_alphabetically();
    Ok(domain)
}

/// Parse a pre-tokenized PDDL problem.
///
/// Prefer [`parse_problem_str`] unless you need to inspect or reuse the token stream.
///
/// # Arguments
///
/// * `tokens` - Token stream produced by [`crate::lexer::tokenize`]
///
/// # Returns
///
/// A [`Problem`] AST on success.
///
/// # Errors
///
/// Returns [`ParseError`] if the token stream does not represent a valid PDDL problem.
pub fn parse_problem(tokens: &[Token]) -> Result<Problem, ParseError> {
    let mut p = cursor::Parser::new(tokens);
    let mut problem = problem::parse_problem_def(&mut p)?;
    problem.sort_alphabetically();
    Ok(problem)
}

/// Tokenize and parse a PDDL domain from a raw source string.
///
/// This is the primary convenience entry point for domain parsing. It chains
/// [`crate::lexer::tokenize`] and [`parse_domain`] in one call.
///
/// # Arguments
///
/// * `input` - The raw PDDL domain source text
///
/// # Returns
///
/// A [`Domain`] AST on success.
///
/// # Errors
///
/// Returns [`ParseError`] on tokenization failures (e.g. unexpected characters)
/// or parse failures (e.g. malformed S-expressions, unknown sections).
pub fn parse_domain_str(input: &str) -> Result<Domain, ParseError> {
    let tokens = crate::lexer::tokenize(input)?;
    parse_domain(&tokens)
}

/// Tokenize and parse a PDDL problem from a raw source string.
///
/// This is the primary convenience entry point for problem parsing. It chains
/// [`crate::lexer::tokenize`] and [`parse_problem`] in one call.
///
/// # Arguments
///
/// * `input` - The raw PDDL problem source text
///
/// # Returns
///
/// A [`Problem`] AST on success.
///
/// # Errors
///
/// Returns [`ParseError`] on tokenization failures or parse failures.
pub fn parse_problem_str(input: &str) -> Result<Problem, ParseError> {
    let tokens = crate::lexer::tokenize(input)?;
    parse_problem(&tokens)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Optimization;

    #[test]
    fn test_parse_minimal_domain() {
        let input = r#"
(define (domain test)
  (:requirements :strips :typing)
  (:types block - object)
  (:predicates (on ?x - block ?y - block) (clear ?x - block))
)
"#;
        let domain = parse_domain_str(input).unwrap();
        assert_eq!(domain.name, "test");
        assert_eq!(domain.requirements.len(), 2);
        assert_eq!(domain.predicates.len(), 2);
        assert_eq!(domain.predicates[0].name, "clear");
        assert_eq!(domain.predicates[1].name, "on");
    }

    #[test]
    fn test_parse_durative_action() {
        let input = r#"
(define (domain test)
  (:durative-action move
    :parameters (?x - obj)
    :duration (= ?duration 5)
    :condition (and
      (at start (clear ?x))
      (over all (safe ?x))
    )
    :effect (and
      (at start (not (clear ?x)))
      (at end (moved ?x))
    )
  )
)
"#;
        let domain = parse_domain_str(input).unwrap();
        assert_eq!(domain.durative_actions.len(), 1);
        let da = &domain.durative_actions[0];
        assert_eq!(da.name, "move");
    }

    #[test]
    fn test_parse_minimal_problem() {
        let input = r#"
(define (problem test-prob)
  (:domain test)
  (:objects a b - block)
  (:init (on a b) (clear a) (= (cost) 0))
  (:goal (and (on b a)))
)
"#;
        let problem = parse_problem_str(input).unwrap();
        assert_eq!(problem.name, "test-prob");
        assert_eq!(problem.domain_name, "test");
        assert_eq!(problem.init.len(), 3);
    }

    #[test]
    fn test_parse_metric() {
        let input = r#"
(define (problem test)
  (:domain d)
  (:init)
  (:goal (and))
  (:metric minimize (total-time))
)
"#;
        let problem = parse_problem_str(input).unwrap();
        assert!(problem.metric.is_some());
        let m = problem.metric.unwrap();
        assert_eq!(m.optimization, Optimization::Minimize);
    }

    #[test]
    fn test_parse_numeric_precondition() {
        let input = r#"
(define (domain test)
  (:durative-action act
    :parameters (?d - driver)
    :duration (= ?duration 10)
    :condition (and
      (at start (>= (time_available ?d) 10))
    )
    :effect (and
      (at start (decrease (time_available ?d) 10))
    )
  )
)
"#;
        let domain = parse_domain_str(input).unwrap();
        let da = &domain.durative_actions[0];
        assert!(da.condition.is_some());
        assert!(da.effect.is_some());
    }
}
