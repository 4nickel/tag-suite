mod tokenizer;
mod parser;
mod expression;
pub mod namespace; // TODO: why pub?
mod logic;
mod expansion;
mod comparison;

pub mod import {
    pub use super::super::import::*;
}

pub mod export {
    pub use super::comparison::{Comparables, Comparison, Parameters};
    pub use super::tokenizer::{Tokenizer, Token, ValueToken, BinaryToken, UnaryToken, CloseToken};
    pub use super::parser::{Parser, Operator, Ast};
    pub use super::expression::{Expression};
    pub use super::namespace::{Namespec, Namespace};
    pub use super::logic::{Logic};
    pub use super::expansion::{Expansions};
}
pub use export::*;

pub mod error {
    #[derive(Debug, Fail)]
    pub enum Error {

        // tokenizer errors
        #[fail(display = "invalid character: '{}'", c)]
        InvalidCharacter { c: char, },
        #[fail(display = "unexpected character: '{}' (expected '{}')", found, expected)]
        UnexpectedCharacter { expected: char, found: char },
        #[fail(display = "unclosed delimiter: '{}'", delimiter)]
        UnclosedDelimiter { delimiter: char, },
        #[fail(display = "unexpected end of stream")]
        UnexpectedEof { },

        // parser errors
        #[fail(display = "missing value for operation")]
        MissingValue { },

        // expression errors
        #[fail(display = "empty expression")]
        EmptyExpression { },

        // expansion errors
        #[fail(display = "unknown expansion: '{}'", key)]
        UnknownExpansion { key: String },
        #[fail(display = "invalid identifier: '{}'", id)]
        InvalidIdentifier { id: String },
        #[fail(display = "recursion limit exceeded while evaluating '{}', limit = {}", key, limit)]
        RecursionLimitReached { key: String, limit: usize },

        // comparison errors
        #[fail(display = "failed to parse comparison: '{}'", expression)]
        FailedCapture { expression: String },
        #[fail(display = "unknown variable: '{}'", variable)]
        UnknownVariable { variable: String },
        #[fail(display = "unknown operator: '{}'", operator)]
        UnknownOperator { operator: String }
    }
}
