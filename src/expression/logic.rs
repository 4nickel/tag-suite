use super::{import::*};
use crate::{expression::{Ast, Operator}};
use std::cell::RefCell;
use std::marker::PhantomData;

/// Boolean expression logic used to implement
/// custom domain-specific languages. We support
/// arbitrary IDENTITY, and logical NOT, AND and OR.
///
/// ID: Identity function
/// UNI: Unary operator
/// BI: Binary operator
/// CTX: User context
/// IN: Input value
/// OUT: Output value
pub struct Logic<ID, UNI, BI, CTX, IN, OUT>
where
    for <'c, 'e, 'i> ID: Fn(&'c RefCell<CTX>, (char, &'e str), &'i IN) -> Res<OUT>,
    for <'c> UNI: Fn(&'c RefCell<CTX>, OUT) -> Res<OUT>,
    for <'c> BI: Fn(&'c RefCell<CTX>, OUT, OUT) -> Res<OUT>,
{
    id: ID,
    not: UNI,
    and: BI,
    or: BI,

    phantom_ctx: PhantomData<CTX>,
    phantom_in: PhantomData<IN>,
    phantom_out: PhantomData<OUT>,
}

impl<ID, UNI, BI, CTX, IN, OUT> Logic<ID, UNI, BI, CTX, IN, OUT>
where
    for <'c, 'e, 'i> ID: Fn(&'c RefCell<CTX>, (char, &'e str), &'i IN) -> Res<OUT>,
    for <'c> UNI: Fn(&'c RefCell<CTX>, OUT) -> Res<OUT>,
    for <'c> BI: Fn(&'c RefCell<CTX>, OUT, OUT) -> Res<OUT>,
{
    /// Create a new Logic instance
    pub fn new(id: ID, not: UNI, and: BI, or: BI) -> Self {
        Self {
            id: id,
            not: not,
            and: and,
            or: or,
            phantom_ctx: PhantomData,
            phantom_in: PhantomData,
            phantom_out: PhantomData,
        }
    }

    /// Evaluate an expression recursively
    pub fn evaluate<'c, 'i>(&self, ast: &Ast, context: &RefCell<CTX>, input: &'i IN) -> Res<OUT> {
        match ast {
            Ast::Operation(operator) => {
                match operator {
                    Operator::And(lhs, rhs) => {
                        Ok((self.and)(context,
                            self.evaluate(&lhs, context, input)?,
                            self.evaluate(&rhs, context, input)?)?)
                    }
                    Operator::Or(lhs, rhs) => {
                        Ok((self.or)(context,
                            self.evaluate(&lhs, context, input)?,
                            self.evaluate(&rhs, context, input)?)?)
                    }
                    Operator::Not(val) => {
                        Ok((self.not)(context,
                            self.evaluate(&val, context, input)?)?)
                    }
                }
            }
            Ast::Expr(c, ref s) => {
                Ok((self.id)(context, (*c, s), input)?)
            }
        }
    }
}
