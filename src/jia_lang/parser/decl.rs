//! Variables block parser for the CP language.
//!
//! Parses declarations like:
//! ```text
//! variables {
//!   Interval: a, b, c
//!   Integer: makespan
//!   Set[Interval]: machine1, machine2
//! }
//! ```

use super::Parser;
use crate::error::ParseError;
use crate::jia_lang::ast::{VarDecl, VarType};
use crate::jia_lang::lexer::TokenKind;

impl<'a> Parser<'a> {
    /// Parse a `variables { ... }` block.
    pub fn parse_variables_block(&mut self) -> Result<Vec<VarDecl>, ParseError> {
        self.expect_ident_matching("variables")?;
        self.expect_token(TokenKind::LBrace)?;

        let mut decls = Vec::new();
        while self.peek_kind() != Some(TokenKind::RBrace) {
            decls.push(self.parse_var_decl()?);
        }
        self.expect_token(TokenKind::RBrace)?;
        Ok(decls)
    }

    /// Parse a single variable declaration: `Type: name1, name2, ...`
    fn parse_var_decl(&mut self) -> Result<VarDecl, ParseError> {
        let var_type = self.parse_var_type()?;
        self.expect_token(TokenKind::Colon)?;
        let names = self.parse_ident_list()?;
        Ok(VarDecl { names, var_type })
    }

    /// Parse a variable type: `Interval`, `Integer`, `Set[Interval]`, `Set[Integer]`
    fn parse_var_type(&mut self) -> Result<VarType, ParseError> {
        let name = self.expect_ident()?;
        match name.as_str() {
            "Interval" => Ok(VarType::Interval),
            "Integer" => Ok(VarType::Integer),
            "Real" => Ok(VarType::Real),
            "Set" => {
                self.expect_token(TokenKind::LBracket)?;
                let inner = self.expect_ident()?;
                self.expect_token(TokenKind::RBracket)?;
                match inner.as_str() {
                    "Interval" => Ok(VarType::SetInterval),
                    "Integer" => Ok(VarType::SetInteger),
                    _ => Err(ParseError::new(
                        format!("expected 'Interval' or 'Integer' in Set[...], got '{inner}'"),
                        self.current_span(),
                    )),
                }
            }
            _ => Err(ParseError::new(
                format!("expected type (Interval, Integer, Real, Set[...]), got '{name}'"),
                self.current_span(),
            )),
        }
    }

    /// Parse a comma-separated list of identifiers (at least one).
    pub fn parse_ident_list(&mut self) -> Result<Vec<String>, ParseError> {
        let mut names = vec![self.expect_ident()?];
        while self.peek_kind() == Some(TokenKind::Comma) {
            self.advance();
            names.push(self.expect_ident()?);
        }
        Ok(names)
    }
}

#[cfg(test)]
mod tests {
    use crate::jia_lang::ast::{VarDecl, VarType};
    use crate::jia_lang::lexer::tokenize;
    use crate::jia_lang::parser::Parser;

    #[test]
    fn test_variables_block() {
        let input = "variables { Interval: a, b Integer: makespan Set[Interval]: m1, m2 }";
        let tokens = tokenize(input).unwrap();
        let mut parser = Parser::new(&tokens);
        let decls = parser.parse_variables_block().unwrap();
        assert_eq!(
            decls,
            vec![
                VarDecl {
                    names: vec!["a".to_string(), "b".to_string()],
                    var_type: VarType::Interval,
                },
                VarDecl {
                    names: vec!["makespan".to_string()],
                    var_type: VarType::Integer,
                },
                VarDecl {
                    names: vec!["m1".to_string(), "m2".to_string()],
                    var_type: VarType::SetInterval,
                },
            ]
        );
    }

    #[test]
    fn test_set_integer_type() {
        let input = "variables { Set[Integer]: costs }";
        let tokens = tokenize(input).unwrap();
        let mut parser = Parser::new(&tokens);
        let decls = parser.parse_variables_block().unwrap();
        assert_eq!(decls[0].var_type, VarType::SetInteger);
    }
}
