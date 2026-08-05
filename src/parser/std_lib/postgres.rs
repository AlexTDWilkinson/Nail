//! Postgres, for when SQLite is not where the data lives.
//!
//! SQLite is a file, which is exactly right until a second copy of the program
//! needs the same data - a deploy that runs two instances, a worker beside a web
//! server, anything on more than one machine. Then the database has to be a
//! server, and on the open internet that server is Postgres.
//!
//! The shape is the same as the SQLite module on purpose: a connection is a
//! handle, statements bind their values rather than being pasted together, and
//! a query returns rows as whatever struct the assignment asks for. A program
//! moving from one to the other changes the connecting and the placeholders and
//! nothing else.
//!
//! Placeholders are Postgres's own `$1`, `$2` rather than SQLite's `?`, because
//! rewriting somebody's SQL to hide the difference is the sort of cleverness
//! that eventually rewrites something inside a string literal.
//!
//! Rows come back as JSON because Postgres will do that conversion itself -
//! `row_to_json` knows about every type in the database, including the ones a
//! Rust driver would have to be taught one at a time.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tokio_postgres::{Client, NoTls};

lazy_static::lazy_static! {
    /// Every open connection, by handle. Held here rather than in the Nail
    /// program because a connection is not a value Nail can hold - it has a
    /// socket in it.
    static ref CONNECTIONS: dashmap::DashMap<String, std::sync::Arc<Client>> = dashmap::DashMap::new();
}

/// A connection to a Postgres database.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DB_Postgres {
    pub handle: String,
    pub database: String,
}

/// What a statement did.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DB_PostgresResult {
    pub rows_affected: i64,
}

fn get_client(handle: &str) -> Result<std::sync::Arc<Client>, String> {
    return CONNECTIONS
        .get(handle)
        .map(|entry| entry.value().clone())
        .ok_or_else(|| format!("db_postgres: unknown database handle '{}' (was the connection already closed?)", handle));
}

/// The database name out of a connection string, for saying which database an
/// error was about. Best effort - if the string is not shaped as expected, the
/// whole thing is not worth printing, since it has the password in it.
fn database_name(url: &str) -> String {
    let after_host = url.rsplit('/').next().unwrap_or("");
    let name = after_host.split('?').next().unwrap_or("");
    if name.is_empty() {
        return "postgres".to_string();
    }
    return name.to_string();
}

/// Connects to a Postgres server. The connection string is the usual
/// `postgres://user:password@host:5432/database`.
///
/// TLS is not set up here, which means this connects in the clear - correct for
/// a database on the same machine or over a private network, and not acceptable
/// across the internet. Use a connection over localhost, whether that is the
/// database itself or a tunnel to it.
pub async fn connect(url: String) -> Result<DB_Postgres, String> {
    let database = database_name(&url);
    let (client, connection) = tokio_postgres::connect(&url, NoTls).await.map_err(|failure| format!("db_postgres_connect: could not connect to database '{}': {}", database, failure))?;

    // The driver splits into a client and a connection task; nothing happens on
    // the client until the task is being polled, so it is spawned here and lives
    // as long as the connection does.
    tokio::spawn(async move {
        // A dropped connection is not something a Nail program can act on - the
        // next statement on it will say so, with the SQL that failed.
        let _ = connection.await;
    });

    let handle = format!("pg_{}", uuid::Uuid::new_v4());
    CONNECTIONS.insert(handle.clone(), std::sync::Arc::new(client));
    return Ok(DB_Postgres { handle, database });
}

/// Closes a connection and forgets its handle. Statements on it after this are
/// an error naming the handle rather than a hang.
pub async fn close(db: &DB_Postgres) -> Result<(), String> {
    match CONNECTIONS.remove(&db.handle) {
        Some(_) => return Ok(()),
        None => return Err(format!("db_postgres_close: unknown database handle '{}' (was it already closed?)", db.handle)),
    }
}

/// Runs a statement that changes data and returns how many rows it changed.
///
/// The values are bound to `$1`, `$2` and so on rather than being put into the
/// SQL text, which is the only way to put input from outside the program into a
/// statement. Every value is sent as text and Postgres casts it to the column's
/// type, so a number in a string still stores as a number - but a comparison
/// against a non-text column may need an explicit cast, `$1::int`.
pub async fn execute(db: &DB_Postgres, sql: String, params: Vec<String>) -> Result<DB_PostgresResult, String> {
    let client = get_client(&db.handle)?;
    let bound: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params.iter().map(|value| value as &(dyn tokio_postgres::types::ToSql + Sync)).collect();
    let affected = client.execute(&sql, &bound).await.map_err(|failure| format!("db_postgres_execute: failed to execute SQL '{}': {}", sql, failure))?;
    return Ok(DB_PostgresResult { rows_affected: affected as i64 });
}

/// Runs several statements in one round trip, for a schema a program creates on
/// startup. No values are bound, so nothing from outside the program belongs in
/// the text - use `db_postgres_execute` for that.
pub async fn execute_batch(db: &DB_Postgres, statements: String) -> Result<(), String> {
    let client = get_client(&db.handle)?;
    client.batch_execute(&statements).await.map_err(|failure| format!("db_postgres_execute_batch: failed to execute the statements: {}", failure))?;
    return Ok(());
}

/// Wraps a query so Postgres returns each row as JSON text. Postgres knows how
/// to render every type it stores; teaching a Rust driver the same list one type
/// at a time is how a database layer grows a thousand lines.
fn as_json_rows(sql: &str) -> String {
    return format!("SELECT row_to_json(nail_row)::text AS nail_json FROM ({}) AS nail_row", sql.trim().trim_end_matches(';'));
}

/// Every row of a query, as whatever struct the assignment asks for.
pub async fn query<T>(db: &DB_Postgres, sql: String, params: Vec<String>) -> Result<Vec<T>, String>
where
    T: DeserializeOwned,
{
    let client = get_client(&db.handle)?;
    let bound: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params.iter().map(|value| value as &(dyn tokio_postgres::types::ToSql + Sync)).collect();
    let rows = client.query(&as_json_rows(&sql), &bound).await.map_err(|failure| format!("db_postgres_query: failed to run SQL '{}': {}", sql, failure))?;

    let mut found = Vec::with_capacity(rows.len());
    for row in rows.iter() {
        let json: String = row.try_get("nail_json").map_err(|failure| format!("db_postgres_query: could not read a row of '{}': {}", sql, failure))?;
        found.push(serde_json::from_str::<T>(&json).map_err(|failure| format!("db_postgres_query: a row of '{}' does not match the type it was read into: {}", sql, failure))?);
    }
    return Ok(found);
}

/// The one row a query returns. More than one row, or none, is an error - which
/// is what makes this the right function for a lookup by primary key and the
/// wrong one for a search.
pub async fn query_single<T>(db: &DB_Postgres, sql: String, params: Vec<String>) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let mut rows = query::<T>(db, sql.clone(), params).await.map_err(|detail| detail.replace("db_postgres_query", "db_postgres_query_single"))?;
    if rows.len() > 1 {
        return Err(format!("db_postgres_query_single: '{}' returned {} rows, and this reads exactly one", sql, rows.len()));
    }
    return match rows.pop() {
        Some(row) => Ok(row),
        None => Err(format!("db_postgres_query_single: '{}' returned no rows", sql)),
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The database name is only for error messages, and must never bring the
    /// password along with it.
    #[test]
    fn the_database_name_is_taken_out_of_a_connection_string() {
        assert_eq!(database_name("postgres://user:secret@localhost:5432/orders"), "orders");
        assert_eq!(database_name("postgres://user:secret@localhost:5432/orders?sslmode=require"), "orders");
        assert_eq!(database_name("postgres://localhost/"), "postgres");
        assert_eq!(database_name("nonsense"), "nonsense");
        assert!(!database_name("postgres://user:secret@localhost:5432/orders").contains("secret"));
    }

    #[test]
    fn a_query_is_wrapped_so_postgres_renders_the_rows() {
        let wrapped = as_json_rows("SELECT id, name FROM people WHERE id = $1");
        assert!(wrapped.starts_with("SELECT row_to_json(nail_row)"), "got: {}", wrapped);
        assert!(wrapped.contains("SELECT id, name FROM people WHERE id = $1"), "got: {}", wrapped);
        // The placeholders are left exactly as written, so the binding still
        // lines up with the values.
        assert!(wrapped.contains("$1"));
    }

    /// A trailing semicolon would end the statement in the middle of the wrapper.
    #[test]
    fn a_trailing_semicolon_does_not_break_the_wrapper() {
        let wrapped = as_json_rows("SELECT 1;  ");
        assert!(!wrapped.contains(";"), "got: {}", wrapped);
        assert!(wrapped.ends_with("AS nail_row"), "got: {}", wrapped);
    }

    #[tokio::test]
    async fn a_handle_that_was_never_opened_says_so() {
        let never_opened = DB_Postgres { handle: "pg_nothing".to_string(), database: "orders".to_string() };
        let failure = execute(&never_opened, "SELECT 1".to_string(), vec![]).await.unwrap_err();
        assert!(failure.contains("unknown database handle"), "got: {}", failure);
        assert!(close(&never_opened).await.is_err());
    }

    /// Nothing here reaches a server, so the failure is about connecting rather
    /// than about the statement - and it names the database, not the password.
    #[tokio::test]
    async fn connecting_to_a_server_that_is_not_there_says_so() {
        let failure = connect("postgres://nail:nail@127.0.0.1:1/orders".to_string()).await.unwrap_err();
        assert!(failure.contains("db_postgres_connect"), "got: {}", failure);
        assert!(failure.contains("orders"), "got: {}", failure);
        assert!(!failure.contains("nail:nail"), "the connection string must not be echoed: {}", failure);
    }
}
