//! Parser for the CP scheduling modelling language.
//!
//! Top-level grammar:
//! ```text
//! model       = "model" IDENT sections* objective?
//! sections    = variables_block | domains_block | constraints_block
//! ```
//!
//! Public entry point: [`parse_model`]

pub mod constraint;
pub mod decl;
pub mod domain;
pub mod expr;

use crate::error::{ParseError, Span};
use crate::jia_lang::ast::{Expr, JiaModel, ModelType, Objective, OptDirection};
use crate::jia_lang::lexer::{Token, TokenKind};

/// Parser state wrapping a token slice with a cursor.
pub struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> Parser<'a> {
    /// Create a new parser over the given token slice.
    pub fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, pos: 0 }
    }

    /// Peek at the current token's kind without consuming it.
    pub fn peek_kind(&self) -> Option<TokenKind> {
        self.tokens.get(self.pos).map(|t| t.kind.clone())
    }

    /// Advance the cursor by one token.
    pub fn advance(&mut self) {
        self.pos += 1;
    }

    /// Get the span of the current token (or a default EOF span).
    pub fn current_span(&self) -> Span {
        self.tokens
            .get(self.pos)
            .map(|t| t.span)
            .unwrap_or(Span::new(0, 1, 1))
    }

    /// Return true if there are no more tokens.
    pub fn is_eof(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    /// Expect and consume a specific token kind.
    pub fn expect_token(&mut self, expected: TokenKind) -> Result<(), ParseError> {
        let span = self.current_span();
        match self.peek_kind() {
            Some(ref kind) if kind == &expected => {
                self.advance();
                Ok(())
            }
            Some(ref kind) => Err(ParseError::new(
                format!("expected {expected:?}, got {kind:?}"),
                span,
            )),
            None => Err(ParseError::new(
                format!("expected {expected:?}, got end of input"),
                span,
            )),
        }
    }

    /// Expect and consume an identifier, returning its name.
    pub fn expect_ident(&mut self) -> Result<String, ParseError> {
        let span = self.current_span();
        match self.peek_kind() {
            Some(TokenKind::Ident(name)) => {
                self.advance();
                Ok(name)
            }
            Some(kind) => Err(ParseError::new(
                format!("expected identifier, got {kind:?}"),
                span,
            )),
            None => Err(ParseError::new(
                "expected identifier, got end of input",
                span,
            )),
        }
    }

    /// Expect and consume a specific identifier.
    pub fn expect_ident_matching(&mut self, expected: &str) -> Result<(), ParseError> {
        let span = self.current_span();
        let name = self.expect_ident()?;
        if name != expected {
            return Err(ParseError::new(
                format!("expected '{expected}', got '{name}'"),
                span,
            ));
        }
        Ok(())
    }

    /// Expect and consume a number literal.
    pub fn expect_number(&mut self) -> Result<i64, ParseError> {
        let span = self.current_span();
        match self.peek_kind() {
            Some(TokenKind::Number(n)) => {
                self.advance();
                Ok(n)
            }
            Some(kind) => Err(ParseError::new(
                format!("expected number, got {kind:?}"),
                span,
            )),
            None => Err(ParseError::new("expected number, got end of input", span)),
        }
    }
}

/// Parse a Jia model from a token slice.
pub fn parse_model(tokens: &[Token]) -> Result<JiaModel, ParseError> {
    let mut parser = Parser::new(tokens);

    // Optional @model tag: @model lp | @model cp
    let model_type = if matches!(parser.peek_kind(), Some(TokenKind::At)) {
        parser.advance(); // consume @
        parser.expect_ident_matching("model")?;
        let span = parser.current_span();
        let type_name = parser.expect_ident()?;
        match type_name.as_str() {
            "lp" => Some(ModelType::Lp),
            "cp" => Some(ModelType::Cp),
            other => {
                return Err(ParseError::new(
                    format!("unknown model type '{other}', expected 'lp' or 'cp'"),
                    span,
                ));
            }
        }
    } else {
        None
    };

    // model <name>
    parser.expect_ident_matching("model")?;
    let name = parser.expect_ident()?;

    let mut variables = Vec::new();
    let mut domains = Vec::new();
    let mut constraints = Vec::new();
    let mut objective = None;

    // Parse sections in any order
    while !parser.is_eof() {
        match parser.peek_kind() {
            Some(TokenKind::Ident(kw)) => match kw.as_str() {
                "variables" => {
                    variables = parser.parse_variables_block()?;
                }
                "domains" => {
                    domains = parser.parse_domains_block()?;
                }
                "constraints" => {
                    constraints = parser.parse_constraints_block()?;
                }
                "minimize" => {
                    parser.advance();
                    let expr = parser.parse_expr()?;
                    objective = Some(Objective {
                        direction: OptDirection::Minimize,
                        expr,
                    });
                }
                "maximize" => {
                    parser.advance();
                    let expr = parser.parse_expr()?;
                    objective = Some(Objective {
                        direction: OptDirection::Maximize,
                        expr,
                    });
                }
                other => {
                    return Err(ParseError::new(
                        format!("unexpected keyword '{other}', expected section or objective"),
                        parser.current_span(),
                    ));
                }
            },
            Some(kind) => {
                return Err(ParseError::new(
                    format!("unexpected token {kind:?}"),
                    parser.current_span(),
                ));
            }
            None => break,
        }
    }

    Ok(JiaModel {
        model_type,
        name,
        variables,
        domains,
        constraints,
        objective,
    })
}

/// Parse a CP model from a source string.
///
/// This is the main public entry point. It tokenizes the input and then parses
/// the token stream into a [`JiaModel`].
pub fn parse_model_str(input: &str) -> Result<JiaModel, ParseError> {
    let tokens = crate::jia_lang::lexer::tokenize(input)?;
    parse_model(&tokens)
}

/// Parse an expression from a source string (for testing).
pub fn parse_expr_str(input: &str) -> Result<Expr, ParseError> {
    let tokens = crate::jia_lang::lexer::tokenize(input)?;
    let mut parser = Parser::new(&tokens);
    parser.parse_expr()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jia_lang::ast::*;

    #[test]
    fn test_minimal_model() {
        let model = parse_model_str("model test").unwrap();
        assert_eq!(model.name, "test");
        assert!(model.variables.is_empty());
        assert!(model.domains.is_empty());
        assert!(model.constraints.is_empty());
        assert!(model.objective.is_none());
    }

    #[test]
    fn test_model_with_objective() {
        let model = parse_model_str(
            "model test\nvariables { Integer: x }\ndomains { x in 0..10 }\nminimize x",
        )
        .unwrap();
        assert_eq!(model.name, "test");
        assert!(model.objective.is_some());
        let obj = model.objective.unwrap();
        assert_eq!(obj.direction, OptDirection::Minimize);
        assert_eq!(obj.expr, Expr::Var("x".to_string()));
    }

    #[test]
    fn test_job_shop_model() {
        let input = r#"
model job_shop

variables {
  Interval: j1_op1, j1_op2
  Integer: makespan
  Set[Interval]: machine1
}

domains {
  duration(j1_op1) = 3
  duration(j1_op2) = 2
  makespan in 0..100
  machine1 = {j1_op1, j1_op2}
}

constraints {
  end_of(j1_op1) <= start_of(j1_op2)
  no_overlap(machine1)
  end_of(j1_op2) <= makespan
}

minimize makespan
"#;
        let model = parse_model_str(input).unwrap();
        assert_eq!(model.name, "job_shop");
        assert_eq!(model.variables.len(), 3);
        assert_eq!(model.constraints.len(), 3);
        assert!(model.objective.is_some());
    }

    #[test]
    fn test_unknown_section() {
        let err = parse_model_str("model test\nfoobar { }").unwrap_err();
        assert!(err.message.contains("unexpected keyword"));
    }

    #[test]
    fn test_model_tag_lp() {
        let model = parse_model_str("@model lp\nmodel test").unwrap();
        assert_eq!(model.model_type, Some(ModelType::Lp));
        assert_eq!(model.name, "test");
    }

    #[test]
    fn test_model_tag_cp() {
        let model = parse_model_str("@model cp\nmodel test").unwrap();
        assert_eq!(model.model_type, Some(ModelType::Cp));
    }

    #[test]
    fn test_model_no_tag() {
        let model = parse_model_str("model test").unwrap();
        assert_eq!(model.model_type, None);
    }

    #[test]
    fn test_model_tag_unknown() {
        let err = parse_model_str("@model foo\nmodel test").unwrap_err();
        assert!(err.message.contains("unknown model type"));
    }

    #[test]
    fn test_real_variables() {
        let model = parse_model_str("@model lp\nmodel test\nvariables { Real: x, y }").unwrap();
        assert_eq!(model.variables.len(), 1);
        assert_eq!(model.variables[0].var_type, VarType::Real);
        assert_eq!(model.variables[0].names, vec!["x", "y"]);
    }

    #[test]
    fn test_float_in_expression() {
        let expr = parse_expr_str("3.25 + x").unwrap();
        match expr {
            Expr::BinaryOp {
                op: ArithOp::Add,
                left,
                right,
            } => {
                assert_eq!(*left, Expr::Float(3.25));
                assert_eq!(*right, Expr::Var("x".to_string()));
            }
            _ => panic!("expected BinaryOp"),
        }
    }

    #[test]
    fn test_division_in_expression() {
        let expr = parse_expr_str("x / 2").unwrap();
        match expr {
            Expr::BinaryOp {
                op: ArithOp::Div, ..
            } => {}
            _ => panic!("expected division"),
        }
    }

    #[test]
    fn test_real_domain_with_inf() {
        let model = parse_model_str(
            "@model lp\nmodel test\nvariables { Real: x }\ndomains { x in 0..inf }",
        )
        .unwrap();
        assert_eq!(model.domains.len(), 1);
    }

    #[test]
    fn test_full_lp_model() {
        let input = r#"
@model lp
model simple

variables {
  Real: x, y
}

domains {
  x in 0..inf
  y in 0..inf
}

constraints {
  x + y >= 10
  x <= 8
}

minimize 2 * x + 3 * y
"#;
        let model = parse_model_str(input).unwrap();
        assert_eq!(model.model_type, Some(ModelType::Lp));
        assert_eq!(model.name, "simple");
        assert_eq!(model.variables.len(), 1);
        assert_eq!(model.variables[0].var_type, VarType::Real);
        assert_eq!(model.constraints.len(), 2);
        assert!(model.objective.is_some());
    }

    #[test]
    fn test_equality_with_single_eq() {
        // In constraints, `=` should work as equality (alongside `==`)
        let model = parse_model_str(
            "model test\nvariables { Integer: x }\ndomains { x in 0..10 }\nconstraints { x = 5 }",
        )
        .unwrap();
        assert_eq!(model.constraints.len(), 1);
        match &model.constraints[0] {
            Constraint::Comparison { op, .. } => assert_eq!(*op, CmpOp::Eq),
            _ => panic!("expected comparison"),
        }
    }
}
