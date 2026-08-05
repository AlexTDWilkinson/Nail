//! Valkey module stdlib registry entries. Every function takes the connection
//! struct, which `simple_fns!` cannot express, so they are built from one
//! helper.

use super::*;

fn valkey_fn(name: &'static str, rust_path: &str, extra_parameters: Vec<StdlibParameter>, return_type: NailDataTypeDescriptor, description: &'static str, example: &'static str) -> (&'static str, StdlibFunction) {
    let mut parameters = vec![StdlibParameter { name: "connection".to_string(), param_type: NailDataTypeDescriptor::Struct("DB_Valkey".to_string()), pass_by_reference: true }];
    parameters.extend(extra_parameters);
    return (name, StdlibFunction {
        rust_path: rust_path.to_string(),
        crate_deps: vec![CrateDependency::ValkeyClient, CrateDependency::Tokio, CrateDependency::DashMap, CrateDependency::Uuid, CrateDependency::Serde],
        struct_derives: vec![],
        custom_type_imports: vec![("DB_Valkey", "nail::std_lib::valkey")],
        module: StdlibModule::Valkey,
        parameters,
        return_type,
        diverging: false,
        description,
        example,
    });
}

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    m.insert("db_valkey_connect", StdlibFunction {
        rust_path: "std_lib::valkey::connect".to_string(),
        crate_deps: vec![CrateDependency::ValkeyClient, CrateDependency::Tokio, CrateDependency::DashMap, CrateDependency::Uuid, CrateDependency::Serde],
        struct_derives: vec![],
        custom_type_imports: vec![("DB_Valkey", "nail::std_lib::valkey")],
        module: StdlibModule::Valkey,
        parameters: vec![nail_param!(url: s)],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Struct("DB_Valkey".to_string()))),
        diverging: false,
        description: "Connects to a Valkey server, or anything else speaking the open RESP protocol - the shared scratchpad between processes: sessions, counters, queues that several programs or machines read together. KeyDB and Dragonfly answer the same calls, and so do servers from the protocol's original lineage. For state one process keeps to itself, the cache module needs no server. URLs look like `redis://127.0.0.1/` or `redis://:password@host:6379/0`.",
        example: "store:DB_Valkey = danger(db_valkey_connect(`redis://127.0.0.1/`));",
    });

    let entries = vec![
        valkey_fn("db_valkey_get", "std_lib::valkey::get", vec![nail_param!(key: s)], nail_type!((s!e)),
            "The value under a key. An error when nothing is there.",
            "session:s = safe(db_valkey_get(store, session_id), fresh_session);"),
        valkey_fn("db_valkey_set", "std_lib::valkey::set", vec![nail_param!(key: s), nail_param!(value: s)], nail_type!((v!e)),
            "Stores a value that stays until something deletes it.",
            "danger(db_valkey_set(store, user_key, profile_json));"),
        valkey_fn("db_valkey_set_ttl", "std_lib::valkey::set_ttl", vec![nail_param!(key: s), nail_param!(value: s), nail_param!(ttl_seconds: i)], nail_type!((v!e)),
            "Stores a value that disappears after the given number of seconds - the shape sessions and rate limits take.",
            "danger(db_valkey_set_ttl(store, session_id, session_json, 3600));"),
        valkey_fn("db_valkey_delete", "std_lib::valkey::delete", vec![nail_param!(key: s)], nail_type!((v!e)),
            "Drops a key. Deleting what is not there is fine.",
            "danger(db_valkey_delete(store, session_id));"),
        valkey_fn("db_valkey_exists", "std_lib::valkey::exists", vec![nail_param!(key: s)], nail_type!((b!e)),
            "Whether a key holds anything.",
            "known:b = danger(db_valkey_exists(store, user_key));"),
        valkey_fn("db_valkey_increment", "std_lib::valkey::increment", vec![nail_param!(key: s), nail_param!(by: i)], nail_type!((i!e)),
            "Adds to a counter atomically and returns the new value. A key holding nothing starts at zero - which is how a rate limiter counts.",
            "hits:i = danger(db_valkey_increment(store, visit_key, 1));"),
        valkey_fn("db_valkey_expire", "std_lib::valkey::expire", vec![nail_param!(key: s), nail_param!(seconds: i)], nail_type!((v!e)),
            "Gives an existing key a remaining life in seconds.",
            "danger(db_valkey_expire(store, visit_key, 60));"),
        valkey_fn("db_valkey_list_push", "std_lib::valkey::list_push", vec![nail_param!(key: s), nail_param!(value: s)], nail_type!((i!e)),
            "Pushes a value onto the end of a list and returns the new length - the producing half of a work queue.",
            "waiting:i = danger(db_valkey_list_push(store, `jobs`, job_json));"),
        valkey_fn("db_valkey_list_pop", "std_lib::valkey::list_pop", vec![nail_param!(key: s)], nail_type!((s!e)),
            "Takes the value at the front of a list - the consuming half of a work queue. An empty list is an error, so a worker loop uses safe().",
            "job:s = danger(db_valkey_list_pop(store, `jobs`));"),
        valkey_fn("db_valkey_list_length", "std_lib::valkey::list_length", vec![nail_param!(key: s)], nail_type!((i!e)),
            "How many values a list holds.",
            "backlog:i = danger(db_valkey_list_length(store, `jobs`));"),
        valkey_fn("db_valkey_publish", "std_lib::valkey::publish", vec![nail_param!(channel: s), nail_param!(message: s)], nail_type!((i!e)),
            "Sends a message to everyone subscribed to a channel, and answers how many heard it.",
            "heard:i = danger(db_valkey_publish(store, `updates`, changed_json));"),
        valkey_fn("db_valkey_close", "std_lib::valkey::close", vec![], nail_type!((v!e)),
            "Forgets the connection. Closing twice is not an error.",
            "danger(db_valkey_close(store));"),
    ];
    for (name, function) in entries {
        m.insert(name, function);
    }
}
