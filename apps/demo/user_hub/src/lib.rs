#![expect(clippy::unused_async)]

use canic::{Error, api::canister::placement::ShardingApi, prelude::*};
use ic_cdk::api::{canister_self, msg_caller};

const POOL_NAME: &str = "user_shards";

canic::start!();

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

#[canic_query(public)]
async fn demo_user_hub_overview(partition_key: Option<String>) -> Result<String, Error> {
    let mut lines = vec![
        "demo=user_hub".to_string(),
        format!("self={}", canister_self()),
        format!("caller={}", msg_caller()),
        format!("pool={POOL_NAME}"),
        format!("registry={:?}", ShardingApi::registry()),
    ];

    if let Some(key) = partition_key {
        lines.push(format!("partition_key={key}"));
        lines.push(format!(
            "current_assignment={:?}",
            ShardingApi::lookup_partition_key(POOL_NAME, &key)
        ));
        lines.push(format!(
            "plan={:?}",
            ShardingApi::plan_assign_to_pool(POOL_NAME, &key)?
        ));
    } else {
        lines.push("partition_key_hint=pass a key to preview its assignment".to_string());
    }

    Ok(lines.join("\n"))
}

#[canic_query(public)]
async fn demo_user_hub_plan(partition_key: String) -> Result<String, Error> {
    let current = ShardingApi::lookup_partition_key(POOL_NAME, &partition_key);
    let plan = ShardingApi::plan_assign_to_pool(POOL_NAME, &partition_key)?;

    Ok(format!(
        "partition_key={partition_key}\npool={POOL_NAME}\ncurrent_assignment={current:?}\nplan={plan:?}"
    ))
}

#[canic_update(public)]
async fn demo_user_hub_assign(partition_key: String) -> Result<String, Error> {
    canic::access::require_local()?;

    let before = ShardingApi::lookup_partition_key(POOL_NAME, &partition_key);
    let plan = ShardingApi::plan_assign_to_pool(POOL_NAME, &partition_key)?;
    let shard = ShardingApi::assign_to_pool(POOL_NAME, &partition_key).await?;
    let after = ShardingApi::lookup_partition_key(POOL_NAME, &partition_key);
    let shard_keys = ShardingApi::partition_keys(POOL_NAME, shard);

    Ok(format!(
        "partition_key={partition_key}\npool={POOL_NAME}\nbefore={before:?}\nplan={plan:?}\nassigned_shard={shard}\nafter={after:?}\nshard_keys={shard_keys:?}"
    ))
}

canic::finish!();
