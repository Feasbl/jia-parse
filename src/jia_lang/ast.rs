//! AST types for the CP scheduling modelling language.
//!
//! The language follows an **X, D, C** pattern:
//! - **Variables (X)**: Declare names and types (Interval, Integer, Set\[Interval\], Set\[Integer\])
//! - **Domains (D)**: Bound variable attributes and assign set membership
//! - **Constraints (C)**: Relationships between variables
//! - **Objective** (optional): Minimize/maximize an expression

/// The declared model type (from `@model` tag).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum ModelType {
    /// Constraint programming (`@model cp`).
    Cp,
    /// Linear programming (`@model lp`).
    Lp,
}

/// A complete Jia model.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct JiaModel {
    /// Model type (from `@model` tag), or None if not specified.
    pub model_type: Option<ModelType>,
    /// Model name (from the `model <name>` declaration).
    pub name: String,
    /// Variable declarations.
    pub variables: Vec<VarDecl>,
    /// Domain specifications.
    pub domains: Vec<DomainStmt>,
    /// Constraints.
    pub constraints: Vec<Constraint>,
    /// Optional optimization objective.
    pub objective: Option<Objective>,
}

/// A variable declaration: `Type: name1, name2, ...`
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct VarDecl {
    /// The declared variable names.
    pub names: Vec<String>,
    /// The type of all variables in this declaration.
    pub var_type: VarType,
}

/// Variable types in the Jia language.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum VarType {
    /// An interval variable with start, end, and duration.
    Interval,
    /// An integer decision variable.
    Integer,
    /// A continuous real-valued decision variable (LP).
    Real,
    /// A set of interval variables.
    SetInterval,
    /// A set of integer variables.
    SetInteger,
}

/// A domain statement: bounds or assigns values to variables.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum DomainStmt {
    /// `duration(x, y) = 3` or `duration(x) in 2..5` or `duration(x) in {2, 5, 7}`
    IntervalDuration {
        /// Interval names this applies to.
        intervals: Vec<String>,
        /// The domain specification.
        domain: Domain,
    },
    /// `start(x, y) in 0..10`
    IntervalStart {
        /// Interval names this applies to.
        intervals: Vec<String>,
        /// The domain specification.
        domain: Domain,
    },
    /// `end(x, y) in 0..20`
    IntervalEnd {
        /// Interval names this applies to.
        intervals: Vec<String>,
        /// The domain specification.
        domain: Domain,
    },
    /// `optional(x, y, z)`
    IntervalOptional {
        /// Interval names marked as optional.
        intervals: Vec<String>,
    },
    /// `x in 0..100` or `x in {1, 3, 5}`
    IntegerDomain {
        /// The integer variable name.
        name: String,
        /// The domain specification.
        domain: Domain,
    },
    /// `x = {a, b, c}`
    SetDomain {
        /// The set variable name.
        name: String,
        /// The member variable names.
        members: Vec<String>,
    },
    /// `x in 0.0..inf` or `x = 3.14` — real variable domain
    RealDomain {
        /// The real variable name.
        name: String,
        /// The domain specification.
        domain: Domain,
    },
    /// `demand(interval, set) = value`
    Demand {
        /// The interval variable name.
        interval: String,
        /// The set (resource) variable name.
        set: String,
        /// The demand value.
        value: i64,
    },
}

/// A domain specification: fixed value, range, or enumerated set.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum Domain {
    /// A single fixed integer value: `= 3`
    Fixed(i64),
    /// An integer range: `in 0..100`
    Range {
        /// Minimum (inclusive).
        min: i64,
        /// Maximum (inclusive).
        max: i64,
    },
    /// An enumerated set of integer values: `in {1, 3, 5}`
    Enumerated(Vec<i64>),
    /// A single fixed real value: `= 3.14`
    RealFixed(f64),
    /// A real range: `in 0.0..inf` (f64::INFINITY for unbounded)
    RealRange {
        /// Minimum (inclusive). f64::NEG_INFINITY for unbounded below.
        min: f64,
        /// Maximum (inclusive). f64::INFINITY for unbounded above.
        max: f64,
    },
}

/// A constraint in the CP model.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum Constraint {
    /// `no_overlap(set_or_interval, ...)`
    NoOverlap {
        /// The intervals or set names (no two may overlap).
        intervals: Vec<String>,
    },
    /// `cumulative(set, capacity)`
    Cumulative {
        /// The set (resource) variable name.
        set: String,
        /// The capacity expression.
        capacity: Expr,
    },
    /// `span(parent, set)` — parent covers earliest start to latest end of present children.
    Span {
        /// The parent interval.
        parent: String,
        /// The set of child intervals.
        set: String,
    },
    /// `alternative(parent, set)` — exactly one interval from set is present.
    Alternative {
        /// The parent interval representing the chosen one.
        parent: String,
        /// The set of candidate intervals.
        set: String,
    },
    /// `<expr> <op> <expr>`
    Comparison {
        /// Left-hand side expression.
        left: Expr,
        /// Comparison operator.
        op: CmpOp,
        /// Right-hand side expression.
        right: Expr,
    },
}

/// Comparison operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum CmpOp {
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,
    /// `==`
    Eq,
    /// `!=`
    Ne,
}

/// An expression in the CP language.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum Expr {
    /// An integer literal.
    Number(i64),
    /// A floating-point literal.
    Float(f64),
    /// A variable reference.
    Var(String),
    /// `start_of(name)`
    StartOf(String),
    /// `end_of(name)`
    EndOf(String),
    /// `duration_of(name)`
    DurationOf(String),
    /// `present_of(name)` — 0 if absent, 1 if present.
    PresentOf(String),
    /// A binary arithmetic operation.
    BinaryOp {
        /// The operator.
        op: ArithOp,
        /// Left operand.
        left: Box<Expr>,
        /// Right operand.
        right: Box<Expr>,
    },
    /// Unary negation.
    Negate(Box<Expr>),
}

/// Arithmetic operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum ArithOp {
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
}

/// An optimization objective.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Objective {
    /// Minimize or maximize.
    pub direction: OptDirection,
    /// The expression to optimize.
    pub expr: Expr,
}

/// Optimization direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum OptDirection {
    /// Minimize the objective.
    Minimize,
    /// Maximize the objective.
    Maximize,
}

impl std::fmt::Display for CmpOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CmpOp::Lt => write!(f, "<"),
            CmpOp::Le => write!(f, "<="),
            CmpOp::Gt => write!(f, ">"),
            CmpOp::Ge => write!(f, ">="),
            CmpOp::Eq => write!(f, "=="),
            CmpOp::Ne => write!(f, "!="),
        }
    }
}

impl std::fmt::Display for ArithOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArithOp::Add => write!(f, "+"),
            ArithOp::Sub => write!(f, "-"),
            ArithOp::Mul => write!(f, "*"),
            ArithOp::Div => write!(f, "/"),
        }
    }
}
