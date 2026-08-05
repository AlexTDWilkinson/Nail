//! Valkey - the shared scratchpad between processes: sessions, counters,
//! queues and pub/sub that several programs (or several machines) read
//! together. For state one process keeps to itself, the cache module is
//! simpler and needs no server.

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct DB_Valkey {
    pub handle: String,
    pub url: String,
}

lazy_static::lazy_static! {
    static ref OPEN_CONNECTIONS: dashmap::DashMap<String, redis::aio::MultiplexedConnection> = dashmap::DashMap::new();
}

fn connection(conn: &DB_Valkey, what: &str) -> Result<redis::aio::MultiplexedConnection, String> {
    return OPEN_CONNECTIONS.get(&conn.handle).map(|held| held.clone()).ok_or_else(|| format!("{}: the connection to {} is closed", what, conn.url));
}

/// Connect to a Valkey server by URL - `redis://127.0.0.1/` locally,
/// `redis://:password@host:6379/0` with credentials.
pub async fn connect(url: String) -> Result<DB_Valkey, String> {
    let trimmed = url.trim().to_string();
    let client = redis::Client::open(trimmed.as_str()).map_err(|e| format!("db_valkey_connect: `{}` is not a valkey URL: {}", trimmed, e))?;
    let opened = client.get_multiplexed_tokio_connection().await.map_err(|e| format!("db_valkey_connect: could not reach {}: {}", trimmed, e))?;
    let handle = format!("valkey_{}", uuid::Uuid::new_v4());
    OPEN_CONNECTIONS.insert(handle.clone(), opened);
    return Ok(DB_Valkey { handle, url: trimmed });
}

/// The value under a key; an error when nothing is there.
pub async fn get(conn: &DB_Valkey, key: String) -> Result<String, String> {
    use redis::AsyncCommands;
    let mut open = connection(conn, "db_valkey_get")?;
    let found: Option<String> = open.get(&key).await.map_err(|e| format!("db_valkey_get: {}", e))?;
    return found.ok_or_else(|| format!("db_valkey_get: nothing stored under `{}`", key));
}

/// Store a value that stays until something deletes it.
pub async fn set(conn: &DB_Valkey, key: String, value: String) -> Result<(), String> {
    use redis::AsyncCommands;
    let mut open = connection(conn, "db_valkey_set")?;
    return open.set(&key, value).await.map_err(|e| format!("db_valkey_set: {}", e));
}

/// Store a value that disappears after the given number of seconds - the
/// shape sessions and rate limits take.
pub async fn set_ttl(conn: &DB_Valkey, key: String, value: String, ttl_seconds: i64) -> Result<(), String> {
    use redis::AsyncCommands;
    if ttl_seconds < 1 {
        return Err(format!("db_valkey_set_ttl: the life must be at least a second, not {}", ttl_seconds));
    }
    let mut open = connection(conn, "db_valkey_set_ttl")?;
    return open.set_ex(&key, value, ttl_seconds as u64).await.map_err(|e| format!("db_valkey_set_ttl: {}", e));
}

/// Drop a key. Deleting what is not there is fine.
pub async fn delete(conn: &DB_Valkey, key: String) -> Result<(), String> {
    use redis::AsyncCommands;
    let mut open = connection(conn, "db_valkey_delete")?;
    let _: i64 = open.del(&key).await.map_err(|e| format!("db_valkey_delete: {}", e))?;
    return Ok(());
}

/// Whether a key holds anything.
pub async fn exists(conn: &DB_Valkey, key: String) -> Result<bool, String> {
    use redis::AsyncCommands;
    let mut open = connection(conn, "db_valkey_exists")?;
    return open.exists(&key).await.map_err(|e| format!("db_valkey_exists: {}", e));
}

/// Add to a counter atomically and return the new value. A key that holds
/// nothing starts at zero, so the first increment answers with the amount.
pub async fn increment(conn: &DB_Valkey, key: String, by: i64) -> Result<i64, String> {
    use redis::AsyncCommands;
    let mut open = connection(conn, "db_valkey_increment")?;
    return open.incr(&key, by).await.map_err(|e| format!("db_valkey_increment: {}", e));
}

/// Give an existing key a remaining life in seconds.
pub async fn expire(conn: &DB_Valkey, key: String, seconds: i64) -> Result<(), String> {
    use redis::AsyncCommands;
    let mut open = connection(conn, "db_valkey_expire")?;
    let _: bool = open.expire(&key, seconds).await.map_err(|e| format!("db_valkey_expire: {}", e))?;
    return Ok(());
}

/// Push a value onto the end of a list and return the list's new length -
/// the producing half of a work queue.
pub async fn list_push(conn: &DB_Valkey, key: String, value: String) -> Result<i64, String> {
    use redis::AsyncCommands;
    let mut open = connection(conn, "db_valkey_list_push")?;
    return open.rpush(&key, value).await.map_err(|e| format!("db_valkey_list_push: {}", e));
}

/// Take the value at the front of a list - the consuming half of a work
/// queue. An empty list is an error, so a worker loop uses safe().
pub async fn list_pop(conn: &DB_Valkey, key: String) -> Result<String, String> {
    use redis::AsyncCommands;
    let mut open = connection(conn, "db_valkey_list_pop")?;
    let found: Option<String> = open.lpop(&key, None).await.map_err(|e| format!("db_valkey_list_pop: {}", e))?;
    return found.ok_or_else(|| format!("db_valkey_list_pop: the list `{}` is empty", key));
}

/// How many values a list holds.
pub async fn list_length(conn: &DB_Valkey, key: String) -> Result<i64, String> {
    use redis::AsyncCommands;
    let mut open = connection(conn, "db_valkey_list_length")?;
    return open.llen(&key).await.map_err(|e| format!("db_valkey_list_length: {}", e));
}

/// Send a message to everyone subscribed to a channel, and learn how many
/// heard it. Nail programs listen with other clients' subscribe tools or a
/// worker loop over a list; a subscriber callback is deliberately not offered
/// here - lists make more honest queues.
pub async fn publish(conn: &DB_Valkey, channel: String, message: String) -> Result<i64, String> {
    use redis::AsyncCommands;
    let mut open = connection(conn, "db_valkey_publish")?;
    return open.publish(&channel, message).await.map_err(|e| format!("db_valkey_publish: {}", e));
}

/// Forget the connection. Closing twice is not an error.
pub async fn close(conn: &DB_Valkey) -> Result<(), String> {
    OPEN_CONNECTIONS.remove(&conn.handle);
    return Ok(());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn nowhere_to_connect_is_a_clear_error() {
        let failure = connect("redis://127.0.0.1:1/".to_string()).await.unwrap_err();
        assert!(failure.contains("could not reach"), "got: {}", failure);
        assert!(connect("not a url".to_string()).await.unwrap_err().contains("not a valkey URL"));
    }

    #[tokio::test]
    async fn a_closed_connection_says_so() {
        let ghost = DB_Valkey { handle: "redis_nobody".to_string(), url: "redis://127.0.0.1/".to_string() };
        assert!(get(&ghost, "key".to_string()).await.unwrap_err().contains("is closed"));
        assert!(close(&ghost).await.is_ok());
    }

    /// The full round trip runs only where a compatible server listens on the usual port -
    /// locally and in CI with a service container; elsewhere it quietly passes.
    #[tokio::test]
    async fn the_round_trip_runs_when_a_server_is_near() {
        let Ok(conn) = connect("redis://127.0.0.1/".to_string()).await else { return };
        let key = format!("nail_test_{}", std::process::id());
        set(&conn, key.clone(), "hello".to_string()).await.unwrap();
        assert_eq!(get(&conn, key.clone()).await.unwrap(), "hello");
        assert!(exists(&conn, key.clone()).await.unwrap());
        assert_eq!(increment(&conn, format!("{}_count", key), 5).await.unwrap(), 5);
        let queue = format!("{}_queue", key);
        assert_eq!(list_push(&conn, queue.clone(), "job".to_string()).await.unwrap(), 1);
        assert_eq!(list_length(&conn, queue.clone()).await.unwrap(), 1);
        assert_eq!(list_pop(&conn, queue.clone()).await.unwrap(), "job");
        delete(&conn, key.clone()).await.unwrap();
        delete(&conn, format!("{}_count", key)).await.unwrap();
        assert!(get(&conn, key).await.unwrap_err().contains("nothing stored"));
        close(&conn).await.unwrap();
    }
}
