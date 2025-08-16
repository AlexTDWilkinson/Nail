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

/// Open or create a SQLite database
pub async fn sqlite_open(path: String) -> Result<DB_SQLite, String> {
    let conn = Connection::open(&path)
        .map_err(|e| format!("Failed to open database '{}': {}", path, e))?;
    
    // Generate a unique handle for this connection
    let handle = format!("db_{}", uuid::Uuid::new_v4().to_string());
    
    CONNECTIONS.insert(handle.clone(), Arc::new(Mutex::new(conn)));
    
    Ok(DB_SQLite {
        handle,
        path,
    })
}

/// Open an in-memory SQLite database
pub async fn sqlite_memory() -> Result<DB_SQLite, String> {
    let conn = Connection::open_in_memory()
        .map_err(|e| format!("Failed to open in-memory database: {}", e))?;
    
    let handle = format!("db_mem_{}", uuid::Uuid::new_v4().to_string());
    
    CONNECTIONS.insert(handle.clone(), Arc::new(Mutex::new(conn)));
    
    Ok(DB_SQLite {
        handle,
        path: ":memory:".to_string(),
    })
}

/// Execute a SQL statement that doesn't return rows (CREATE, INSERT, UPDATE, DELETE)
pub async fn sqlite_execute(db: &DB_SQLite, sql: String) -> Result<DB_Result, String> {
    let conn_arc = CONNECTIONS.get(&db.handle)
        .ok_or_else(|| format!("Database handle '{}' not found", db.handle))?;
    
    let conn = conn_arc.lock()
        .map_err(|e| format!("Failed to lock database connection: {}", e))?;
    
    let affected = conn.execute(&sql, [])
        .map_err(|e| format!("Failed to execute SQL: {}", e))?;
    
    // Try to get last insert rowid
    let last_insert_id = conn.last_insert_rowid();
    
    Ok(DB_Result {
        rows_affected: affected as i64,
        last_insert_id,
    })
}

/// Execute a SQL query and return results as a vector of typed structs
pub async fn sqlite_query<T>(db: &DB_SQLite, sql: String) -> Result<Vec<T>, String> 
where
    T: DeserializeOwned,
{
    let conn_arc = CONNECTIONS.get(&db.handle)
        .ok_or_else(|| format!("Database handle '{}' not found", db.handle))?;
    
    let conn = conn_arc.lock()
        .map_err(|e| format!("Failed to lock database connection: {}", e))?;
    
    let mut stmt = conn.prepare(&sql)
        .map_err(|e| format!("Failed to prepare SQL statement: {}", e))?;
    
    let rows = from_rows::<T>(stmt.query([]).map_err(|e| format!("Failed to execute query: {}", e))?)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to deserialize rows: {}", e))?;
    
    Ok(rows)
}

/// Execute a SQL query and return the first result as a typed struct
pub async fn sqlite_query_single<T>(db: &DB_SQLite, sql: String) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let conn_arc = CONNECTIONS.get(&db.handle)
        .ok_or_else(|| format!("Database handle '{}' not found", db.handle))?;
    
    let conn = conn_arc.lock()
        .map_err(|e| format!("Failed to lock database connection: {}", e))?;
    
    let mut stmt = conn.prepare(&sql)
        .map_err(|e| format!("Failed to prepare SQL statement: {}", e))?;
    
    let mut rows = stmt.query([])
        .map_err(|e| format!("Failed to execute query: {}", e))?;
    
    match rows.next().map_err(|e| format!("Failed to get row: {}", e))? {
        Some(row) => from_row::<T>(row)
            .map_err(|e| format!("Failed to deserialize row: {}", e)),
        None => Err("No results found".to_string()),
    }
}

/// Close a database connection
pub async fn sqlite_close(db: &DB_SQLite) -> Result<(), String> {
    CONNECTIONS.remove(&db.handle)
        .ok_or_else(|| format!("Database handle '{}' not found", db.handle))?;
    
    Ok(())
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
    let conn_arc = CONNECTIONS.get(&db.handle)
        .ok_or_else(|| format!("Database handle '{}' not found", db.handle))?;
    
    let conn = conn_arc.lock()
        .map_err(|e| format!("Failed to lock database connection: {}", e))?;
    
    // Start transaction
    conn.execute("BEGIN TRANSACTION", [])
        .map_err(|e| format!("Failed to begin transaction: {}", e))?;
    
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
                return Err(format!("Failed to execute SQL '{}': {}", sql, e));
            }
        }
    }
    
    // Commit if all succeeded
    conn.execute("COMMIT", [])
        .map_err(|e| {
            let _ = conn.execute("ROLLBACK", []);
            format!("Failed to commit transaction: {}", e)
        })?;
    
    Ok(DB_Result {
        rows_affected: total_affected,
        last_insert_id: last_id,
    })
}