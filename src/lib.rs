//! Parser library for PDDL and `.jia` model files.

pub mod ast;
pub mod error;
pub mod jia_lang;
pub mod lexer;
pub mod parser;

/// PDDL parser facade.
pub mod pddl {
    pub use crate::ast;
    pub use crate::lexer;
    pub use crate::parser;
    pub use crate::parser::{parse_domain, parse_domain_str, parse_problem, parse_problem_str};
}

/// `.jia` model parser facade.
pub mod jia {
    pub use crate::jia_lang::analysis;
    pub use crate::jia_lang::ast;
    pub use crate::jia_lang::lexer;
    pub use crate::jia_lang::parse_model_str;
    pub use crate::jia_lang::parser;
}

pub use error::{ParseError, Span};
