use super::{import::*, error::{Error as E}};
use std::cell::RefCell;
use diesel::prelude::*;
use crate::{db::schema::{files, tags, file_tags}, model::file_tag::{BooleanJoined as Boolean, SelectJoined as Select}, expression::{Logic, Ast}};
use crate::expression::{Namespec};

/// Custom querying Dsl
/// The query language allows us to dynamically
/// construct queries from expressions.
pub struct Dsl<'a>(
    Logic
    <
        fn(&RefCell<()>, (char, &str), &()) -> Res<Boolean<'a>>,
        fn(&RefCell<()>, Boolean<'a>) -> Res<Boolean<'a>>,
        fn(&RefCell<()>, Boolean<'a>, Boolean<'a>) -> Res<Boolean<'a>>,
        (),
        (),
        Boolean<'a>
    >
);

impl<'a> Dsl<'a> {

    pub fn new() -> Self {
        Self(Logic::new(
            Self::id,
            Self::not,
            Self::and,
            Self::or,
        ))
    }

    pub fn evaluate(&self, ast: &Ast, context: &RefCell<()>, input: &()) -> Res<Boolean<'a>> {
        Ok(self.0.evaluate(ast, context, input)?)
    }

    /// Select file ids
    fn fids() -> Select<'a> {
        file_tags::table
            .inner_join(files::table)
            .inner_join(tags::table)
            .select(files::id)
            .into_boxed()
    }

    /// Select file ids WHERE file.path LIKE
    fn path_fids(exp: &str) -> Select<'a> {
        use crate::util::sql::sql_text;
        let concat = sql_text("::").concat(files::path);
        Self::fids().filter(concat.like(exp.to_string()))
    }

    /// Select file ids WHERE tag.name LIKE
    fn tags_fids(exp: &str) -> Select<'a> {
        use crate::util::sql::sql_text;
        let concat = sql_text("::").concat(tags::name);
        Self::fids().filter(concat.like(exp.to_string()))
    }

    /// Dispatch the subselect
    fn subselect_fids(exp: (char, &str)) -> Res<Select<'a>> {
        use crate::expression::namespace::constants::*;
        match exp.0 {
            '=' => {
                let expression = Namespec::apply_shorthand_syntax(exp.1);
                let (canonical, user) = Namespec::canonicalize_user_expression(&expression);
                match canonical.get_reserved().as_str() {
                    RESERVED_TAG => { Ok(Self::tags_fids(&user.to_string())) }
                    RESERVED_PATH => { Ok(Self::path_fids(&user.to_string())) }
                    e => { Err(E::InvalidNamespace{ name: e.into() }.into()) }
                }
            },
            _ => Err(E::InvalidModifier{ c: exp.0 }.into()),
        }
    }

    /// Logical AND
    fn and(_context: &RefCell<()>, a: Boolean<'a>, b: Boolean<'a>) -> Res<Boolean<'a>> {
        Ok(box a.and(b))
    }

    /// Logical OR
    fn or(_context: &RefCell<()>, a: Boolean<'a>, b: Boolean<'a>) -> Res<Boolean<'a>> {
        Ok(box a.or(b))
    }

    /// Logical NOT
    fn not(_context: &RefCell<()>, v: Boolean<'a>) -> Res<Boolean<'a>> {
        Ok(box diesel::dsl::not(v))
    }

    /// Logical ID
    fn id(_context: &RefCell<()>, id: (char, &str), _input: &()) -> Res<Boolean<'a>> {
        Ok(box file_tags::file_id.eq_any(Self::subselect_fids(id)?))
    }
}
