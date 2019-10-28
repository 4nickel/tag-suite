use super::{import::*, error::Error};
use diesel::prelude::{RunQueryDsl, SqliteConnection};
use diesel::r2d2::{
    CustomizeConnection,
    ConnectionManager,
    Pool,
    PooledConnection
};

/// A connection Pool managing SqliteConnections
pub type SqlitePool = Pool<ConnectionManager<SqliteConnection>>;

/// A connection customizer which enables on foreign key support
#[derive(Debug)]
struct ConnectionCustomizer ();
impl<C: diesel::Connection, E> CustomizeConnection<C, E> for ConnectionCustomizer
{
    fn on_acquire(&self, connection: &mut C) -> Result<(), E> {
        // FIXME: I cannot for the life of me figure
        // out how to return a proper error here.
        diesel::dsl::sql_query(format!("PRAGMA foreign_keys = ON"))
            .execute(connection)
            .expect("pragma error: failed to enable foreign key support");
        Ok(())
    }
}

pub struct Connection(pub PooledConnection<ConnectionManager<SqliteConnection>>);

impl Connection {

    /// Create a new connection pool.
    pub fn new_pool(database_url: &str, max_size: u32) -> Res<SqlitePool> {
        let manager = ConnectionManager::<SqliteConnection>::new(database_url);
        Pool::builder()
            .max_size(max_size)
            .connection_customizer(box ConnectionCustomizer { })
            .build(manager)
            .map_err(|e| Error::ConnectionPoolError { message: format!("{:?}", e) }.into())
    }

    /// Return the underlying connection.
    #[inline(always)]
    pub fn get(&self) -> &SqliteConnection {
        &self.0
    }
}
