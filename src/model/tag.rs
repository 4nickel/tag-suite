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
