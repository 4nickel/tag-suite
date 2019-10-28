use diesel::{dsl::*, query_builder::{BoxedSelectStatement}, query_source::joins::{JoinOn, Join, Inner}};
use diesel::{expression::{SqlLiteral, BoxableExpression}, sqlite::Sqlite, sql_types::{Bool, BigInt}};
use crate::{model::export::*, util::{sql}};
use super::import::*;

pub const IDS:
    (file_tags::file_id, file_tags::tag_id) =
    (file_tags::file_id, file_tags::tag_id);

// These are some helper types to wrangle the mess
// of diesels type gibberish.
pub type SqlType = <(file_tags::file_id) as Expression>::SqlType;
pub type FilesJoined = JoinOn<Join<file_tags::table, files::table, Inner>, Eq<Nullable<file_tags::file_id>, Nullable<files::id>>>;
pub type TagsJoined = JoinOn<Join<file_tags::table, tags::table, Inner>, Eq<Nullable<file_tags::tag_id>, Nullable<tags::id>>>;
pub type FileTagsJoined = JoinOn<Join<FilesJoined, tags::table, Inner>, Eq<Nullable<file_tags::tag_id>, Nullable<tags::id>>>;

/// A 'SELECT' statement against joined File & Tag tables, returning BigInts
pub type SelectJoined<'e> = BoxedSelectStatement<'e, BigInt, FileTagsJoined, Sqlite>;
/// An expression against joined File & Tag tables, returning a Bool
pub type BooleanJoined<'e> = Box<dyn BoxableExpression<file_tags::table, Sqlite, SqlType = Bool> + 'e>;

#[derive(Debug, Identifiable, Insertable, Queryable, Associations, PartialEq, Eq, Hash, Clone, Copy)]
#[belongs_to(File)]
#[belongs_to(Tag)]
#[primary_key(file_id, tag_id)]
#[table_name="file_tags"]
pub struct FileTag {
    pub file_id: i64,
    pub tag_id: i64,
}

impl FileTag {

    /// WHERE (f, t) IN (VALUES ( ... ))
    pub fn with_pairs<'a>(pairs: &'a Vec<(i64, i64)>, keyword: bool) -> SqlLiteral<Bool> {
        sql::with_pairs(("`file_tags`.`file_id`", "`file_tags`.`tag_id`"), pairs, keyword)
    }

    /// WHERE (f, t) = (Fid, Tid)
    pub fn with_pair(pair: &(i64, i64)) -> And<Eq<file_tags::file_id, i64>, Eq<file_tags::tag_id, i64>> {
        file_tags::file_id.eq(pair.0).and(file_tags::tag_id.eq(pair.1))
    }

    /// Insert file-tags into the database
    pub fn insert_all(values: &Vec<Self>, c: &db::Connection) -> Res<()> {
        diesel::insert_into(file_tags::table).values(values).execute(c.get())?;
        Ok(())
    }

    /// Delete file-tags from the database
    pub fn delete_ids(pairs: &Vec<(i64, i64)>, c: &db::Connection) -> Res<usize> {
        Ok(diesel::delete(file_tags::table.filter(Self::with_pairs(pairs, true))).execute(c.get())?)
    }
}

use core::fmt::{Display, Formatter, Error as FmtError};
impl Display for FileTag {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        write!(f, "FileTag[{}][{}]", self.file_id, self.tag_id)
    }
}
