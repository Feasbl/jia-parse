//! Jia modelling language: parser, AST, and analysis.
//!
//! The `.jia` format is a unified modelling language for constraint programming,
//! linear programming, and (future) planning problems. The `@model` tag declares
//! the problem paradigm (`cp`, `lp`), and the parser validates constructs
//! against the declared type.
//!
//! # Example
//!
//! ```
//! let model = jia_parse::jia::parse_model_str("model empty").unwrap();
//! ```

pub mod analysis;
pub mod ast;
pub mod lexer;
pub mod parser;

pub use parser::parse_model_str;
