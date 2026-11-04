//! Qualify one generated catalog and composed owner validators, without IC transport.

use canic_contracts::ids::{CanonicalNetworkId, wire_schema};
use ic_blob_storage_contracts::{dto::account::AccountRelationshipView, wire::schema};
use ic_platform::{
    Principal,
    candid::contract::{Int, Nat},
};
use ic_wire::{Limits, native, typescript};
use serde_json::json;
use std::{collections::BTreeMap, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("output directory required")?,
    );
    fs::create_dir_all(&directory)?;
    let limits = Limits {
        bytes: 4096,
        allocation: 16384,
        nodes: 256,
        depth: 32,
    };
    let network: CanonicalNetworkId = "ab".repeat(32).parse()?;
    let account = AccountRelationshipView {
        paid_canister: Principal::anonymous(),
        payment_account: Principal::management_canister(),
        spending_limit_per_day: Int::from(-1000),
        current_period_spent: Int::from(1_234_567),
        current_period_start: 1,
        added_timestamp: 2,
        expiration_timestamp: Some(3),
        bandwidth_baseline_uploaded: Nat::from(1000_u32),
        bandwidth_baseline_downloaded: Nat::from(0_u32),
        bandwidth_baseline_ts_ns: 4,
    };
    let request = (network, account);
    let mut methods = schema::methods().map_err(|error| format!("Blob schema: {error:?}"))?;
    // One qualification-only method composes actual types from both owners.
    methods.push(json!({
        "name":"composed", "mode":"query", "id":u64::MAX.to_string(),
        "request":typescript::native_schema::<(CanonicalNetworkId, AccountRelationshipView)>()
            .map_err(|error| format!("composed schema: {error:?}"))?,
        "response":typescript::native_schema::<()>().unwrap(),
        "requestBytes":limits.bytes, "responseBytes":limits.bytes,
        "requestAllocation":limits.allocation, "responseAllocation":limits.allocation,
        "requestNodes":limits.nodes, "responseNodes":limits.nodes,
        "requestDepth":limits.depth, "responseDepth":limits.depth,
    }));
    let bindings = typescript::generate_registry(
        &BTreeMap::new(),
        &methods,
        "./blob-runtime.js",
        &std::collections::BTreeMap::new(),
        |report| eprint!("{report}"),
    )
    .map_err(|error| format!("combined generation: {error:?}"))?;
    for (name, source) in [
        ("bindings.ts", bindings),
        ("blob-runtime.ts", schema::runtime_adapter("./adapter.js")),
        ("adapter.ts", wire_schema::TYPESCRIPT_ADAPTER.to_owned()),
        (
            "identities.ts",
            wire_schema::TYPESCRIPT_VALIDATOR.to_owned(),
        ),
        ("wire-runtime.ts", typescript::RUNTIME.to_owned()),
        ("check.mjs", include_str!("check.mjs").to_owned()),
        ("package.json", r#"{"type":"module"}"#.to_owned()),
    ] {
        fs::write(directory.join(name), source)?;
    }
    let frame = native::encode_request(u64::MAX, &request, limits)?;
    let decoded: (CanonicalNetworkId, AccountRelationshipView) =
        native::decode_request(u64::MAX, &frame, limits)?;
    assert_eq!(decoded, request);
    fs::write(directory.join("frame.json"), serde_json::to_vec(&frame)?)?;
    Ok(())
}
