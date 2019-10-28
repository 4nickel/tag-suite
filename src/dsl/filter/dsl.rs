use super::{import::*, Context, error::{Error as E}};
use crate::{app::data::query::export::* ,expression::export::*};
use std::{cell::RefCell, process::Command};

/// Custom filtering Dsl
/// The filter is able to match an attrs
/// parameters, such as it's tags or path
/// against regexes.
/// Additionally we support comparison
/// syntax for other parameters.
pub struct Dsl<'q>(
    Logic
    <
        fn(&RefCell<Context>, (char, &str), &FileView<'q>) -> Res<bool>,
        fn(&RefCell<Context>, bool) -> Res<bool>,
        fn(&RefCell<Context>, bool, bool) -> Res<bool>,
        Context,
        FileView<'q>,
        bool
    >
);

impl<'q> Dsl<'q> {

    /// Create a new Dsl instance
    pub fn new() -> Self {
        Self(Logic::new(
            Self::id,
            Self::not,
            Self::and,
            Self::or,
        ))
    }

    /// Evaluate an expression
    pub fn evaluate(&self, ast: &Ast, context: &RefCell<Context>, input: &FileView<'q>) -> Res<bool> {
        self.0.evaluate(ast, context, input)
    }

    /// Logical AND
    fn and(_context: &RefCell<Context>, a: bool, b: bool) -> Res<bool> {
        Ok(a && b)
    }

    /// Logical OR
    fn or(_context: &RefCell<Context>, a: bool, b: bool) -> Res<bool> {
        Ok(a || b)
    }

    /// Logical NOT
    fn not(_context: &RefCell<Context>, v: bool) -> Res<bool> {
        Ok(!v)
    }

    /// The identity of our expression is, in this
    /// case the result of evaluating the underlying
    /// regex match or comparison.
    fn id(context: &RefCell<Context>, exp: (char, &str), attr: &FileView<'q>) -> Res<bool> {
        match exp.0 {
            '=' => { Ok(Self::match_expr(context, attr, exp.1)?) },
            '?' => { Ok(Self::match_cmps(context, attr, exp.1)?) },
            '$' => { Ok(Self::match_exec(context, attr, exp.1)?) },
            _ => { Err(E::InvalidModifier { c: exp.0 }.into()) },
        }
    }

    /// Match the attrs tags against the expression
    fn match_expr(context: &RefCell<Context>, attr: &FileView<'q>, exp: &str) -> Res<bool> {
        use crate::expression::namespace::constants::*;
        let mut context = context.borrow_mut();
        let (canon, user) = Namespec::canonicalize_user_expression(exp);
        let regex = context.regex(&user.to_string()[2..])?;
        match canon.get_reserved().as_str() {
            RESERVED_TAG => {
                for tag in attr.iter() {
                    if regex.is_match(tag.name()) { return Ok(true) }
                }
                Ok(false)
            }
            RESERVED_PATH => {
                Ok(regex.is_match(&attr.path()))
            }
            e => { Err(E::InvalidNamespace{ name: e.into() }.into()) }
        }
    }

    /// Evaluate the parameter comparison expression
    fn match_cmps(context: &RefCell<Context>, attr: &FileView<'q>, exp: &str) -> Res<bool> {
        let mut context = context.borrow_mut();
        let comparison = context.comparison(exp)?;
        let comparables: Comparables = attr.parameters().into();
        Ok(comparables.evaluate(comparison)?)
    }

    /// Evaluate the parameter comparison expression
    fn match_exec(_context: &RefCell<Context>, attr: &FileView<'q>, exp: &str) -> Res<bool> {
        use shell_escape::unix;
        use std::borrow::Cow;
        // TODO: error handling and performance is bad here
        //       perhaps we can do something about the errors
        //       at least
        let escaped = unix::escape(Cow::Borrowed(attr.path()));
        let command = exp.replace("{}", &format!("{}", escaped));
        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .output()?;
        Ok(output.status.success())
    }
}
