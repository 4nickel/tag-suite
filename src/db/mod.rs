pub mod schema;
pub mod connection;

pub mod error {
    #[derive(Debug, Fail)]
    pub enum Error {
        #[fail(display = "connection pool error: '{}'", message)]
        ConnectionPoolError { message: String, },
    }
}

/// glob import this when wrangling diesel types
pub mod wrangle {
    pub use diesel::{prelude::*};
    pub use diesel::{AppearsOnTable, expression::array_comparison::AsInExpression, sql_types::BigInt, sqlite::Sqlite};
    pub use diesel::query_builder::{QueryId, QueryFragment};
}

pub mod import {
    pub use super::super::import::*;
}

pub mod export {
    pub use super::schema::*;
    pub use super::{connection as db};
}
pub use export::*;
