pub mod collect;
pub use collect::*;
pub mod maps;
pub use maps::*;
pub mod iter;
pub use iter::*;
pub mod state;
pub use state::*;
pub mod filter;
pub use filter::*;

pub use super::error;

pub mod prelude {
    /// Our built-in Output options
    /// TODO: add custom formatting
    pub enum Output {
        FileCount,
        FilePaths,
        TagCount,
        TagNames,
        Serialize,
    }
    /// Forcings are used to indicate to the
    /// query engine what kind of output data
    /// we require.
    pub enum Forcing {
        Mapped = 0,
        Filtered = 1,
        Piped = 2,
    }
    #[derive(Debug, Copy, Clone)]
    pub struct Forcings(u8);
    impl Forcings {
        // TODO: use some crate for this?
        #[inline(always)] pub fn combine(self, other: Forcings) -> Self { Forcings(self.0 | other.0) }
        #[inline(always)] pub fn new()                          -> Self { Forcings(0u8) }
        #[inline(always)] pub fn mapped(self)                   -> Self { Forcings(self.0 | (1u8 << Forcing::Mapped as u8)) }
        #[inline(always)] pub fn filtered(self)                 -> Self { Forcings(self.mapped().0 | (1u8 << Forcing::Filtered as u8)) }
        #[inline(always)] pub fn piped(self)                    -> Self { Forcings(self.mapped().0 | (1u8 << Forcing::Piped as u8)) }
        #[inline(always)] pub fn has_none(&self)                -> bool { (self.0 == 0u8) }
        #[inline(always)] pub fn has_mapped(&self)              -> bool { (self.0 & (1u8 << Forcing::Mapped as u8)) != 0u8}
        #[inline(always)] pub fn has_filtered(&self)            -> bool { (self.0 & (1u8 << Forcing::Filtered as u8)) != 0u8}
        #[inline(always)] pub fn has_piped(&self)               -> bool { (self.0 & (1u8 << Forcing::Piped as u8)) != 0u8}
    }
}

/// A single row as returned by our queries
pub mod import {
    pub use super::super::import::*;
    pub use super::prelude::*;
    pub use crate::{
        db::export::*,
        model::prelude::*,
        expression::{Expression as CompiledExpression, Expansions},
    };
}

pub mod export {
    pub use super::{state::*, iter::*, maps::*};
    pub use super::prelude::*;
}
pub use export::*;

pub mod api {

    use super::{import::*};
    use crate::{dsl, db::wrangle::*, model::{tag, file_tag, prelude::Ids}};

    /// When the user queries all files doing so directly is
    /// more efficient
    pub fn query_all(c: &db::Connection) -> Res<Vec<Ids>> {
        let rows: Vec<Ids> = profile!("all", {
            file_tags::table
                .select(file_tag::IDS)
                .get_results(c.get())?
        });
        Ok(rows)
    }

    /// The core query functionality: builds an aggregate
    /// SQL command from the compiled Expression and runs
    /// the query.
    use std::cell::RefCell;
    pub fn combinator_dsl(exp: &CompiledExpression, c: &db::Connection) -> Res<Vec<Ids>> {
        let (dsl, context) = (dsl::combinator::Dsl::new(), RefCell::new(()));
        profile!("query dsl", { c.get().transaction::<_, Error, _>(|| {
            let expression = profile!("evaluate", {
                dsl.evaluate(exp.as_ast(), &context, &())
            })?;
            let fids: Vec<Fid> = profile!("subselect", {
                file_tags::table
                    .filter(&expression)
                    .select(file_tags::file_id)
                    .get_results(c.get())?
            });
            let rows: Vec<Ids> = profile!("final", {
                file_tags::table
                    .select(file_tag::IDS)
                    .filter(file_tags::file_id.eq_any(fids))
                    .get_results(c.get())?
            });
            Ok(rows)
        }) })
    }

    /// The core query functionality: builds an aggregate
    /// SQL command from the compiled Expression and runs
    /// the query.
    pub fn query_dsl(exp: &CompiledExpression, c: &db::Connection) -> Res<Vec<Ids>> {
        let (dsl, context) = (dsl::query::Dsl::new(), RefCell::new(dsl::query::Context::new(c)));
        profile!("query dsl", { c.get().transaction::<_, Error, _>(|| {
            let expression = profile!("evaluate", {
                dsl.evaluate(exp.as_ast(), &context, &())?
            });
            let fids: Vec<Fid> = profile!("subselect", {
                files::table
                    .filter(&expression)
                    .select(files::id)
                    .get_results(c.get())?
            });
            let rows: Vec<Ids> = profile!("final", {
                file_tags::table
                    .select(file_tag::IDS)
                    .filter(file_tags::file_id.eq_any(fids))
                    .get_results(c.get())?
            });
            Ok(rows)
        })})
    }

    pub fn query_columns<'a, T>(fids: T, tids: T, c: &db::Connection) -> Res<(Vec<FCol>, Vec<TCol>)>
    where
        T: AsInExpression<BigInt> + Iterator<Item=&'a i64> + ExactSizeIterator,
        <T as AsInExpression<BigInt>>::InExpression: QueryFragment<Sqlite>,
        <T as AsInExpression<BigInt>>::InExpression: QueryId,
        <T as AsInExpression<BigInt>>::InExpression: AppearsOnTable<tags::table>,
        <T as AsInExpression<BigInt>>::InExpression: AppearsOnTable<files::table>,
    {
        let flen = fids.len();
        let tlen = tids.len();
        let fcol: Vec<FCol> = profile!("files", {
            info!("naming {} Files", flen);
            files::table
                .select((files::id, files::path, files::kind))
                .filter(files::id.eq_any(fids))
                .get_results(c.get())?
        });
        let tcol: Vec<TCol> = profile!("tags", {
            info!("naming {} Tags", tlen);
            tags::table
                .select(tag::IDS)
                .filter(tags::id.eq_any(tids))
                .get_results(c.get())?
        });
        assert_eq!(fcol.len(), flen,
            "bug: inconsistent query results");
        assert_eq!(tcol.len(), tlen,
            "bug: inconsistent query results");
        Ok((fcol, tcol))
    }

    /// Takes raw data returned by the query dsl and
    /// queries additional column data.
    /// Does not generate Associations data.
    pub fn query_unassociated(map: Vec<Ids>, c: &db::Connection) -> Res<Columns> {
        info!("recieved {} Rows..", map.len());
        let (fids, tids) = profile!("recv", {
            map.iter().fold((HashSet::new(), HashSet::new()),
                |(mut fids, mut tids), (fid, tid)| {
                    fids.insert(*fid);
                    tids.insert(*tid);
                    (fids, tids)
                })
        });
        let (fcol, tcol) = query_columns(fids.iter(), tids.iter(), c)?;
        Ok(Columns::from_cols(fcol, tcol, map))
    }

    /// Takes raw data returned by the query dsl and
    /// queries additional column data.
    /// Since we're iterating anyway, we can gerate
    /// the maps that define File-to-Tag associations
    /// as we go, too.
    pub fn query_associated(map: Vec<Ids>, c: &db::Connection) -> Res<(Columns, ManyToManyIds)> {
        info!("recieved {} Rows..", map.len());
        let (fids, tids, mtom) = profile!("recv", {
            map.iter().fold((Vec::new(), Vec::new(), ManyToManyIds::new()),
                |(mut fids, mut tids, mut mtom), (fid, tid)| {
                    let (f, t) = mtom.map(*fid, *tid);
                    if f { fids.push(*fid); }
                    if t { tids.push(*tid); }
                    (fids, tids, mtom)
                })
        });
        let (fcol, tcol) = query_columns(fids.iter(), tids.iter(), c)?;
        Ok((Columns::from_cols(fcol, tcol, map), mtom))
    }

    /// The core query functionality: builds an aggregate
    /// SQL command from the compiled Expression and runs
    /// the query.
    pub fn query_tags_by_file_ids(ids: &Vec<Fid>, c: &db::Connection) -> Res<Vec<Tid>> {
        let tids: Vec<Tid> =
            file_tags::table
                .select(file_tags::tag_id)
                .filter(file_tags::file_id.eq_any(ids))
                .get_results(c.get())?;
        Ok(tids)
    }

    /// The core filter functionality: filters the queried
    /// data by matching each file against our custom dsl and
    /// returns a list of collected ids
    pub fn filter_dsl<'a>(data: &'a Maps, filter: &CompiledExpression) -> Res<Vec<Fid>> {
        let context = RefCell::new(dsl::filter::Context::new());
        let dsl = dsl::filter::Dsl::new();
        data.into_iter()
            .try_fold(Vec::new(), |mut accu, file| {
                if dsl.evaluate(filter.as_ast(), &context, &file)? {
                    accu.push(file.id())
                }
                Ok(accu)
            })
    }
}
