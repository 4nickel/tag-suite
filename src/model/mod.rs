pub mod file;
pub mod tag;
pub mod file_tag;

pub mod prelude {
    pub type Uid = i64;
    pub type Kind = i64;
    pub type Fid = Uid;
    pub type Bid = Uid;
    pub type Tid = Uid;
    pub type Ids = (Fid, Tid);
    pub type Row = (Fid, String, Tid, String);
    pub type TCol = (Uid, String);
    pub type FCol = (Uid, String, Kind);
}

pub mod import {
    pub use super::super::import::*;
    pub use super::prelude::*;
    pub use crate::db::export::*;
    pub use diesel::{
        prelude::*,
        query_builder::{BoxedSelectStatement},
        expression::{SqlLiteral, BoxableExpression},
        sqlite::{Sqlite},
        sql_types::{Bool, BigInt},
        dsl::*,
        query_source::joins::{JoinOn, Join, Inner},
    };
}

pub mod export {
    pub use super::prelude::*;
    pub use super::file::*;
    pub use super::tag::*;
    pub use super::file_tag::*;
}
pub use export::*;
