use super::import::*;

/// Caches compiled regexes and comparison expressions
pub struct Context<'a> {
    connection: &'a db::Connection,
}

impl<'a> Context<'a> {

    /// Create a new Context instance
    pub fn new(connection: &'a db::Connection) -> Self {
        Self { connection }
    }

    /// Get a cached comparison
    pub fn connection(&'a self) -> &'a db::Connection {
        self.connection
    }
}
