use super::import::*;

pub const IDS:
    (tags::id, tags::name) =
    (tags::id, tags::name);

#[derive(Debug, Identifiable, AsChangeset, Queryable, Associations, PartialEq, Eq, Hash, Clone)]
#[table_name="tags"]
pub struct Tag {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Insertable)]
#[table_name="tags"]
pub struct Insert<'a> {
    pub name: &'a str,
}

pub trait TagExt {
    fn id(&self) -> Fid;
    fn name<'a>(&'a self) -> &'a str;
    fn borrow<'a>(&'a self) -> Borrow<'a> {
        Borrow {
            id: self.id(),
            name: self.name(),
        }
    }
}

impl TagExt for Tag {
    fn id(&self) -> Tid { self.id }
    fn name<'a>(&'a self) -> &'a str { self.name.as_str() }
}

impl TagExt for TCol {
    fn id(&self) -> Tid { self.0 }
    fn name<'a>(&'a self) -> &'a str { self.1.as_str() }
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct Borrow<'a> {
    pub id: Tid,
    pub name: &'a str,
}

impl<'a> Borrow<'a> {
    pub fn ident(&self) -> Ident<'a> {
        Ident { name: self.name }
    }
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct Ident<'a> {
    pub name: &'a str,
}

impl Tag {

    /// Insert tags into the database
    pub fn insert_all(values: &Vec<Insert>, c: &db::Connection) -> Res<Vec<Self>> {
        c.get().transaction::<_, Error, _>(|| {
            diesel::insert_into(tags::table).values(values).execute(c.get())?;
            let names: Vec<&str> = values.iter().map(|e| e.name).collect();
            Ok(tags::table.filter(tags::name.eq_any(&names)).get_results(c.get())?)
        })
    }

    /// Delete tags from the database, by id
    pub fn delete_ids(ids: &Vec<i64>, c: &db::Connection) -> Res<usize> {
        Ok(diesel::delete(tags::table.filter(tags::id.eq_any(ids))).execute(c.get())?)
    }

    /// Delete tags from the database, by name
    pub fn delete_names(names: &Vec<&str>, c: &db::Connection) -> Res<usize> {
        Ok(diesel::delete(tags::table.filter(tags::name.eq_any(names))).execute(c.get())?)
    }
}

use core::fmt::{Display, Formatter, Error as FmtError};
impl Display for Tag {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        write!(f, "Tag[{}]", self.id)
    }
}
