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
    use crate::ast::{AssignOp, Condition, DurationConstraint, Effect, Optimization};

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
    fn test_parse_functions_derived_and_duration_inequalities() {
        let input = r#"
(define (domain test)
  (:requirements :typing :numeric-fluents :durative-actions :derived-predicates)
  (:predicates (ready ?x - obj))
  (:functions (fuel ?x - obj) (total-cost) - number)
  (:derived (available ?x - obj) (ready ?x))
  (:durative-action wait
    :parameters (?x - obj)
    :duration (and (>= ?duration 1) (<= ?duration 5))
    :condition (and (at start (ready ?x)))
    :effect (and))
)
"#;

        let domain = parse_domain_str(input).unwrap();

        assert_eq!(domain.functions.len(), 2);
        assert_eq!(domain.derived_predicates.len(), 1);
        assert!(matches!(
            domain.durative_actions[0].duration,
            DurationConstraint::And(_)
        ));
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

    #[test]
    fn test_parse_broad_condition_variants() {
        let input = r#"
(define (problem test)
  (:domain d)
  (:init)
  (:goal
    (and
      (or (ready a) (ready b))
      (not (blocked a))
      (imply (ready a) (ready b))
      (forall (?x - obj) (ready ?x))
      (exists (?x - obj) (ready ?x))
      (preference (ready a))
      (at a loc)
      (over a b)
      (= a b)
      (= (cost) 0)
      (< (cost) 10)
      (> (cost) 0)
      (always (ready a))
      (sometime (ready b))
      (at-most-once (ready a))
      (within 5 (ready a))
      (sometime-before (ready a) (ready b))
      (sometime-after (ready a) (ready b))
      (always-within 3 (ready a) (ready b))
      (hold-during 1 2 (ready a))
      (hold-after 4 (ready a))
    ))
)
"#;

        let problem = parse_problem_str(input).unwrap();
        let conditions = match &problem.goal {
            Condition::And(conditions) => Some(conditions),
            _ => None,
        }
        .unwrap();

        assert!(conditions.iter().any(|c| matches!(c, Condition::Or(_))));
        assert!(conditions.iter().any(|c| matches!(c, Condition::Not(_))));
        assert!(conditions
            .iter()
            .any(|c| matches!(c, Condition::Imply(_, _))));
        assert!(conditions
            .iter()
            .any(|c| matches!(c, Condition::Forall { .. })));
        assert!(conditions
            .iter()
            .any(|c| matches!(c, Condition::Exists { .. })));
        assert!(conditions
            .iter()
            .any(|c| matches!(c, Condition::Preference { name: None, .. })));
        assert!(conditions.iter().any(|c| matches!(
            c,
            Condition::Predicate(predicate) if predicate.name == "at"
        )));
        assert!(conditions.iter().any(|c| matches!(
            c,
            Condition::Predicate(predicate) if predicate.name == "over"
        )));
        assert!(conditions
            .iter()
            .any(|c| matches!(c, Condition::Equals(_, _))));
        assert!(conditions
            .iter()
            .any(|c| matches!(c, Condition::NumericComparison { .. })));
        assert!(conditions.iter().any(|c| matches!(c, Condition::Always(_))));
        assert!(conditions
            .iter()
            .any(|c| matches!(c, Condition::Sometime(_))));
        assert!(conditions
            .iter()
            .any(|c| matches!(c, Condition::AtMostOnce(_))));
        assert!(conditions
            .iter()
            .any(|c| matches!(c, Condition::Within(_, _))));
        assert!(conditions
            .iter()
            .any(|c| matches!(c, Condition::SometimeBefore(_, _))));
        assert!(conditions
            .iter()
            .any(|c| matches!(c, Condition::SometimeAfter(_, _))));
        assert!(conditions
            .iter()
            .any(|c| matches!(c, Condition::AlwaysWithin(_, _, _))));
        assert!(conditions
            .iter()
            .any(|c| matches!(c, Condition::HoldDuring(_, _, _))));
        assert!(conditions
            .iter()
            .any(|c| matches!(c, Condition::HoldAfter(_, _))));
    }

    #[test]
    fn test_parse_broad_effect_variants() {
        let input = r#"
(define (domain test)
  (:action a
    :parameters (?x - obj)
    :precondition (and)
    :effect
      (and
        (at ?x loc)
        (assign (cost) 1)
        (increase (cost) 2)
        (scale-up (cost) 3)
        (when (ready ?x) (decrease (cost) 1))))
  (:durative-action d
    :parameters (?x - obj)
    :duration (= ?duration 1)
    :condition (and)
    :effect
      (and
        (at start (at ?x loc))
        (at end (scale-down (cost) 2))))
)
"#;

        let domain = parse_domain_str(input).unwrap();
        let effects = domain.actions[0]
            .effect
            .as_ref()
            .and_then(|effect| match effect {
                Effect::And(effects) => Some(effects),
                _ => None,
            })
            .unwrap();

        assert!(effects.iter().any(|e| matches!(
            e,
            Effect::Predicate(predicate) if predicate.name == "at"
        )));
        assert!(effects.iter().any(|e| matches!(
            e,
            Effect::NumericAssign {
                op: AssignOp::Assign,
                ..
            }
        )));
        assert!(effects.iter().any(|e| matches!(
            e,
            Effect::NumericAssign {
                op: AssignOp::Increase,
                ..
            }
        )));
        assert!(effects.iter().any(|e| matches!(
            e,
            Effect::NumericAssign {
                op: AssignOp::ScaleUp,
                ..
            }
        )));
        assert!(effects.iter().any(|e| matches!(e, Effect::When { .. })));

        let durative_effects = domain.durative_actions[0]
            .effect
            .as_ref()
            .and_then(|effect| match effect {
                Effect::And(effects) => Some(effects),
                _ => None,
            })
            .unwrap();
        assert!(durative_effects
            .iter()
            .any(|e| matches!(e, Effect::AtStart(_))));
        assert!(durative_effects
            .iter()
            .any(|e| matches!(e, Effect::AtEnd(_))));
    }

    #[test]
    fn parser_skips_unknown_domain_and_problem_sections() {
        let domain = parse_domain_str(
            r#"
(define (domain test)
  (:requirements :strips)
  (:unknown-section (nested value))
  (legacy-section (ignored value))
  (:action a
    :parameters ()
    :precondition (and)
    :unknown-action-field (ignored value)
    :effect (and))
  (:durative-action d
    :parameters ()
    :duration (= ?duration 1)
    :unknown-durative-field (ignored value)
    :condition (and)
    :effect (and))
)
"#,
        )
        .unwrap();
        assert_eq!(domain.actions.len(), 1);
        assert_eq!(domain.durative_actions.len(), 1);

        let problem = parse_problem_str(
            r#"
(define (problem p)
  (:domain test)
  (:unknown-section (nested value))
  (legacy-section (ignored value))
  (:init)
  (:goal (and))
)
"#,
        )
        .unwrap();
        assert_eq!(problem.domain_name, "test");
    }

    #[test]
    fn parser_skips_unknown_action_fields_with_sexp_values() {
        let domain = parse_domain_str(
            r#"
(define (domain test)
  (:action a
    :parameters ()
    :precondition (and)
    :unknown-action-field (ignored value)
    :effect (and))
  (:durative-action d
    :parameters ()
    :duration (= ?duration 1)
    :condition (and)
    :unknown-durative-field (ignored value)
    :effect (and))
)
"#,
        )
        .unwrap();

        assert_eq!(domain.actions.len(), 1);
        assert_eq!(domain.durative_actions.len(), 1);
    }

    #[test]
    fn parser_reports_malformed_conditions_effects_and_metrics() {
        for input in [
            "(define (problem p) (:domain d) (:goal 42))",
            "(define (problem p) (:domain d) (:goal (42)))",
            "(define (problem p) (:domain d) (:goal (> ?x 1)))",
            "(define (problem p) (:domain d) (:goal (at",
            "(define (problem p) (:domain d) (:goal (over",
            "(define (problem p) (:domain d) (:goal (and)) (:metric fastest 1))",
            "(define (problem p) (:domain d) (:goal (and)) (:metric minimize ?x))",
            "(define (problem p) (:domain d) (:goal (and)) (:metric minimize (42)))",
        ] {
            assert!(parse_problem_str(input).is_err(), "{input}");
        }

        for input in [
            "(define (domain d) (:action a :parameters () :precondition (and) :effect (42)))",
            "(define (domain d) (:action a :parameters () :precondition (and) :effect (at",
            "(define (domain d) (:action a :parameters () :precondition (and) :effect (increase (cost) ?x)))",
        ] {
            assert!(parse_domain_str(input).is_err(), "{input}");
        }
    }
}
