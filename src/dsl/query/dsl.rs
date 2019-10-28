use super::{import::*, Context, error::{Error as E}};
use crate::{db::export::*, model::export::*, expression::export::*};
use std::cell::RefCell;

/// Custom querying Dsl.
/// The query language allows us to dynamically
/// construct queries from expressions.
pub struct Dsl<'a>(
    Logic
    <
        for<'c, 'e, 'i> fn(&'c RefCell<Context<'a>>, (char, &'e str), &'i ()) -> Res<Boolean<'a>>,
        for<'c> fn(&'c RefCell<Context<'a>>, Boolean<'a>) -> Res<Boolean<'a>>,
        for<'c> fn(&'c RefCell<Context<'a>>, Boolean<'a>, Boolean<'a>) -> Res<Boolean<'a>>,
        Context<'a>,
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

    pub fn evaluate(&self, ast: &Ast, context: &RefCell<Context<'a>>, input: &()) -> Res<Boolean<'a>> {
        Ok(self.0.evaluate(ast, context, input)?)
    }

    /// Select file ids WHERE file.kind = ?.
    fn kind_fids(exp: &str, c: &db::Connection) -> Res<Vec<i64>> {
        use crate::util::file;
        let kind = match exp {
            "::file" => file::UnixFileType::File as i64,
            "::fifo" => file::UnixFileType::Fifo as i64,
            "::link" => file::UnixFileType::Symlink as i64,
            "::dir" => file::UnixFileType::Dir as i64,
            "::socket" => file::UnixFileType::Socket as i64,
            "::blkdev" => file::UnixFileType::BlockDevice as i64,
            "::chrdev" => file::UnixFileType::CharDevice as i64,
            s => panic!("unknown kind: '{}'", s),
        };
        Ok(files::table
            .select(files::id)
            .filter(files::kind.eq(kind))
            .get_results(c.get())?)
    }

    /// Select file ids WHERE file.path LIKE.
    fn path_fids(exp: &str, c: &db::Connection) -> Res<Vec<i64>> {
        use crate::util::sql::sql_text;
        let concat = sql_text("::").concat(files::path);
        Ok(files::table
            .select(files::id)
            .filter(concat.like(exp.to_string()))
            .get_results(c.get())?)
    }

    /// Select file ids WHERE tag.name LIKE.
    fn tags_tids(exp: &str, c: &db::Connection) -> Res<Vec<i64>> {
        use crate::util::sql::sql_text;
        let concat = sql_text("::").concat(tags::name);
        Ok(tags::table
            .select(tags::id)
            .filter(concat.like(exp.to_string()))
            .get_results(c.get())?)
    }

    fn tags_fids(exp: &str, c: &db::Connection) -> Res<Vec<i64>> {
        let tids = Self::tags_tids(exp, c)?;
        Ok(file_tags::table
            .select(file_tags::file_id)
            .filter(file_tags::tag_id.eq_any(tids))
            .get_results(c.get())?)
    }

    /// Dispatch the subselect.
    fn subselect_fids(exp: (char, &str), c: &db::Connection) -> Res<Select<'a>> {
        use crate::expression::namespace::constants::*;
        let result = match exp.0 {
            '=' => {
                let expression = Namespec::apply_shorthand_syntax(exp.1);
                let (canonical, user) = Namespec::canonicalize_user_expression(&expression);
                let fids = match canonical.get_reserved().as_str() {
                    RESERVED_TAG => { Self::tags_fids(&user.to_string(), c)? }
                    RESERVED_PATH => { Self::path_fids(&user.to_string(), c)? }
                    RESERVED_KIND => { Self::kind_fids(&user.to_string(), c)? }
                    e => { return Err(E::InvalidNamespace{ name: e.into() }.into()) }
                };
                Ok(files::table.filter(files::id.eq_any(fids)).select(files::id).into_boxed())
            },
            _ => Err(E::InvalidModifier{ c: exp.0 }.into()),
        };
        result
    }

    /// Logical AND
    fn and<'c>(_context: &'c RefCell<Context<'a>>, a: Boolean<'a>, b: Boolean<'a>) -> Res<Boolean<'a>> {
        Ok(box a.and(b))
    }

    /// Logical OR
    fn or<'c>(_context: &'c RefCell<Context<'a>>, a: Boolean<'a>, b: Boolean<'a>) -> Res<Boolean<'a>> {
        Ok(box a.or(b))
    }

    /// Logical NOT
    fn not<'c>(_context: &'c RefCell<Context<'a>>, v: Boolean<'a>) -> Res<Boolean<'a>> {
        Ok(box diesel::dsl::not(v))
    }

    /// Logical ID
    fn id<'c>(context: &'c RefCell<Context<'a>>, id: (char, &str), _input: &()) -> Res<Boolean<'a>> {
        Ok(box files::id.eq_any(Self::subselect_fids(id, context.borrow().connection())?))
    }
}
