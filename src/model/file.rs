use diesel::{self, prelude::*, query_builder::{BoxedSelectStatement}};
use diesel::{expression::{BoxableExpression}, sqlite::Sqlite, sql_types::{Bool, BigInt}};
use super::{import::*};

pub const IDS:
    (files::id, files::path) =
    (files::id, files::path);

/// A 'SELECT' statement this table, returning BigInts
pub type Select<'e> = BoxedSelectStatement<'e, BigInt, files::table, Sqlite>;
/// An expression against this table, returning a Bool
pub type Boolean<'e> = Box<dyn BoxableExpression<files::table, Sqlite, SqlType = Bool> + 'e>;

#[derive(Debug, Identifiable, Queryable, Associations, PartialEq, Eq, Hash, Clone)]
#[table_name="files"]
pub struct File {
    pub id: i64,
    pub kind: i64,
    pub path: String,
}

#[derive(Debug, Insertable)]
#[table_name="files"]
pub struct Insert<'a> {
    pub kind: i64,
    pub path: &'a str,
}

impl File {

    /// Insert files into the database
    pub fn insert_all(values: &Vec<Insert>, c: &db::Connection) -> Res<Vec<Self>> {
        c.get().transaction::<_, Error, _>(|| {
            diesel::insert_into(files::table).values(values).execute(c.get())?;
            let paths: Vec<&str> = values.iter().map(|e| e.path).collect();
            Ok(files::table.filter(files::path.eq_any(&paths)).get_results(c.get())?)
        })
    }

    /// Delete files from the database, by id
    pub fn delete_ids(ids: &Vec<i64>, c: &db::Connection) -> Res<usize> {
        Ok(diesel::delete(files::table.filter(files::id.eq_any(ids))).execute(c.get())?)
    }

    /// Delete files from the database, by path
    pub fn delete_paths(paths: &Vec<&str>, c: &db::Connection) -> Res<usize> {
        Ok(diesel::delete(files::table.filter(files::path.eq_any(paths))).execute(c.get())?)
    }
}

use core::fmt::{Display, Formatter, Error as FmtError};
impl Display for File {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        write!(f, "File[{}]", self.id)
    }
}
