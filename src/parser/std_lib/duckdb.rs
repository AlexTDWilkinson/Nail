use base64::Engine;
use dashmap::DashMap;
use duckdb::types::ValueRef;
use duckdb::Connection;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::sync::{Arc, Mutex};

// Store database connections by handle
lazy_static::lazy_static! {
    static ref CONNECTIONS: DashMap<String, Arc<Mutex<Connection>>> = DashMap::new();
}

/// Represents a DuckDB database connection
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DB_DuckDB {
    pub handle: String,
    pub path: String,
}

/// Represents the result of a DuckDB statement execution
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DB_DuckDB_Result {
    pub rows_affected: i64,
}

/// Look up the shared connection for a database handle
fn get_connection(handle: &str) -> Result<Arc<Mutex<Connection>>, String> {
    CONNECTIONS.get(handle).map(|entry| entry.value().clone()).ok_or_else(|| format!("DuckDB handle '{}' not found", handle))
}

/// Run blocking DuckDB work on the blocking thread pool so it doesn't
/// stall the async executor
async fn run_blocking<T, F>(work: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tokio::task::spawn_blocking(work).await.map_err(|e| format!("DuckDB task failed: {}", e))?
}

/// Open or create a DuckDB database file
pub async fn duckdb_open(path: String) -> Result<DB_DuckDB, String> {
    run_blocking(move || {
        let conn = Connection::open(&path).map_err(|e| format!("Failed to open DuckDB database '{}': {}", path, e))?;

        let handle = format!("duckdb_{}", uuid::Uuid::new_v4().to_string());

        CONNECTIONS.insert(handle.clone(), Arc::new(Mutex::new(conn)));

        Ok(DB_DuckDB { handle, path })
    })
    .await
}

/// Open an in-memory DuckDB database
pub async fn duckdb_memory() -> Result<DB_DuckDB, String> {
    run_blocking(move || {
        let conn = Connection::open_in_memory().map_err(|e| format!("Failed to open in-memory DuckDB database: {}", e))?;

        let handle = format!("duckdb_mem_{}", uuid::Uuid::new_v4().to_string());

        CONNECTIONS.insert(handle.clone(), Arc::new(Mutex::new(conn)));

        Ok(DB_DuckDB { handle, path: ":memory:".to_string() })
    })
    .await
}

/// Execute a SQL statement that doesn't return rows (CREATE, INSERT, UPDATE, DELETE, COPY)
pub async fn duckdb_execute(db: &DB_DuckDB, sql: String) -> Result<DB_DuckDB_Result, String> {
    let conn_arc = get_connection(&db.handle)?;

    run_blocking(move || {
        let conn = conn_arc.lock().map_err(|e| format!("Failed to lock DuckDB connection: {}", e))?;

        let affected = conn.execute(&sql, []).map_err(|e| format!("Failed to execute SQL: {}", e))?;

        Ok(DB_DuckDB_Result { rows_affected: affected as i64 })
    })
    .await
}

/// Convert a single DuckDB value to a JSON value so rows can be
/// deserialized into arbitrary user structs via serde.
fn value_ref_to_json(value: ValueRef) -> Result<serde_json::Value, String> {
    use serde_json::{Number, Value};
    Ok(match value {
        ValueRef::Null => Value::Null,
        ValueRef::Boolean(b) => Value::Bool(b),
        ValueRef::TinyInt(i) => Value::Number(Number::from(i)),
        ValueRef::SmallInt(i) => Value::Number(Number::from(i)),
        ValueRef::Int(i) => Value::Number(Number::from(i)),
        ValueRef::BigInt(i) => Value::Number(Number::from(i)),
        ValueRef::HugeInt(i) => {
            let as_i64 = i64::try_from(i).map_err(|_| format!("HUGEINT value {} does not fit in a 64-bit integer", i))?;
            Value::Number(Number::from(as_i64))
        }
        ValueRef::UTinyInt(i) => Value::Number(Number::from(i)),
        ValueRef::USmallInt(i) => Value::Number(Number::from(i)),
        ValueRef::UInt(i) => Value::Number(Number::from(i)),
        ValueRef::UBigInt(i) => Value::Number(Number::from(i)),
        ValueRef::Float(f) => Number::from_f64(f as f64).map(Value::Number).unwrap_or(Value::Null),
        ValueRef::Double(f) => Number::from_f64(f).map(Value::Number).unwrap_or(Value::Null),
        ValueRef::Text(bytes) => Value::String(String::from_utf8_lossy(bytes).to_string()),
        ValueRef::Blob(bytes) => Value::String(base64::engine::general_purpose::STANDARD.encode(bytes)),
        other => Value::String(format!("{:?}", other)),
    })
}

fn rows_to_json<'a>(mut rows: duckdb::Rows<'a>, column_names: Vec<String>) -> Result<Vec<serde_json::Value>, String> {
    let mut json_rows = Vec::new();

    while let Some(row) = rows.next().map_err(|e| format!("Failed to get row: {}", e))? {
        let mut object = serde_json::Map::new();
        for (index, name) in column_names.iter().enumerate() {
            let value = row.get_ref(index).map_err(|e| format!("Failed to read column '{}': {}", name, e))?;
            object.insert(name.clone(), value_ref_to_json(value)?);
        }
        json_rows.push(serde_json::Value::Object(object));
    }

    Ok(json_rows)
}

/// Execute a SQL query and return results as a vector of typed structs
pub async fn duckdb_query<T>(db: &DB_DuckDB, sql: String) -> Result<Vec<T>, String>
where
    T: DeserializeOwned + Send + 'static,
{
    let conn_arc = get_connection(&db.handle)?;

    run_blocking(move || {
        let conn = conn_arc.lock().map_err(|e| format!("Failed to lock DuckDB connection: {}", e))?;

        let mut stmt = conn.prepare(&sql).map_err(|e| format!("Failed to prepare SQL statement: {}", e))?;

        let rows = stmt.query([]).map_err(|e| format!("Failed to execute query: {}", e))?;
        let column_names: Vec<String> = rows.as_ref().map(|statement| statement.column_names()).unwrap_or_default();

        let json_rows = rows_to_json(rows, column_names)?;

        json_rows.into_iter().map(|row| serde_json::from_value::<T>(row).map_err(|e| format!("Failed to deserialize row: {}", e))).collect()
    })
    .await
}

/// Execute a SQL query and return the first result as a typed struct
pub async fn duckdb_query_single<T>(db: &DB_DuckDB, sql: String) -> Result<T, String>
where
    T: DeserializeOwned + Send + 'static,
{
    let results: Vec<T> = duckdb_query(db, sql).await?;

    results.into_iter().next().ok_or_else(|| "No results found".to_string())
}

/// Close a DuckDB connection
pub async fn duckdb_close(db: &DB_DuckDB) -> Result<(), String> {
    let handle = db.handle.clone();

    run_blocking(move || {
        // Dropping the connection closes the database, which can block
        CONNECTIONS.remove(&handle).ok_or_else(|| format!("DuckDB handle '{}' not found", handle))?;

        Ok(())
    })
    .await
}

/// Execute multiple SQL statements in a single transaction
/// All statements succeed or all fail atomically
pub async fn duckdb_execute_batch(db: &DB_DuckDB, statements: Vec<String>) -> Result<DB_DuckDB_Result, String> {
    let conn_arc = get_connection(&db.handle)?;

    run_blocking(move || {
        let conn = conn_arc.lock().map_err(|e| format!("Failed to lock DuckDB connection: {}", e))?;

        conn.execute("BEGIN TRANSACTION", []).map_err(|e| format!("Failed to begin transaction: {}", e))?;

        let mut total_affected = 0i64;

        for sql in statements {
            match conn.execute(&sql, []) {
                Ok(affected) => {
                    total_affected += affected as i64;
                }
                Err(e) => {
                    let _ = conn.execute("ROLLBACK", []);
                    return Err(format!("Failed to execute SQL '{}': {}", sql, e));
                }
            }
        }

        conn.execute("COMMIT", []).map_err(|e| {
            let _ = conn.execute("ROLLBACK", []);
            format!("Failed to commit transaction: {}", e)
        })?;

        Ok(DB_DuckDB_Result { rows_affected: total_affected })
    })
    .await
}
