#![expect(clippy::unused_async)]

use canic::{Error, prelude::*};

canic::start!();

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

#[canic_query]
async fn demo_user_shard_overview(partition_key: Option<String>) -> Result<String, Error> {
    let mut lines = vec![
        "demo=user_shard".to_string(),
        format!("self={}", canister_self()),
        format!("caller={}", msg_caller()),
    ];

    if let Some(key) = partition_key {
        lines.push(format!("partition_key={key}"));
        lines.push(format!("partition_key_bytes={}", key.len()));
    } else {
        lines.push("partition_key_hint=pass a key to see what this shard receives".to_string());
    }

    Ok(lines.join("\n"))
}

#[canic_query]
async fn demo_user_shard_describe(partition_key: String) -> Result<String, Error> {
    Ok(format!(
        "partition_key={partition_key}\npartition_key_bytes={}\nhandled_by={}\ncaller={}",
        partition_key.len(),
        canister_self(),
        msg_caller()
    ))
}

canic::finish!();
