//! Term parsing: variables, names, atomic formulas, typed lists.

use crate::ast::{AtomicFormula, FunctionTerm, Name, Term, TypedGroup, TypedList, Variable};
use crate::error::ParseError;

use super::cursor::Parser;

/// Parse a typed list of names (e.g. for `:objects`, `:constants`, `:types`).
pub(super) fn parse_typed_list_names(p: &mut Parser) -> Result<TypedList<Name>, ParseError> {
    parse_typed_list(p, |p| p.expect_symbol())
}

/// Parse a typed list of variables (e.g. for action `:parameters`).
pub(super) fn parse_typed_list_vars(p: &mut Parser) -> Result<TypedList<Variable>, ParseError> {
    parse_typed_list(p, |p| p.expect_variable())
}

/// Generic typed list parser parameterized over item type.
pub(super) fn parse_typed_list<T, F>(
    p: &mut Parser,
    mut item_fn: F,
) -> Result<Vec<TypedGroup<T>>, ParseError>
where
    T: serde::Serialize,
    F: FnMut(&mut Parser) -> Result<T, ParseError>,
{
    let mut groups: Vec<TypedGroup<T>> = Vec::new();
    let mut pending: Vec<T> = Vec::new();

    while !p.at_rparen() && p.peek().is_some() {
        // Check for `-` type separator
        if p.at_symbol("-") {
            p.advance()?;
            let type_name = parse_type_name(p)?;
            groups.push(TypedGroup {
                items: std::mem::take(&mut pending),
                type_name: Some(type_name),
            });
            continue;
        }

        // Handle `-typename` (no space after `-`), e.g. `-goods` meaning `- goods`
        if let Some(tok) = p.peek() {
            if let crate::lexer::TokenKind::Symbol(s) = &tok.kind {
                if s.starts_with('-') && s.len() > 1 && !pending.is_empty() {
                    let type_name = s[1..].to_string();
                    p.advance()?;
                    groups.push(TypedGroup {
                        items: std::mem::take(&mut pending),
                        type_name: Some(type_name),
                    });
                    continue;
                }
            }
        }

        let item = item_fn(p)?;
        pending.push(item);
    }

    if !pending.is_empty() {
        groups.push(TypedGroup {
            items: pending,
            type_name: None,
        });
    }

    Ok(groups)
}

/// Parse a type name which can be either a simple name or `(either type1 type2 ...)`
pub(super) fn parse_type_name(p: &mut Parser) -> Result<Name, ParseError> {
    if p.at_lparen() {
        p.expect_lparen()?;
        p.expect_symbol_eq("either")?;
        let mut types = Vec::new();
        while !p.at_rparen() {
            types.push(p.expect_symbol()?);
        }
        p.expect_rparen()?;
        // Represent either-types as a joined string for now
        Ok(format!("either:{}", types.join(":")))
    } else {
        p.expect_symbol()
    }
}

/// Parse a term (variable or constant name).
pub(super) fn parse_term(p: &mut Parser) -> Result<Term, ParseError> {
    let tok = p.advance()?;
    match &tok.kind {
        crate::lexer::TokenKind::Symbol(s) => Ok(Term::Name(s.clone())),
        crate::lexer::TokenKind::Variable(v) => Ok(Term::Variable(v.clone())),
        _ => Err(ParseError::new(
            format!("expected term (name or variable), got {:?}", tok.kind),
            tok.span,
        )),
    }
}

/// Parse a term but only names (for init section where variables don't appear)
pub(super) fn parse_term_name_only(p: &mut Parser) -> Result<Term, ParseError> {
    let tok = p.advance()?;
    match &tok.kind {
        crate::lexer::TokenKind::Symbol(s) => Ok(Term::Name(s.clone())),
        _ => Err(ParseError::new(
            format!("expected name, got {:?}", tok.kind),
            tok.span,
        )),
    }
}

/// Parse `(predicate-name term*)`.
pub(super) fn parse_atomic_formula(p: &mut Parser) -> Result<AtomicFormula, ParseError> {
    p.expect_lparen()?;
    let name = p.expect_symbol()?;
    let mut args = Vec::new();
    while !p.at_rparen() {
        args.push(parse_term(p)?);
    }
    p.expect_rparen()?;
    Ok(AtomicFormula { name, args })
}

/// Parse a function term, either `(func-name term*)` or a bare 0-arity name.
pub(super) fn parse_function_term(p: &mut Parser) -> Result<FunctionTerm, ParseError> {
    if p.at_lparen() {
        p.expect_lparen()?;
        let name = p.expect_symbol()?;
        let mut args = Vec::new();
        while !p.at_rparen() {
            args.push(parse_term(p)?);
        }
        p.expect_rparen()?;
        Ok(FunctionTerm { name, args })
    } else {
        // 0-arity function: bare name
        let name = p.expect_symbol()?;
        Ok(FunctionTerm {
            name,
            args: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::cursor::Parser;

    #[test]
    fn typed_lists_support_dash_prefixed_types_and_either() {
        let tokens = tokenize("a b -thing c - (either x y))").unwrap();
        let mut parser = Parser::new(&tokens);
        let groups = parse_typed_list_names(&mut parser).unwrap();

        assert_eq!(groups[0].type_name.as_deref(), Some("thing"));
        assert_eq!(groups[1].type_name.as_deref(), Some("either:x:y"));

        let tokens = tokenize("?a ?b ?c").unwrap();
        let mut parser = Parser::new(&tokens);
        let groups = parse_typed_list_vars(&mut parser).unwrap();
        assert_eq!(groups[0].items.len(), 3);
        assert!(groups[0].type_name.is_none());
    }

    #[test]
    fn term_parsers_report_invalid_tokens() {
        let tokens = tokenize(")").unwrap();
        let mut parser = Parser::new(&tokens);
        assert!(parse_term(&mut parser).is_err());

        let tokens = tokenize("?x").unwrap();
        let mut parser = Parser::new(&tokens);
        assert!(parse_term_name_only(&mut parser).is_err());
    }

    #[test]
    fn function_term_supports_bare_zero_arity_name() {
        let tokens = tokenize("total-cost").unwrap();
        let mut parser = Parser::new(&tokens);
        let function = parse_function_term(&mut parser).unwrap();

        assert_eq!(function.name, "total-cost");
        assert!(function.args.is_empty());
    }
}
