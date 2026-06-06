//! Domain parsing.

use crate::ast::*;
use crate::error::ParseError;

use super::condition::parse_condition;
use super::cursor::Parser;
use super::effect::parse_effect;
use super::expr::parse_numeric_expr;
use super::terms::{parse_atomic_formula, parse_typed_list_names, parse_typed_list_vars};
use super::utils::{parse_compare_op, skip_sexp, skip_to_define};

/// Internal: parse the body of a `(define (domain ...))` form.
pub(super) fn parse_domain_def(p: &mut Parser) -> Result<Domain, ParseError> {
    skip_to_define(p)?;

    p.expect_lparen()?; // (
    p.expect_symbol_eq("domain")?; // domain
    let name = p.expect_symbol()?; // <name>
    p.expect_rparen()?; // )

    let mut domain = Domain {
        name,
        requirements: Vec::new(),
        types: Vec::new(),
        constants: Vec::new(),
        predicates: Vec::new(),
        functions: Vec::new(),
        actions: Vec::new(),
        durative_actions: Vec::new(),
        derived_predicates: Vec::new(),
    };

    while p.at_lparen() {
        let save = p.pos;
        p.expect_lparen()?;
        let tok = p.advance()?;

        match &tok.kind {
            crate::lexer::TokenKind::Keyword(kw) => {
                let kw = kw.clone();
                match kw.as_str() {
                    "requirements" => domain.requirements = parse_requirements(p)?,
                    "types" => domain.types = parse_type_declarations(p)?,
                    "constants" => domain.constants = parse_typed_list_names(p)?,
                    "predicates" => domain.predicates = parse_predicate_decls(p)?,
                    "functions" => domain.functions = parse_function_decls(p)?,
                    "action" => {
                        domain.actions.push(parse_basic_action(p)?);
                    }
                    "durative-action" => {
                        domain.durative_actions.push(parse_durative_action(p)?);
                    }
                    "derived" => {
                        domain.derived_predicates.push(parse_derived_predicate(p)?);
                    }
                    _ => {
                        // Unknown section -- skip to matching rparen
                        p.pos = save;
                        skip_sexp(p)?;
                        continue;
                    }
                }
            }
            _ => {
                // Not a keyword section -- skip
                p.pos = save;
                skip_sexp(p)?;
                continue;
            }
        }

        p.expect_rparen()?;
    }

    p.expect_rparen()?; // closing ) of define

    Ok(domain)
}

/// Parse the `:requirements` keyword list.
pub(super) fn parse_requirements(p: &mut Parser) -> Result<Vec<Requirement>, ParseError> {
    let mut reqs = Vec::new();
    while p.at_keyword_any() {
        let kw = p.expect_keyword()?;
        let req = match kw.as_str() {
            "strips" => Requirement::Strips,
            "typing" => Requirement::Typing,
            "negative-preconditions" => Requirement::NegativePreconditions,
            "disjunctive-preconditions" => Requirement::DisjunctivePreconditions,
            "equality" => Requirement::Equality,
            "existential-preconditions" => Requirement::ExistentialPreconditions,
            "universal-preconditions" => Requirement::UniversalPreconditions,
            "quantified-preconditions" => Requirement::QuantifiedPreconditions,
            "conditional-effects" => Requirement::ConditionalEffects,
            "fluents" => Requirement::Fluents,
            "numeric-fluents" => Requirement::NumericFluents,
            "adl" => Requirement::Adl,
            "durative-actions" => Requirement::DurativeActions,
            "duration-inequalities" => Requirement::DurationInequalities,
            "timed-initial-literals" => Requirement::TimedInitialLiterals,
            "preferences" => Requirement::Preferences,
            "constraints" => Requirement::Constraints,
            "action-costs" => Requirement::ActionCosts,
            "goal-utilities" => Requirement::GoalUtilities,
            "derived-predicates" => Requirement::DerivedPredicates,
            "domain-axioms" => Requirement::DomainAxioms,
            _ => {
                continue;
            }
        };
        reqs.push(req);
    }
    Ok(reqs)
}

/// Parse type declarations: `type1 type2 - parent_type type3`
fn parse_type_declarations(p: &mut Parser) -> Result<TypeDeclarations, ParseError> {
    parse_typed_list_names(p)
}

/// Parse the `:predicates` section.
fn parse_predicate_decls(p: &mut Parser) -> Result<Vec<PredicateDecl>, ParseError> {
    let mut decls = Vec::new();
    while p.at_lparen() {
        p.expect_lparen()?;
        let name = p.expect_symbol()?;
        let parameters = parse_typed_list_vars(p)?;
        p.expect_rparen()?;
        decls.push(PredicateDecl { name, parameters });
    }
    Ok(decls)
}

/// Parse the `:functions` section with optional return types.
fn parse_function_decls(p: &mut Parser) -> Result<Vec<FunctionDecl>, ParseError> {
    let mut decls = Vec::new();
    while p.at_lparen() {
        p.expect_lparen()?;
        let name = p.expect_symbol()?;
        let parameters = parse_typed_list_vars(p)?;
        p.expect_rparen()?;

        // Optional return type: `- number` or `- type`
        let return_type = if p.at_symbol("-") {
            p.advance()?;
            Some(p.expect_symbol()?)
        } else {
            None
        };

        decls.push(FunctionDecl {
            name,
            parameters,
            return_type,
        });
    }
    Ok(decls)
}

/// Parse a `:action` definition including PDDL 1.2 `:vars` support.
fn parse_basic_action(p: &mut Parser) -> Result<BasicAction, ParseError> {
    let name = p.expect_symbol()?;

    let mut parameters = Vec::new();
    let mut precondition = None;
    let mut effect = None;

    while p.at_keyword_any() {
        let kw = p.expect_keyword()?;
        match kw.as_str() {
            "parameters" => {
                p.expect_lparen()?;
                parameters = parse_typed_list_vars(p)?;
                p.expect_rparen()?;
            }
            "vars" => {
                // PDDL 1.2 :vars -- extra universally-quantified parameters
                p.expect_lparen()?;
                let vars = parse_typed_list_vars(p)?;
                parameters.extend(vars);
                p.expect_rparen()?;
            }
            "precondition" => {
                precondition = Some(parse_condition(p)?);
            }
            "effect" => {
                effect = Some(parse_effect(p)?);
            }
            _ => {
                // Skip unknown keyword's value
                if p.at_lparen() {
                    skip_sexp(p)?;
                }
            }
        }
    }

    Ok(BasicAction {
        name,
        parameters,
        precondition,
        effect,
    })
}

/// Parse a `:durative-action` definition with duration, condition, and effect.
fn parse_durative_action(p: &mut Parser) -> Result<DurativeAction, ParseError> {
    let name = p.expect_symbol()?;

    let mut parameters = Vec::new();
    let mut duration = DurationConstraint::Cmp {
        op: CompareOp::Eq,
        expr: NumericExpr::Number(0.0),
    };
    let mut condition = None;
    let mut effect = None;

    while p.at_keyword_any() {
        let kw = p.expect_keyword()?;
        match kw.as_str() {
            "parameters" => {
                p.expect_lparen()?;
                parameters = parse_typed_list_vars(p)?;
                p.expect_rparen()?;
            }
            "duration" => {
                duration = parse_duration_constraint(p)?;
            }
            "condition" => {
                condition = Some(parse_condition(p)?);
            }
            "effect" => {
                effect = Some(parse_effect(p)?);
            }
            _ => {
                if p.at_lparen() {
                    skip_sexp(p)?;
                }
            }
        }
    }

    Ok(DurativeAction {
        name,
        parameters,
        duration,
        condition,
        effect,
    })
}

/// Parse a duration constraint, which may be a single comparison or a conjunction.
fn parse_duration_constraint(p: &mut Parser) -> Result<DurationConstraint, ParseError> {
    p.expect_lparen()?;

    if p.at_symbol("and") {
        p.advance()?;
        let mut constraints = Vec::new();
        while !p.at_rparen() {
            constraints.push(parse_duration_constraint_inner(p)?);
        }
        p.expect_rparen()?;
        return Ok(DurationConstraint::And(constraints));
    }

    // Single constraint: (op ?duration expr)
    let op = parse_compare_op(p)?;
    p.expect_variable()?; // ?duration
    let expr = parse_numeric_expr(p)?;
    p.expect_rparen()?;

    Ok(DurationConstraint::Cmp { op, expr })
}

/// Parse a single `(op ?duration expr)` duration constraint.
fn parse_duration_constraint_inner(p: &mut Parser) -> Result<DurationConstraint, ParseError> {
    p.expect_lparen()?;
    let op = parse_compare_op(p)?;
    p.expect_variable()?; // ?duration
    let expr = parse_numeric_expr(p)?;
    p.expect_rparen()?;
    Ok(DurationConstraint::Cmp { op, expr })
}

/// Parse a `:derived` predicate with its axiom body.
fn parse_derived_predicate(p: &mut Parser) -> Result<DerivedPredicate, ParseError> {
    let predicate = parse_atomic_formula(p)?;
    let condition = parse_condition(p)?;
    Ok(DerivedPredicate {
        predicate,
        condition,
    })
}
