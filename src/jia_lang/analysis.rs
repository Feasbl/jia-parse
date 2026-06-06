//! Symbol table for the CP scheduling modelling language.
//!
//! Builds a [`SymbolTable`] from a parsed [`JiaModel`] and its token stream,
//! mapping declared variable names to their types, declaration spans, usage
//! spans, and domain summaries. Used by the LSP server for hover, go-to-definition,
//! and completion.

use std::collections::HashMap;

use crate::error::Span;
use crate::jia_lang::ast::{Domain, DomainStmt, Expr, JiaModel, VarType};
use crate::jia_lang::lexer::{Token, TokenKind};

/// Information about a single declared variable.
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    /// The variable's declared type.
    pub var_type: VarType,
    /// Span of the name token in the variables block (declaration site).
    pub decl_span: Span,
    /// All spans where this name appears as a reference (domains, constraints, exprs).
    pub ref_spans: Vec<Span>,
    /// Human-readable domain summary for hover display (e.g. "duration = 3, start in 0..10").
    pub domain_summary: Option<String>,
}

/// Maps variable names to their declaration and usage information.
#[derive(Debug, Clone)]
pub struct SymbolTable {
    /// Variable name → symbol info.
    pub symbols: HashMap<String, SymbolInfo>,
}

/// Compute the text length (in bytes) of a token, for span-range calculations.
pub fn token_text_len(token: &Token) -> usize {
    match &token.kind {
        TokenKind::Ident(s) => s.len(),
        TokenKind::Number(n) => {
            if *n == 0 {
                1
            } else {
                let abs = n.unsigned_abs();
                let digits = (abs as f64).log10().floor() as usize + 1;
                if *n < 0 {
                    digits + 1
                } else {
                    digits
                }
            }
        }
        TokenKind::LParen
        | TokenKind::RParen
        | TokenKind::LBrace
        | TokenKind::RBrace
        | TokenKind::LBracket
        | TokenKind::RBracket
        | TokenKind::Comma
        | TokenKind::Colon
        | TokenKind::Plus
        | TokenKind::Minus
        | TokenKind::Star
        | TokenKind::Lt
        | TokenKind::Gt
        | TokenKind::Eq
        | TokenKind::Slash
        | TokenKind::At => 1,
        TokenKind::DotDot | TokenKind::Le | TokenKind::Ge | TokenKind::EqEq | TokenKind::Ne => 2,
        TokenKind::Float(f) => format!("{f}").len(),
    }
}

/// Build a [`SymbolTable`] from a successfully parsed model and its token stream.
///
/// This performs a post-parse scan of the tokens to find declaration and reference
/// spans, then extracts domain summaries from the AST. No changes to the parser
/// or AST types are required.
pub fn build_symbol_table(model: &JiaModel, tokens: &[Token]) -> SymbolTable {
    // Step 1: Collect declared variable names and types from the AST.
    let mut declared: HashMap<String, VarType> = HashMap::new();
    for decl in &model.variables {
        for name in &decl.names {
            declared.insert(name.clone(), decl.var_type.clone());
        }
    }

    // Step 2: Scan the token stream to find declaration vs. reference spans.
    // Declaration spans are ident tokens inside the variables block;
    // reference spans are ident tokens matching a declared name everywhere else.
    let mut symbols: HashMap<String, SymbolInfo> = HashMap::new();
    let mut in_variables_block = false;
    let mut brace_depth: usize = 0;
    let mut variables_brace_depth: Option<usize> = None;

    for token in tokens {
        match &token.kind {
            TokenKind::Ident(name) if name == "variables" => {
                in_variables_block = true;
            }
            TokenKind::LBrace => {
                brace_depth += 1;
                if in_variables_block && variables_brace_depth.is_none() {
                    variables_brace_depth = Some(brace_depth);
                }
            }
            TokenKind::RBrace => {
                if let Some(vbd) = variables_brace_depth {
                    if brace_depth == vbd {
                        in_variables_block = false;
                        variables_brace_depth = None;
                    }
                }
                brace_depth = brace_depth.saturating_sub(1);
            }
            TokenKind::Ident(name) if declared.contains_key(name) => {
                let entry = symbols.entry(name.clone()).or_insert_with(|| SymbolInfo {
                    var_type: declared[name].clone(),
                    decl_span: token.span,
                    ref_spans: Vec::new(),
                    domain_summary: None,
                });
                if in_variables_block {
                    // First occurrence in variables block is the declaration.
                    entry.decl_span = token.span;
                } else {
                    entry.ref_spans.push(token.span);
                }
            }
            _ => {}
        }
    }

    // Step 3: Build domain summaries from the AST.
    for stmt in &model.domains {
        match stmt {
            DomainStmt::IntervalDuration { intervals, domain } => {
                let desc = format!("duration {}", format_domain(domain));
                for name in intervals {
                    append_domain_summary(&mut symbols, name, &desc);
                }
            }
            DomainStmt::IntervalStart { intervals, domain } => {
                let desc = format!("start {}", format_domain(domain));
                for name in intervals {
                    append_domain_summary(&mut symbols, name, &desc);
                }
            }
            DomainStmt::IntervalEnd { intervals, domain } => {
                let desc = format!("end {}", format_domain(domain));
                for name in intervals {
                    append_domain_summary(&mut symbols, name, &desc);
                }
            }
            DomainStmt::IntervalOptional { intervals } => {
                for name in intervals {
                    append_domain_summary(&mut symbols, name, "optional");
                }
            }
            DomainStmt::IntegerDomain { name, domain } => {
                let desc = format_domain(domain);
                append_domain_summary(&mut symbols, name, &desc);
            }
            DomainStmt::SetDomain { name, members } => {
                let desc = format!("members: {{{}}}", members.join(", "));
                append_domain_summary(&mut symbols, name, &desc);
            }
            DomainStmt::Demand {
                interval,
                set,
                value,
            } => {
                let desc = format!("demand({set}) = {value}");
                append_domain_summary(&mut symbols, interval, &desc);
            }
            DomainStmt::RealDomain { name, domain } => {
                let desc = format!("real {}", format_domain(domain));
                append_domain_summary(&mut symbols, name, &desc);
            }
        }
    }

    SymbolTable { symbols }
}

/// Format a [`Domain`] as a human-readable string.
fn format_domain(domain: &Domain) -> String {
    match domain {
        Domain::Fixed(v) => format!("= {v}"),
        Domain::Range { min, max } => format!("in {min}..{max}"),
        Domain::Enumerated(vals) => {
            let vals_str: Vec<String> = vals.iter().map(|v| v.to_string()).collect();
            format!("in {{{}}}", vals_str.join(", "))
        }
        Domain::RealFixed(v) => format!("= {v}"),
        Domain::RealRange { min, max } => format!("in {min}..{max}"),
    }
}

/// Append a domain description line to a symbol's domain summary.
fn append_domain_summary(symbols: &mut HashMap<String, SymbolInfo>, name: &str, desc: &str) {
    if let Some(info) = symbols.get_mut(name) {
        match &mut info.domain_summary {
            Some(existing) => {
                existing.push_str(", ");
                existing.push_str(desc);
            }
            None => {
                info.domain_summary = Some(desc.to_string());
            }
        }
    }
}

/// Find the token at a given 0-based line and character position.
///
/// Returns the token whose span covers the given position, or `None` if no
/// token is at that position.
pub fn token_at_position(tokens: &[Token], line: u32, character: u32) -> Option<&Token> {
    // Convert from 0-based LSP position to 1-based Span position.
    let target_line = line as usize + 1;
    let target_col = character as usize + 1;

    for token in tokens {
        if token.span.line == target_line {
            let start_col = token.span.col;
            let end_col = start_col + token_text_len(token);
            if target_col >= start_col && target_col < end_col {
                return Some(token);
            }
        }
    }
    None
}

/// Collect all variable names referenced inside an expression.
fn _collect_expr_names(expr: &Expr, names: &mut Vec<String>) {
    match expr {
        Expr::Var(name)
        | Expr::StartOf(name)
        | Expr::EndOf(name)
        | Expr::DurationOf(name)
        | Expr::PresentOf(name) => {
            names.push(name.clone());
        }
        Expr::BinaryOp { left, right, .. } => {
            _collect_expr_names(left, names);
            _collect_expr_names(right, names);
        }
        Expr::Negate(inner) => {
            _collect_expr_names(inner, names);
        }
        Expr::Number(_) | Expr::Float(_) => {}
    }
}

impl std::fmt::Display for VarType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VarType::Interval => write!(f, "Interval"),
            VarType::Integer => write!(f, "Integer"),
            VarType::SetInterval => write!(f, "Set[Interval]"),
            VarType::SetInteger => write!(f, "Set[Integer]"),
            VarType::Real => write!(f, "Real"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Span;
    use crate::jia_lang::ast::{ArithOp, Expr, VarDecl};
    use crate::jia_lang::lexer::tokenize;
    use crate::jia_lang::parser::parse_model;

    fn build_table(input: &str) -> (JiaModel, SymbolTable) {
        let tokens = tokenize(input).unwrap();
        let model = parse_model(&tokens).unwrap();
        let table = build_symbol_table(&model, &tokens);
        (model, table)
    }

    #[test]
    fn test_basic_symbol_table() {
        let input = r#"
model test
variables {
  Interval: a, b
  Integer: x
}
domains {
  duration(a) = 3
  x in 0..10
}
constraints {
  end_of(a) <= start_of(b)
  end_of(b) <= x
}
"#;
        let (_model, table) = build_table(input);

        // Check all variables are in the table.
        assert!(table.symbols.contains_key("a"));
        assert!(table.symbols.contains_key("b"));
        assert!(table.symbols.contains_key("x"));

        // Check types.
        assert_eq!(table.symbols["a"].var_type, VarType::Interval);
        assert_eq!(table.symbols["b"].var_type, VarType::Interval);
        assert_eq!(table.symbols["x"].var_type, VarType::Integer);

        // Check reference counts:
        // 'a' is referenced in: duration(a), end_of(a) => 2 refs
        assert_eq!(table.symbols["a"].ref_spans.len(), 2);
        // 'b' is referenced in: start_of(b), end_of(b) => 2 refs
        assert_eq!(table.symbols["b"].ref_spans.len(), 2);
        // 'x' is referenced in: x in 0..10, end_of(b) <= x => 2 refs
        assert_eq!(table.symbols["x"].ref_spans.len(), 2);

        // Check domain summaries.
        assert_eq!(
            table.symbols["a"].domain_summary.as_deref(),
            Some("duration = 3")
        );
        assert_eq!(
            table.symbols["x"].domain_summary.as_deref(),
            Some("in 0..10")
        );
        assert!(table.symbols["b"].domain_summary.is_none());
    }

    #[test]
    fn test_decl_span_points_to_variables_block() {
        let input = "model t\nvariables { Interval: task }\nconstraints { end_of(task) <= 10 }";
        let (_model, table) = build_table(input);

        let info = &table.symbols["task"];
        // 'task' declared on line 2 (the variables line).
        assert_eq!(info.decl_span.line, 2);
        // Referenced once in constraints.
        assert_eq!(info.ref_spans.len(), 1);
        assert_eq!(info.ref_spans[0].line, 3);
    }

    #[test]
    fn test_token_at_position() {
        let input = "model test";
        let tokens = tokenize(input).unwrap();
        // 'model' is at line 0 (0-based), col 0 (0-based).
        let tok = token_at_position(&tokens, 0, 0).unwrap();
        assert_eq!(tok.kind, TokenKind::Ident("model".to_string()));

        // 'test' is at line 0, col 6.
        let tok = token_at_position(&tokens, 0, 6).unwrap();
        assert_eq!(tok.kind, TokenKind::Ident("test".to_string()));

        // Nothing at col 5 (space).
        assert!(token_at_position(&tokens, 0, 5).is_none());
        assert!(token_at_position(&tokens, 1, 0).is_none());
    }

    #[test]
    fn test_token_text_len() {
        let tokens = tokenize("model 42 <= ..").unwrap();
        assert_eq!(token_text_len(&tokens[0]), 5); // "model"
        assert_eq!(token_text_len(&tokens[1]), 2); // "42"
        assert_eq!(token_text_len(&tokens[2]), 2); // "<="
        assert_eq!(token_text_len(&tokens[3]), 2); // ".."

        assert_eq!(
            token_text_len(&Token {
                kind: TokenKind::Number(0),
                span: Span::new(0, 1, 1)
            }),
            1
        );
        assert_eq!(
            token_text_len(&Token {
                kind: TokenKind::Number(-123),
                span: Span::new(0, 1, 1)
            }),
            4
        );
        assert_eq!(
            token_text_len(&Token {
                kind: TokenKind::Float(12.5),
                span: Span::new(0, 1, 1)
            }),
            4
        );
    }

    #[test]
    fn test_domain_summary_multiple_entries() {
        let input = r#"
model test
variables { Interval: a }
domains {
  duration(a) = 5
  start(a) in 0..10
  optional(a)
}
"#;
        let (_model, table) = build_table(input);
        let summary = table.symbols["a"].domain_summary.as_deref().unwrap();
        assert!(summary.contains("duration = 5"));
        assert!(summary.contains("start in 0..10"));
        assert!(summary.contains("optional"));
    }

    #[test]
    fn test_set_domain_summary() {
        let input = r#"
model test
variables {
  Interval: t1, t2
  Set[Interval]: machine
}
domains {
  machine = {t1, t2}
}
"#;
        let (_model, table) = build_table(input);
        let summary = table.symbols["machine"].domain_summary.as_deref().unwrap();
        assert_eq!(summary, "members: {t1, t2}");
    }

    #[test]
    fn test_demand_summary() {
        let input = r#"
model test
variables {
  Interval: a
  Set[Interval]: res
}
domains {
  res = {a}
  demand(a, res) = 3
}
"#;
        let (_model, table) = build_table(input);
        let summary = table.symbols["a"].domain_summary.as_deref().unwrap();
        assert!(summary.contains("demand(res) = 3"));
    }

    #[test]
    fn test_real_domain_summary_and_missing_append_target() {
        let mut symbols = HashMap::new();
        append_domain_summary(&mut symbols, "missing", "ignored");

        let input = r#"
model test
variables { Real: rate, slack }
domains {
  rate in 1.5..3.5
  slack in -inf..inf
}
"#;
        let (_model, table) = build_table(input);
        assert_eq!(
            table.symbols["rate"].domain_summary.as_deref(),
            Some("in 1.5..3.5")
        );
        assert_eq!(
            table.symbols["slack"].domain_summary.as_deref(),
            Some("in -inf..inf")
        );

        let tokens = tokenize("model manual\nvariables { Real: exact }").unwrap();
        let model = JiaModel {
            model_type: None,
            name: "manual".to_string(),
            variables: vec![VarDecl {
                names: vec!["exact".to_string()],
                var_type: VarType::Real,
            }],
            domains: vec![DomainStmt::RealDomain {
                name: "exact".to_string(),
                domain: Domain::RealFixed(2.25),
            }],
            constraints: Vec::new(),
            objective: None,
        };
        let table = build_symbol_table(&model, &tokens);
        assert_eq!(
            table.symbols["exact"].domain_summary.as_deref(),
            Some("real = 2.25")
        );
    }

    #[test]
    fn test_collect_expr_names() {
        let expr = Expr::BinaryOp {
            op: ArithOp::Add,
            left: Box::new(Expr::BinaryOp {
                op: ArithOp::Sub,
                left: Box::new(Expr::StartOf("a".to_string())),
                right: Box::new(Expr::Negate(Box::new(Expr::EndOf("b".to_string())))),
            }),
            right: Box::new(Expr::BinaryOp {
                op: ArithOp::Mul,
                left: Box::new(Expr::DurationOf("c".to_string())),
                right: Box::new(Expr::BinaryOp {
                    op: ArithOp::Div,
                    left: Box::new(Expr::PresentOf("d".to_string())),
                    right: Box::new(Expr::Var("e".to_string())),
                }),
            }),
        };
        let mut names = Vec::new();
        _collect_expr_names(&expr, &mut names);
        assert_eq!(names, ["a", "b", "c", "d", "e"]);

        _collect_expr_names(&Expr::Number(1), &mut names);
        _collect_expr_names(&Expr::Float(1.5), &mut names);
        assert_eq!(names, ["a", "b", "c", "d", "e"]);
    }
}
