//! Abstract Syntax Tree (AST) for PDDL domain and problem files.
//!
//! Every type here is produced by the parser (see [`super::parser`]) and consumed by the grounder
//! (see [`crate::grounder`]). The AST is a direct structural representation of the PDDL syntax:
//! no semantic analysis, type checking, or grounding is performed at this stage.

use serde::Serialize;

// ---------------------------------------------------------------------------
// Names & identifiers
// ---------------------------------------------------------------------------

/// A symbol name: predicate/action/object names, type names, etc.
pub type Name = String;

/// A variable reference including the leading `?` (e.g. `?x`, `?from`).
pub type Variable = String; // includes the leading '?'

// ---------------------------------------------------------------------------
// Requirements
// ---------------------------------------------------------------------------

/// PDDL requirement flags declared in the `:requirements` section of a domain.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub enum Requirement {
    Strips,
    Typing,
    NegativePreconditions,
    DisjunctivePreconditions,
    Equality,
    ExistentialPreconditions,
    UniversalPreconditions,
    QuantifiedPreconditions,
    ConditionalEffects,
    Fluents,
    NumericFluents,
    Adl,
    DurativeActions,
    DurationInequalities,
    TimedInitialLiterals,
    Preferences,
    Constraints,
    ActionCosts,
    GoalUtilities,
    DerivedPredicates,
    DomainAxioms,
}

// ---------------------------------------------------------------------------
// Typed lists: "?x ?y - type1 ?z - type2"
// ---------------------------------------------------------------------------

/// A group of items sharing a common PDDL type.
///
/// For example, `?x ?y - location` is represented as:
///
/// ```text
/// TypedGroup { items: ["?x", "?y"], type_name: Some("location") }
/// ```
///
/// When no type is specified (untyped PDDL), `type_name` is `None`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TypedGroup<T: Serialize> {
    /// The items sharing this type.
    pub items: Vec<T>,
    /// The PDDL type name, or `None` for untyped items (implicitly type `object`).
    pub type_name: Option<Name>,
}

/// A sequence of typed groups; the standard PDDL typed-list construct (e.g. `?x ?y - type1 ?z - type2`).
pub type TypedList<T> = Vec<TypedGroup<T>>;

// ---------------------------------------------------------------------------
// Type declarations (with hierarchy)
// ---------------------------------------------------------------------------

/// Represents type declarations like `driver truck obj - locatable`.
pub type TypeDeclarations = TypedList<Name>;

// ---------------------------------------------------------------------------
// Predicate / Function declarations
// ---------------------------------------------------------------------------

/// A predicate declaration from the `:predicates` section, with name and typed parameters.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PredicateDecl {
    pub name: Name,
    pub parameters: TypedList<Variable>,
}

/// A numeric function declaration from `:functions`, with optional return type.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FunctionDecl {
    pub name: Name,
    pub parameters: TypedList<Variable>,
    pub return_type: Option<Name>,
}

// ---------------------------------------------------------------------------
// Numeric expressions
// ---------------------------------------------------------------------------

/// A numeric expression tree used in preconditions, effects, durations, and metrics.
///
/// N-ary PDDL operators like `(+ a b c)` are desugared into nested binary operations
/// during parsing, e.g. `(+ (+ a b) c)`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum NumericExpr {
    /// A numeric literal (e.g. `3.14`, `0`, `100`).
    Number(f64),
    /// A reference to a numeric fluent (e.g. `(distance ?x ?y)`).
    FunctionCall(FunctionTerm),
    /// A binary arithmetic operation.
    BinaryOp {
        op: BinaryOp,
        left: Box<NumericExpr>,
        right: Box<NumericExpr>,
    },
    /// Unary negation (e.g. `(- expr)`).
    Negate(Box<NumericExpr>),
    /// The built-in `total-time` expression (plan makespan).
    TotalTime,
    /// The `?duration` variable inside durative action constraints.
    Duration,
}

/// Arithmetic binary operators for [`NumericExpr::BinaryOp`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum BinaryOp {
    /// Addition (`+`).
    Add,
    /// Subtraction (`-`).
    Sub,
    /// Multiplication (`*`).
    Mul,
    /// Division (`/`).
    Div,
}

/// A reference to a numeric function (fluent) with its arguments.
///
/// For example, `(distance ?from ?to)` becomes:
///
/// ```text
/// FunctionTerm { name: "distance", args: [Variable("?from"), Variable("?to")] }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FunctionTerm {
    /// The function (fluent) name.
    pub name: Name,
    /// The arguments to the function, each a constant name or variable reference.
    pub args: Vec<Term>,
}

/// A term appearing as an argument to a predicate or function.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Term {
    /// A constant (object) name, e.g. `city1`, `truck-a`.
    Name(Name),
    /// A variable reference, e.g. `?x`. Includes the leading `?`.
    Variable(Variable),
}

// ---------------------------------------------------------------------------
// Comparisons
// ---------------------------------------------------------------------------

/// Numeric comparison operators used in [`Condition::NumericComparison`] and
/// [`DurationConstraint::Cmp`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum CompareOp {
    /// `<` -- strictly less than.
    Lt,
    /// `<=` -- less than or equal.
    Lte,
    /// `>` -- strictly greater than.
    Gt,
    /// `>=` -- greater than or equal.
    Gte,
    /// `=` -- equal.
    Eq,
}

// ---------------------------------------------------------------------------
// Conditions / Goal descriptions
// ---------------------------------------------------------------------------

/// A condition (goal description) in PDDL.
///
/// This is the central recursive type for preconditions, goals, and constraint bodies.
/// It covers logical connectives, first-order quantifiers, predicate tests, numeric
/// comparisons, temporal wrappers (durative actions), PDDL3 trajectory constraints,
/// and preferences.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Condition {
    /// Conjunction: `(and c1 c2 ...)`.
    And(Vec<Condition>),
    /// Disjunction: `(or c1 c2 ...)`.
    Or(Vec<Condition>),
    /// Negation: `(not c)`.
    Not(Box<Condition>),
    /// Implication: `(imply c1 c2)`.
    Imply(Box<Condition>, Box<Condition>),
    /// Universal quantification: `(forall (?x - type) c)`.
    Forall {
        variables: TypedList<Variable>,
        condition: Box<Condition>,
    },
    /// Existential quantification: `(exists (?x - type) c)`.
    Exists {
        variables: TypedList<Variable>,
        condition: Box<Condition>,
    },
    /// A positive predicate test: `(at ?x ?y)`.
    Predicate(AtomicFormula),
    /// Object equality: `(= ?x ?y)`.
    Equals(Term, Term),
    /// Numeric comparison: `(<op> <expr> <expr>)`.
    NumericComparison {
        op: CompareOp,
        left: NumericExpr,
        right: NumericExpr,
    },
    /// PDDL3 named preference: `(preference <name> <cond>)`.
    Preference {
        name: Option<Name>,
        condition: Box<Condition>,
    },

    // -- Temporal conditions (only inside durative actions) --
    /// `(at start <cond>)` -- holds at the action's start.
    AtStart(Box<Condition>),
    /// `(at end <cond>)` -- holds at the action's end.
    AtEnd(Box<Condition>),
    /// `(over all <cond>)` -- holds throughout the action.
    OverAll(Box<Condition>),

    // -- PDDL3 trajectory constraints --
    /// `(always <cond>)`.
    Always(Box<Condition>),
    /// `(sometime <cond>)`.
    Sometime(Box<Condition>),
    /// `(at-most-once <cond>)`.
    AtMostOnce(Box<Condition>),
    /// `(within <deadline> <cond>)`.
    Within(f64, Box<Condition>),
    /// `(sometime-before <cond1> <cond2>)`.
    SometimeBefore(Box<Condition>, Box<Condition>),
    /// `(sometime-after <cond1> <cond2>)`.
    SometimeAfter(Box<Condition>, Box<Condition>),
    /// `(always-within <window> <cond1> <cond2>)`.
    AlwaysWithin(f64, Box<Condition>, Box<Condition>),
    /// `(hold-during <t1> <t2> <cond>)`.
    HoldDuring(f64, f64, Box<Condition>),
    /// `(hold-after <t> <cond>)`.
    HoldAfter(f64, Box<Condition>),
}

/// A predicate applied to arguments, e.g. `(at truck1 city-a)`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AtomicFormula {
    /// The predicate name.
    pub name: Name,
    /// Arguments (constants or variables).
    pub args: Vec<Term>,
}

// ---------------------------------------------------------------------------
// Effects
// ---------------------------------------------------------------------------

/// An action effect.
///
/// Effects can add/delete predicate instances, perform numeric assignments,
/// and include conditional (`when`) and universal (`forall`) sub-effects.
/// Inside durative actions, effects are wrapped in temporal markers (`AtStart`, `AtEnd`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Effect {
    /// Conjunction: `(and e1 e2 ...)`.
    And(Vec<Effect>),
    /// Add (assert) a predicate instance.
    Predicate(AtomicFormula),
    /// Delete (negate) a predicate instance: `(not (pred ...))`.
    NotPredicate(AtomicFormula),
    /// Universal effect: `(forall (?x - type) <effect>)`.
    Forall {
        variables: TypedList<Variable>,
        effect: Box<Effect>,
    },
    /// Conditional effect: `(when <condition> <effect>)`.
    When {
        condition: Condition,
        effect: Box<Effect>,
    },
    /// Numeric assignment: `(assign/increase/decrease/scale-up/scale-down <fn> <expr>)`.
    NumericAssign {
        op: AssignOp,
        function: FunctionTerm,
        expr: NumericExpr,
    },

    // -- Temporal effects (only inside durative actions) --
    /// `(at start <effect>)`.
    AtStart(Box<Effect>),
    /// `(at end <effect>)`.
    AtEnd(Box<Effect>),
}

/// Numeric assignment operators for [`Effect::NumericAssign`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum AssignOp {
    /// `(assign <fn> <expr>)` -- set to value.
    Assign,
    /// `(scale-up <fn> <expr>)` -- multiply.
    ScaleUp,
    /// `(scale-down <fn> <expr>)` -- divide.
    ScaleDown,
    /// `(increase <fn> <expr>)` -- add.
    Increase,
    /// `(decrease <fn> <expr>)` -- subtract.
    Decrease,
}

// ---------------------------------------------------------------------------
// Duration constraints
// ---------------------------------------------------------------------------

/// Duration constraint on a durative action (e.g. `(= ?duration 5)`, `(>= ?duration (distance ?x ?y))`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum DurationConstraint {
    /// Conjunction of multiple duration constraints.
    And(Vec<DurationConstraint>),
    /// A single comparison against `?duration`.
    Cmp {
        /// The comparison operator (`=`, `>=`, `<=`).
        op: CompareOp,
        /// The right-hand-side expression (left-hand side is always `?duration`).
        expr: NumericExpr,
    },
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

/// A non-temporal (instantaneous) PDDL action defined with `:action`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BasicAction {
    /// The action name (e.g. `"drive"`).
    pub name: Name,
    /// Typed parameter list. Includes any PDDL 1.2 `:vars` parameters.
    pub parameters: TypedList<Variable>,
    /// Optional precondition; `None` means unconditionally applicable.
    pub precondition: Option<Condition>,
    /// Optional effect; `None` means no state change.
    pub effect: Option<Effect>,
}

/// A temporal PDDL action defined with `:durative-action`.
///
/// Conditions and effects typically contain temporal wrappers (`at start`, `at end`,
/// `over all`) which the grounder flattens into separate vectors.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DurativeAction {
    /// The action name (e.g. `"drive"`).
    pub name: Name,
    /// Typed parameter list.
    pub parameters: TypedList<Variable>,
    /// Duration constraint (e.g. `(= ?duration 10)`).
    pub duration: DurationConstraint,
    /// Optional condition with temporal annotations.
    pub condition: Option<Condition>,
    /// Optional effect with temporal annotations.
    pub effect: Option<Effect>,
}

// ---------------------------------------------------------------------------
// Derived predicates
// ---------------------------------------------------------------------------

/// A derived predicate (`:derived`) with axiom body.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DerivedPredicate {
    pub predicate: AtomicFormula,
    pub condition: Condition,
}

// ---------------------------------------------------------------------------
// Init elements (problem file)
// ---------------------------------------------------------------------------

/// An element of the problem's `:init` section.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum InitElement {
    /// A predicate that holds in the initial state (e.g. `(at truck1 s0)`).
    Predicate(AtomicFormula),
    /// A negated predicate in the initial state (e.g. `(not (visited c1))`).
    NotPredicate(AtomicFormula),
    /// A numeric fluent initialization (e.g. `(= (distance a b) 10)`).
    NumericAssignment(FunctionTerm, f64),
    /// A timed initial literal: `(at <time> <literal>)`. The inner element is
    /// typically a `Predicate` or `NotPredicate`.
    At(f64, Box<InitElement>),
}

// ---------------------------------------------------------------------------
// Metric specification
// ---------------------------------------------------------------------------

/// Optimization direction for the `:metric` specification.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Optimization {
    /// `minimize` the metric expression.
    Minimize,
    /// `maximize` the metric expression.
    Maximize,
}

/// The `:metric` specification combining optimization direction and expression.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MetricSpec {
    pub optimization: Optimization,
    pub expr: NumericExpr,
}

// ---------------------------------------------------------------------------
// Top-level structures
// ---------------------------------------------------------------------------

/// A parsed PDDL domain file.
///
/// Produced by [`super::parser::parse_domain`] / [`super::parser::parse_domain_str`].
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Domain {
    /// The domain name from `(domain <name>)`.
    pub name: Name,
    /// Declared requirements (`:requirements`).
    pub requirements: Vec<Requirement>,
    /// Type hierarchy declarations (`:types`).
    pub types: TypeDeclarations,
    /// Domain-level constants (`:constants`).
    pub constants: TypedList<Name>,
    /// Predicate declarations (`:predicates`).
    pub predicates: Vec<PredicateDecl>,
    /// Numeric function declarations (`:functions`).
    pub functions: Vec<FunctionDecl>,
    /// Instantaneous actions (`:action`).
    pub actions: Vec<BasicAction>,
    /// Temporal actions (`:durative-action`).
    pub durative_actions: Vec<DurativeAction>,
    /// Derived predicates / axioms (`:derived`).
    pub derived_predicates: Vec<DerivedPredicate>,
}

impl Domain {
    /// Sort all declaration lists alphabetically by name.
    ///
    /// Sorts predicates, functions, actions, durative actions, derived predicates,
    /// and items within each typed group (constants, types) for deterministic ordering.
    pub fn sort_alphabetically(&mut self) {
        for group in &mut self.types {
            group.items.sort();
        }
        for group in &mut self.constants {
            group.items.sort();
        }
        self.predicates.sort_by(|a, b| a.name.cmp(&b.name));
        self.functions.sort_by(|a, b| a.name.cmp(&b.name));
        self.actions.sort_by(|a, b| a.name.cmp(&b.name));
        self.durative_actions.sort_by(|a, b| a.name.cmp(&b.name));
        self.derived_predicates
            .sort_by(|a, b| a.predicate.name.cmp(&b.predicate.name));
    }
}

/// A parsed PDDL problem file.
///
/// Produced by [`super::parser::parse_problem`] / [`super::parser::parse_problem_str`].
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Problem {
    /// The problem name from `(problem <name>)`.
    pub name: Name,
    /// The domain this problem refers to, from `(:domain <name>)`.
    pub domain_name: Name,
    /// Declared requirements (`:requirements`), if any.
    pub requirements: Vec<Requirement>,
    /// Typed object list (`:objects`).
    pub objects: TypedList<Name>,
    /// Initial state elements (`:init`).
    pub init: Vec<InitElement>,
    /// Goal condition (`:goal`).
    pub goal: Condition,
    /// Optional optimization metric (`:metric`).
    pub metric: Option<MetricSpec>,
    /// Optional PDDL3 trajectory constraints (`:constraints`).
    pub constraints: Option<Condition>,
}

impl Problem {
    /// Sort object lists and init elements alphabetically for deterministic ordering.
    pub fn sort_alphabetically(&mut self) {
        for group in &mut self.objects {
            group.items.sort();
        }
        self.init.sort_by_key(init_sort_key);
    }
}

/// Format an atomic formula for sorting purposes.
fn format_atomic_formula(af: &AtomicFormula) -> String {
    if af.args.is_empty() {
        af.name.clone()
    } else {
        let args: Vec<&str> = af.args.iter().map(term_name).collect();
        format!("{}({})", af.name, args.join(","))
    }
}

/// Format a function term for sorting purposes.
fn format_function_term(ft: &FunctionTerm) -> String {
    if ft.args.is_empty() {
        ft.name.clone()
    } else {
        let args: Vec<&str> = ft.args.iter().map(term_name).collect();
        format!("{}({})", ft.name, args.join(","))
    }
}

/// Compute a sort key for an init element.
fn init_sort_key(e: &InitElement) -> (u8, String) {
    match e {
        InitElement::Predicate(af) => (0, format_atomic_formula(af)),
        InitElement::NotPredicate(af) => (1, format_atomic_formula(af)),
        InitElement::NumericAssignment(ft, _) => (2, format_function_term(ft)),
        InitElement::At(t, inner) => {
            let (_, s) = init_sort_key(inner);
            (3, format!("{t:.6}{s}"))
        }
    }
}

/// Extract the name from a Term for sorting.
fn term_name(t: &Term) -> &str {
    match t {
        Term::Name(n) => n.as_str(),
        Term::Variable(v) => v.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_sort_orders_all_named_declarations() {
        let mut domain = Domain {
            name: "d".to_string(),
            requirements: Vec::new(),
            types: vec![TypedGroup {
                items: vec!["z".to_string(), "a".to_string()],
                type_name: None,
            }],
            constants: vec![TypedGroup {
                items: vec!["c2".to_string(), "c1".to_string()],
                type_name: None,
            }],
            predicates: vec![
                PredicateDecl {
                    name: "zpred".to_string(),
                    parameters: Vec::new(),
                },
                PredicateDecl {
                    name: "apred".to_string(),
                    parameters: Vec::new(),
                },
            ],
            functions: vec![
                FunctionDecl {
                    name: "zfunc".to_string(),
                    parameters: Vec::new(),
                    return_type: None,
                },
                FunctionDecl {
                    name: "afunc".to_string(),
                    parameters: Vec::new(),
                    return_type: None,
                },
            ],
            actions: vec![
                BasicAction {
                    name: "zact".to_string(),
                    parameters: Vec::new(),
                    precondition: None,
                    effect: None,
                },
                BasicAction {
                    name: "aact".to_string(),
                    parameters: Vec::new(),
                    precondition: None,
                    effect: None,
                },
            ],
            durative_actions: vec![
                DurativeAction {
                    name: "zdur".to_string(),
                    parameters: Vec::new(),
                    duration: DurationConstraint::Cmp {
                        op: CompareOp::Eq,
                        expr: NumericExpr::Number(1.0),
                    },
                    condition: None,
                    effect: None,
                },
                DurativeAction {
                    name: "adur".to_string(),
                    parameters: Vec::new(),
                    duration: DurationConstraint::Cmp {
                        op: CompareOp::Eq,
                        expr: NumericExpr::Number(1.0),
                    },
                    condition: None,
                    effect: None,
                },
            ],
            derived_predicates: vec![
                DerivedPredicate {
                    predicate: AtomicFormula {
                        name: "zderived".to_string(),
                        args: Vec::new(),
                    },
                    condition: Condition::And(Vec::new()),
                },
                DerivedPredicate {
                    predicate: AtomicFormula {
                        name: "aderived".to_string(),
                        args: Vec::new(),
                    },
                    condition: Condition::And(Vec::new()),
                },
            ],
        };

        domain.sort_alphabetically();

        assert_eq!(domain.types[0].items, ["a", "z"]);
        assert_eq!(domain.constants[0].items, ["c1", "c2"]);
        assert_eq!(domain.predicates[0].name, "apred");
        assert_eq!(domain.functions[0].name, "afunc");
        assert_eq!(domain.actions[0].name, "aact");
        assert_eq!(domain.durative_actions[0].name, "adur");
        assert_eq!(domain.derived_predicates[0].predicate.name, "aderived");
    }

    #[test]
    fn problem_sort_handles_zero_arity_predicates_and_variable_terms() {
        let mut problem = Problem {
            name: "p".to_string(),
            domain_name: "d".to_string(),
            requirements: Vec::new(),
            objects: Vec::new(),
            init: vec![
                InitElement::NumericAssignment(
                    FunctionTerm {
                        name: "cost".to_string(),
                        args: vec![Term::Variable("?x".to_string())],
                    },
                    1.0,
                ),
                InitElement::Predicate(AtomicFormula {
                    name: "ready".to_string(),
                    args: Vec::new(),
                }),
            ],
            goal: Condition::And(Vec::new()),
            metric: None,
            constraints: None,
        };

        problem.sort_alphabetically();
        assert!(matches!(problem.init[0], InitElement::Predicate(_)));
        assert!(matches!(
            problem.init[1],
            InitElement::NumericAssignment(_, _)
        ));
    }
}
