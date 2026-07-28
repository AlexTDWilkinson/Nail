use dashmap::DashMap;
use datafusion::arrow::array::{Array, UInt64Array};
use datafusion::arrow::json::ArrayWriter;
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::prelude::{CsvReadOptions, ParquetReadOptions, SessionContext};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

// Store sessions by handle. SessionContext is Send + Sync and every
// DataFusion operation is async-native on tokio, so no blocking pool or
// mutex is involved.
lazy_static::lazy_static! {
    static ref SESSIONS: DashMap<String, SessionContext> = DashMap::new();
}

/// Represents a DataFusion analytics session
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DB_DataFusion {
    pub handle: String,
}

/// Represents the result of a DataFusion statement execution
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DB_DataFusion_Result {
    pub rows_affected: i64,
}

/// Look up the session for a handle
fn get_session(handle: &str) -> Result<SessionContext, String> {
    SESSIONS.get(handle).map(|entry| entry.value().clone()).ok_or_else(|| format!("db_datafusion: unknown session handle '{}' (was the session already closed?)", handle))
}

/// Open a new in-memory DataFusion analytics session
pub async fn datafusion_session() -> Result<DB_DataFusion, String> {
    let handle = format!("datafusion_{}", uuid::Uuid::new_v4());
    SESSIONS.insert(handle.clone(), SessionContext::new());
    Ok(DB_DataFusion { handle })
}

/// Register a Parquet file as a queryable table in the session
pub async fn datafusion_register_parquet(db: &DB_DataFusion, table: String, path: String) -> Result<(), String> {
    let session = get_session(&db.handle)?;
    session
        .register_parquet(&table, &path, ParquetReadOptions::default())
        .await
        .map_err(|e| format!("db_datafusion_register_parquet: could not register '{}' as table '{}': {}", path, table, e))
}

/// Register a CSV file as a queryable table in the session
pub async fn datafusion_register_csv(db: &DB_DataFusion, table: String, path: String) -> Result<(), String> {
    let session = get_session(&db.handle)?;
    session
        .register_csv(&table, &path, CsvReadOptions::new())
        .await
        .map_err(|e| format!("db_datafusion_register_csv: could not register '{}' as table '{}': {}", path, table, e))
}

/// Statements like INSERT report how many rows they wrote as a single
/// count column; sum it so execute can return rows_affected.
fn count_from_batches(batches: &[RecordBatch]) -> i64 {
    let mut total = 0i64;
    for batch in batches {
        if batch.num_columns() == 1 {
            if let Some(counts) = batch.column(0).as_any().downcast_ref::<UInt64Array>() {
                for index in 0..counts.len() {
                    if counts.is_valid(index) {
                        total += counts.value(index) as i64;
                    }
                }
            }
        }
    }
    total
}

/// Execute a SQL statement that doesn't return rows (CREATE TABLE, INSERT, ...)
pub async fn datafusion_execute(db: &DB_DataFusion, sql: String) -> Result<DB_DataFusion_Result, String> {
    let session = get_session(&db.handle)?;
    let dataframe = session.sql(&sql).await.map_err(|e| format!("db_datafusion_execute: failed to plan SQL '{}': {}", sql, e))?;
    let batches = dataframe.collect().await.map_err(|e| format!("db_datafusion_execute: failed to execute SQL '{}': {}", sql, e))?;
    Ok(DB_DataFusion_Result { rows_affected: count_from_batches(&batches) })
}

fn batches_to_json(batches: &[RecordBatch]) -> Result<Vec<serde_json::Value>, String> {
    let mut writer = ArrayWriter::new(Vec::new());
    writer.write_batches(&batches.iter().collect::<Vec<_>>()).map_err(|e| format!("db_datafusion_query: could not convert result rows to JSON: {}", e))?;
    writer.finish().map_err(|e| format!("db_datafusion_query: could not convert result rows to JSON: {}", e))?;
    let bytes = writer.into_inner();
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_slice(&bytes).map_err(|e| format!("db_datafusion_query: could not parse converted result rows: {}", e))
}

/// Execute a SQL query and return results as a vector of typed structs
pub async fn datafusion_query<T>(db: &DB_DataFusion, sql: String) -> Result<Vec<T>, String>
where
    T: DeserializeOwned + Send + 'static,
{
    let session = get_session(&db.handle)?;
    let dataframe = session.sql(&sql).await.map_err(|e| format!("db_datafusion_query: failed to plan SQL '{}': {}", sql, e))?;
    let batches = dataframe.collect().await.map_err(|e| format!("db_datafusion_query: failed to execute SQL '{}': {}", sql, e))?;

    batches_to_json(&batches)?
        .into_iter()
        .map(|row| serde_json::from_value::<T>(row).map_err(|e| format!("db_datafusion_query: could not deserialize a result row into the target struct: {}", e)))
        .collect()
}

/// Execute a SQL query and return the first result as a typed struct
pub async fn datafusion_query_single<T>(db: &DB_DataFusion, sql: String) -> Result<T, String>
where
    T: DeserializeOwned + Send + 'static,
{
    let results: Vec<T> = datafusion_query(db, sql).await?;
    results.into_iter().next().ok_or_else(|| "db_datafusion_query_single: the query returned no rows".to_string())
}

/// Close a DataFusion session
pub async fn datafusion_close(db: &DB_DataFusion) -> Result<(), String> {
    SESSIONS.remove(&db.handle).ok_or_else(|| format!("db_datafusion_close: unknown session handle '{}' (was the session already closed?)", db.handle))?;
    Ok(())
}
