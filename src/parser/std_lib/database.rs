use dashmap::DashMap;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use serde_rusqlite::{from_row, from_rows};

// Store database connections by handle
lazy_static::lazy_static! {
    static ref CONNECTIONS: DashMap<String, Arc<Mutex<Connection>>> = DashMap::new();
}

/// Represents a SQLite database connection
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DB_SQLite {
    pub handle: String,
    pub path: String,
}

/// Represents the result of a database operation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DB_Result {
    pub rows_affected: i64,
    pub last_insert_id: i64,
}

/// Look up the shared connection for a database handle
fn get_connection(handle: &str) -> Result<Arc<Mutex<Connection>>, String> {
    CONNECTIONS
        .get(handle)
        .map(|entry| entry.value().clone())
        .ok_or_else(|| format!("db_sqlite: unknown database handle '{}' (was the connection already closed?)", handle))
}

/// Run blocking SQLite work on the blocking thread pool so it doesn't
/// stall the async executor
async fn run_blocking<T, F>(work: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|e| format!("db_sqlite: background database task failed: {}", e))?
}

/// Open or create a SQLite database
pub async fn sqlite_open(path: String) -> Result<DB_SQLite, String> {
    run_blocking(move || {
        let conn = Connection::open(&path)
            .map_err(|e| format!("db_sqlite_open: could not open database '{}': {}", path, e))?;

        // Generate a unique handle for this connection
        let handle = format!("db_{}", uuid::Uuid::new_v4().to_string());

        CONNECTIONS.insert(handle.clone(), Arc::new(Mutex::new(conn)));

        Ok(DB_SQLite {
            handle,
            path,
        })
    }).await
}

/// Open an in-memory SQLite database
pub async fn sqlite_memory() -> Result<DB_SQLite, String> {
    run_blocking(move || {
        let conn = Connection::open_in_memory()
            .map_err(|e| format!("db_sqlite_memory: could not open an in-memory database: {}", e))?;

        let handle = format!("db_mem_{}", uuid::Uuid::new_v4().to_string());

        CONNECTIONS.insert(handle.clone(), Arc::new(Mutex::new(conn)));

        Ok(DB_SQLite {
            handle,
            path: ":memory:".to_string(),
        })
    }).await
}

/// The parameterised variants below take `?` placeholders and a list of
/// values, so a value never becomes part of the SQL text. That is the only way
/// to put untrusted input in a statement: quoting it by hand works right up
/// until the day a value is spliced somewhere a quote was not expected, and
/// then it is not a bug, it is somebody else's query. SQLite binds every value
/// as text and applies the column's affinity, so a number kept in a string
/// still compares and stores as a number.
///
/// Execute a SQL statement with bound parameters
pub async fn sqlite_execute_params(db: &DB_SQLite, sql: String, params: Vec<String>) -> Result<DB_Result, String> {
    let conn_arc = get_connection(&db.handle)?;

    run_blocking(move || {
        let conn = conn_arc.lock().map_err(|e| format!("db_sqlite_execute_params: could not lock the database connection: {}", e))?;

        let affected = conn
            .execute(&sql, rusqlite::params_from_iter(params.iter()))
            .map_err(|e| format!("db_sqlite_execute_params: failed to execute SQL '{}': {}", sql, e))?;

        let last_insert_id = conn.last_insert_rowid();

        Ok(DB_Result { rows_affected: affected as i64, last_insert_id })
    })
    .await
}

/// Query with bound parameters, returning every row as a typed struct
pub async fn sqlite_query_params<T>(db: &DB_SQLite, sql: String, params: Vec<String>) -> Result<Vec<T>, String>
where
    T: DeserializeOwned + Send + 'static,
{
    let conn_arc = get_connection(&db.handle)?;

    run_blocking(move || {
        let conn = conn_arc.lock().map_err(|e| format!("db_sqlite_query_params: could not lock the database connection: {}", e))?;

        let mut stmt = conn.prepare(&sql).map_err(|e| format!("db_sqlite_query_params: could not prepare SQL '{}': {}", sql, e))?;

        let rows = from_rows::<T>(
            stmt.query(rusqlite::params_from_iter(params.iter()))
                .map_err(|e| format!("db_sqlite_query_params: failed to execute SQL '{}': {}", sql, e))?,
        )
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("db_sqlite_query_params: could not deserialize the result rows into the target struct: {}", e))?;

        Ok(rows)
    })
    .await
}

/// Query with bound parameters, returning the first row as a typed struct
pub async fn sqlite_query_single_params<T>(db: &DB_SQLite, sql: String, params: Vec<String>) -> Result<T, String>
where
    T: DeserializeOwned + Send + 'static,
{
    let conn_arc = get_connection(&db.handle)?;

    run_blocking(move || {
        let conn = conn_arc.lock().map_err(|e| format!("db_sqlite_query_single_params: could not lock the database connection: {}", e))?;

        let mut stmt = conn.prepare(&sql).map_err(|e| format!("db_sqlite_query_single_params: could not prepare SQL '{}': {}", sql, e))?;

        let mut rows = stmt
            .query(rusqlite::params_from_iter(params.iter()))
            .map_err(|e| format!("db_sqlite_query_single_params: failed to execute SQL '{}': {}", sql, e))?;

        match rows.next().map_err(|e| format!("db_sqlite_query_single_params: could not read the result row: {}", e))? {
            Some(row) => from_row::<T>(row).map_err(|e| format!("db_sqlite_query_single_params: could not deserialize the result row into the target struct: {}", e)),
            None => Err(format!("db_sqlite_query_single_params: the query '{}' returned no rows", sql)),
        }
    })
    .await
}

/// Execute a SQL statement that doesn't return rows (CREATE, INSERT, UPDATE, DELETE)
pub async fn sqlite_execute(db: &DB_SQLite, sql: String) -> Result<DB_Result, String> {
    let conn_arc = get_connection(&db.handle)?;

    run_blocking(move || {
        let conn = conn_arc.lock()
            .map_err(|e| format!("db_sqlite_execute: could not lock the database connection: {}", e))?;

        let affected = conn.execute(&sql, [])
            .map_err(|e| format!("db_sqlite_execute: failed to execute SQL '{}': {}", sql, e))?;

        // Try to get last insert rowid
        let last_insert_id = conn.last_insert_rowid();

        Ok(DB_Result {
            rows_affected: affected as i64,
            last_insert_id,
        })
    }).await
}

/// Execute a SQL query and return results as a vector of typed structs
pub async fn sqlite_query<T>(db: &DB_SQLite, sql: String) -> Result<Vec<T>, String>
where
    T: DeserializeOwned + Send + 'static,
{
    let conn_arc = get_connection(&db.handle)?;

    run_blocking(move || {
        let conn = conn_arc.lock()
            .map_err(|e| format!("db_sqlite_query: could not lock the database connection: {}", e))?;

        let mut stmt = conn.prepare(&sql)
            .map_err(|e| format!("db_sqlite_query: could not prepare SQL '{}': {}", sql, e))?;

        let rows = from_rows::<T>(stmt.query([]).map_err(|e| format!("db_sqlite_query: failed to execute SQL '{}': {}", sql, e))?)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("db_sqlite_query: could not deserialize the result rows into the target struct: {}", e))?;

        Ok(rows)
    }).await
}

/// Execute a SQL query and return the first result as a typed struct
pub async fn sqlite_query_single<T>(db: &DB_SQLite, sql: String) -> Result<T, String>
where
    T: DeserializeOwned + Send + 'static,
{
    let conn_arc = get_connection(&db.handle)?;

    run_blocking(move || {
        let conn = conn_arc.lock()
            .map_err(|e| format!("db_sqlite_query_single: could not lock the database connection: {}", e))?;

        let mut stmt = conn.prepare(&sql)
            .map_err(|e| format!("db_sqlite_query_single: could not prepare SQL '{}': {}", sql, e))?;

        let mut rows = stmt.query([])
            .map_err(|e| format!("db_sqlite_query_single: failed to execute SQL '{}': {}", sql, e))?;

        match rows.next().map_err(|e| format!("db_sqlite_query_single: could not read the result row: {}", e))? {
            Some(row) => from_row::<T>(row)
                .map_err(|e| format!("db_sqlite_query_single: could not deserialize the result row into the target struct: {}", e)),
            None => Err(format!("db_sqlite_query_single: the query '{}' returned no rows", sql)),
        }
    }).await
}

/// Close a database connection
pub async fn sqlite_close(db: &DB_SQLite) -> Result<(), String> {
    let handle = db.handle.clone();

    run_blocking(move || {
        // Dropping the connection closes the database, which can block
        CONNECTIONS.remove(&handle)
            .ok_or_else(|| format!("db_sqlite_close: unknown database handle '{}' (was the connection already closed?)", handle))?;

        Ok(())
    }).await
}

/// Begin a transaction
pub async fn sqlite_begin(db: &DB_SQLite) -> Result<(), String> {
    let result = sqlite_execute(db, "BEGIN TRANSACTION".to_string()).await?;
    Ok(())
}

/// Commit a transaction
pub async fn sqlite_commit(db: &DB_SQLite) -> Result<(), String> {
    let result = sqlite_execute(db, "COMMIT".to_string()).await?;
    Ok(())
}

/// Rollback a transaction
pub async fn sqlite_rollback(db: &DB_SQLite) -> Result<(), String> {
    let result = sqlite_execute(db, "ROLLBACK".to_string()).await?;
    Ok(())
}

/// Execute multiple SQL statements in a single transaction
/// All statements succeed or all fail atomically
pub async fn sqlite_execute_batch(db: &DB_SQLite, statements: Vec<String>) -> Result<DB_Result, String> {
    let conn_arc = get_connection(&db.handle)?;

    run_blocking(move || {
        let conn = conn_arc.lock()
            .map_err(|e| format!("db_sqlite_execute_batch: could not lock the database connection: {}", e))?;

        // Start transaction
        conn.execute("BEGIN TRANSACTION", [])
            .map_err(|e| format!("db_sqlite_execute_batch: could not begin the transaction: {}", e))?;

        let mut total_affected = 0i64;
        let mut last_id = 0i64;

        // Execute all statements
        for sql in statements {
            match conn.execute(&sql, []) {
                Ok(affected) => {
                    total_affected += affected as i64;
                    last_id = conn.last_insert_rowid();
                },
                Err(e) => {
                    // Rollback on any error
                    let _ = conn.execute("ROLLBACK", []);
                    return Err(format!("db_sqlite_execute_batch: failed to execute SQL '{}': {} (the transaction was rolled back)", sql, e));
                }
            }
        }

        // Commit if all succeeded
        conn.execute("COMMIT", [])
            .map_err(|e| {
                let _ = conn.execute("ROLLBACK", []);
                format!("db_sqlite_execute_batch: could not commit the transaction: {} (the transaction was rolled back)", e)
            })?;

        Ok(DB_Result {
            rows_affected: total_affected,
            last_insert_id: last_id,
        })
    }).await
}
