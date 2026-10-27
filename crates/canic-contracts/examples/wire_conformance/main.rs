//! Emit actual Canic DTO conformance vectors and generated clients for local qualification.

mod accounts;
mod identities;
mod results;
mod runtime_values;
mod unsigned_magnitude;

use canic_contracts::{
    dto::page::{Page, PageRequest},
    ids::{CanonicalNetworkId, wire_schema},
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
        bytes: 1024,
        allocation: 8192,
        nodes: 128,
        depth: 16,
    };
    let network = "ab".repeat(32);
    let request = PageRequest {
        limit: 128,
        offset: 1,
    };
    let response = Page {
        entries: vec![network.parse::<CanonicalNetworkId>()?],
        total: 1,
    };
    // Test-only identity; this example registers no production endpoint.
    let method = json!({
        "name":"pagination", "mode":"query", "id":"17",
        "request":typescript::native_schema::<PageRequest>().map_err(|e| format!("request schema: {e:?}"))?,
        "response":typescript::native_schema::<Page<CanonicalNetworkId>>().map_err(|e| format!("response schema: {e:?}"))?,
        "requestBytes":1024,"responseBytes":1024,
        "requestAllocation":8192,"responseAllocation":8192,
        "requestNodes":128,"responseNodes":128,
        "requestDepth":16,"responseDepth":16
    });
    let principals = Page {
        entries: vec![
            ic_platform::Principal::management_canister(),
            ic_platform::Principal::anonymous(),
            ic_platform::Principal::from_slice(&[42; 29]),
        ],
        total: 3,
    };
    let mut principal_method = method.clone();
    principal_method["name"] = json!("principals");
    principal_method["id"] = json!("18");
    principal_method["response"] = typescript::native_schema::<Page<ic_platform::Principal>>()
        .map_err(|error| format!("principal schema: {error:?}"))?;
    let result_method = results::method(&method)?;
    let account_method = accounts::method(&method)?;
    let identity_method = identities::method(&method)?;
    let mut methods = vec![
        method.clone(),
        principal_method,
        result_method,
        account_method,
        identity_method,
    ];
    methods.push(unsigned_magnitude::method(&method)?);
    let runtime_values = runtime_values::append(&mut methods, &method, limits)?;
    fs::write(
        directory.join("bindings.ts"),
        typescript::generate_registry(
            &BTreeMap::new(),
            &methods,
            "./adapter.js",
            &std::collections::BTreeMap::new(),
            |report| eprint!("{report}"),
        )
        .map_err(|error| format!("generation: {error:?}"))?,
    )?;
    fs::write(directory.join("wire-runtime.ts"), typescript::RUNTIME)?;
    fs::write(
        directory.join("identities.ts"),
        wire_schema::TYPESCRIPT_VALIDATOR,
    )?;
    fs::write(
        directory.join("adapter.ts"),
        wire_schema::TYPESCRIPT_ADAPTER,
    )?;
    fs::write(directory.join("check.mjs"), include_str!("check.mjs"))?;
    fs::write(directory.join("package.json"), r#"{"type":"module"}"#)?;
    fs::write(
        directory.join("vectors.json"),
        serde_json::to_vec(&json!({
            "runtimeValues":runtime_values,
            "unsignedMagnitudes": unsigned_magnitude::vectors(limits)?,
            "results":results::vectors(limits).map_err(|e| format!("diagnostic vectors: {e}"))?,
            "identities":identities::vectors(limits).map_err(|e| format!("identity vectors: {e}"))?,
            "accounts":accounts::vectors(limits).map_err(|e| format!("account vectors: {e}"))?,
            "network":network,
            "principals":principals.entries.iter().map(ic_platform::Principal::as_slice).collect::<Vec<_>>(),
            "principalResponse":native::encode_response(18, &principals, limits)?,
            "request":native::encode_request(17, &request, limits)?,
            "response":native::encode_response(17, &response, limits)?
        }))?,
    )?;
    Ok(())
}
