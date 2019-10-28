pub mod query;
pub mod update;
pub mod tag;

pub mod import {
    pub use super::super::import::*;
    pub use super::{query::export::*, api::{DatabaseLayer}};
    pub use crate::{model::export::*, db::export::*};
    pub use diesel::prelude::*;
}

pub mod export {
    pub use super::api::*;
    pub use super::query::*;
    pub use super::tag::*;
}
pub use export::*;

pub mod error {
    #[derive(Debug, Fail)]
    pub enum Error {
        // state machine errors
        #[fail(display = "wrong state for operation: '{}' -> '{}", operation, state)]
        WrongState { state: String, operation: String },

        // query errors
        #[fail(display = "unknown id: '{}'", id)]
        UnknownIntId { id: i64 },
        #[fail(display = "unknown id: '{}'", id)]
        UnknownStrId { id: String },
    }
}

pub mod api {
    use super::import::*;
    pub use crate::app::meta::{export::*, action, config::CommandAction};
    pub use super::{Query, update, query::{self, Pipeline, Forcings, collect}, tag};

    /// The database connection is the only
    /// persistent state need in our api struct.
    pub struct DatabaseLayer {
        pub connection: db::Connection,
    }

    impl DatabaseLayer {

        /// Create a new api instance
        pub fn new(connection: db::Connection) -> Self {
            Self { connection }
        }

        /// Update the database by scanning the given paths recursively
        pub fn update(&self, paths: &Vec<&str>) -> Res<()> {
            update::api::run(paths, &self.connection)
        }

        /// Enforce the configured database conventions
        pub fn enforce(&self, convention: &Vec<Convention>, commit: bool) -> Res<()> {
            for convention in convention.iter() {
                convention.enforce(&self, commit)?;
            }
            Ok(())
        }

        /// Run a query and map a single action over all files
        pub fn query_map(&self, pipeline: Pipeline, action: CommandAction, commit: bool) -> Res<()> {
            let command = Command::from_command_action(pipeline, &action);
            command.run(&self, commit)?;
            Ok(())
        }

        /// Since query results come in different shapes depending
        /// on the queries requirements (simple queries can skip
        /// mapping or filtering) we use Collectors to provide a
        /// simple interface for getting data out of the results.
        ///
        /// let files = api.query_collect(query, filter, query::collect::File)
        /// let count = api.query_collect(query, filter, query::collect::FileCount)
        pub fn query_collect<C>(&self, pipeline: &Pipeline, collector: impl collect::Collector<C>) -> Res<C> {
            Ok(collector.collect(self.query(pipeline, collector.forcings())?))
        }

        /// Create a new query, and run it
        pub fn query<'q>(&self, pipeline: &Pipeline, forcings: query::Forcings) -> Res<query::Results<'q>> {
            pipeline.forced_query(forcings, &self).execute()
        }

        /// Forget any unused tags
        pub fn clean(&self) -> Res<usize> {
            self.connection.get().transaction::<_, Error, _>(|| {
                let used_tids = file_tags::table.select(file_tags::tag_id).distinct();
                let deleted = diesel::delete(
                    tags::table.filter(tags::id.ne_all(used_tids))
                ).execute(self.connection.get())?;
                info!("DELETE: {} Tag(s)", deleted);
                Ok(deleted)
            })
        }

        /// Return a list of all tag names and ids
        pub fn query_tag_statistics(&self) -> Res<tag::Statistics> {
            tag::api::generate_tag_rankings(&self.connection)
        }

        /// Return a list of all tag names and ids
        pub fn query_all_tags(&self) -> Res<Vec<(i64, String)>> {
            Ok(tags::table.select((tags::id, tags::name)).get_results(self.connection.get())?)
        }

        /// Forget files by id
        pub fn forget(&self, files: &Vec<Fid>) -> Res<usize> {
            info!("DELETE: {} File(s)", files.len());
            Ok(File::delete_ids(files, &self.connection)?)
        }
    }
}
