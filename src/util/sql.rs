use crate::db::export::*;
use diesel::{self, prelude::*};

// {{{ Sqlite

pub fn last_insert_rowid(c: &db::Connection) -> i64
{
    no_arg_sql_function!(last_insert_rowid, diesel::sql_types::BigInt);
    diesel::select(last_insert_rowid).first(c.get()).unwrap_or(0i64)
}

use diesel::sql_types::{Bool, Text};
use diesel::expression::sql_literal::{SqlLiteral, sql};
pub fn with_rowid(oid: i64) -> SqlLiteral<Bool>
{
    sql::<Bool>(&format!("OID = {}", oid))
}

use std::fmt::Display;
pub fn with_pairs<'a, T: Display>(columns: (&'a str, &'a str), values: &'a Vec<(T, T)>, keyword: bool) -> SqlLiteral<Bool>
{
    let len = values.len();
    if len == 0 { return sql_false() }
    let mut s = String::with_capacity(values.len() * 5);
    for (i, (a, b)) in values.iter().enumerate() {
        s.push_str(&format!("('{}','{}')", a, b));
        if i != len-1 { s.push(','); }
    }
    if keyword {
        sql::<Bool>(&format!("({}, {}) IN (VALUES {})", columns.0, columns.1, s))
    } else {
        sql::<Bool>(&format!("({}, {}) IN ({})", columns.0, columns.1, s))
    }
}

pub fn sql_text(text: &str) -> SqlLiteral<Text>
{ sql::<Text>(&format!("'{}'", text)) }

pub fn sql_false() -> SqlLiteral<Bool>
{ sql::<Bool>(&format!("FALSE")) }

pub fn sql_true() -> SqlLiteral<Bool>
{ sql::<Bool>(&format!("TRUE")) }

// }}}
