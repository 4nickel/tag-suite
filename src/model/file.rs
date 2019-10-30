use super::{import::*};
use diesel::{self, prelude::*, query_builder::{BoxedSelectStatement}};
use diesel::{expression::{BoxableExpression}, sqlite::Sqlite, sql_types::{Bool, BigInt}};

pub const IDS:
    (files::id, files::path, files::kind) =
    (files::id, files::path, files::kind);

/// A 'SELECT' statement this table, returning BigInts
pub type Select<'e> = BoxedSelectStatement<'e, BigInt, files::table, Sqlite>;
/// An expression against this table, returning a Bool
pub type Boolean<'e> = Box<dyn BoxableExpression<files::table, Sqlite, SqlType = Bool> + 'e>;

#[derive(Debug, Identifiable, Queryable, Associations, PartialEq, Eq, Hash, Clone)]
#[table_name="files"]
pub struct File {
    pub id: Fid,
    pub kind: Kind,
    pub path: String,
}

#[derive(Debug, Insertable)]
#[table_name="files"]
pub struct Insert<'a> {
    pub kind: i64,
    pub path: &'a str,
}

pub trait FileExt {
    fn id(&self) -> Fid;
    fn kind(&self) -> Kind;
    fn path<'a>(&'a self) -> &'a str;
    fn borrow<'a>(&'a self) -> Borrow<'a> {
        Borrow {
            id: self.id(),
            path: self.path(),
            kind: self.kind(),
        }
    }
}

impl FileExt for File {
    fn id(&self) -> Fid { self.id }
    fn kind(&self) -> Kind { self.kind }
    fn path<'a>(&'a self) -> &'a str { self.path.as_str() }
}

impl FileExt for FCol {
    fn id(&self) -> Fid { self.0 }
    fn kind(&self) -> Kind { self.2 }
    fn path<'a>(&'a self) -> &'a str { self.1.as_str() }
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct Borrow<'a> {
    pub id: Fid,
    pub kind: Kind,
    pub path: &'a str,
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct Ident<'a> {
    pub path: &'a str,
    pub kind: Kind,
}

impl<'a> Borrow<'a> {
    pub fn ident(&self) -> Ident<'a> {
        Ident { path: self.path, kind: self.kind }
    }
}

impl File {

    /// Insert files into the database
    pub fn insert_all(values: &Vec<Insert>, c: &db::Connection) -> Res<Vec<Self>> {
        c.get().transaction::<_, Error, _>(|| {
            diesel::insert_into(files::table).values(values).execute(c.get())?;
            Ok(files::table.filter(
                files::path.eq_any(values.iter().map(|e| e.path))
            ).get_results(c.get())?)
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
