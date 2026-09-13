//! Single-step PocketIC platform effects with exact process ownership.

use crate::icp::cycles_ledger::{
    CanisterSettings, CmcCreateCanisterArgs, CreateCanisterArgs, CreateCanisterError,
    CreateCanisterSuccess, SubnetSelection,
};
use crate::local_fleet::{
    LocalFleetError,
    model::{LocalAllocationIntentRecord, LocalFleetConfig},
};
use candid::{CandidType, Deserialize, Nat, Principal};
use canic_core::dto::fleet_subnet_root::FleetSubnetRootAuthority;
use ic_testkit::{
    pic::{PocketIcBuilderExt, PocketIcManagedServer, PocketIcStartupConfig},
    pocket_ic::{
        PocketIc, PocketIcBuilder,
        common::rest::{IcpFeatures, IcpFeaturesConfig, InstanceHttpGatewayConfig},
    },
};
use std::{path::Path, time::Duration};

/// Spawn one managed server whose drop waits for that exact child.
pub fn server(config: &LocalFleetConfig) -> Result<PocketIcManagedServer, LocalFleetError> {
    Ok(PocketIcStartupConfig::spawn(
        &config.server_binary,
        Duration::from_secs(u64::from(config.request_timeout_secs)),
    )
    .with_server_hard_ttl(Duration::from_secs(u64::from(config.server_lifetime_secs)))
    .start_managed_server()?)
}

/// Load the owned state directory or create the explicit bounded subnet topology.
pub fn instance(
    config: &LocalFleetConfig,
    directory: &Path,
    server: &PocketIcManagedServer,
) -> Result<PocketIc, LocalFleetError> {
    std::fs::create_dir_all(directory)?;
    let mut builder = PocketIcBuilder::new()
        .with_nns_subnet()
        .with_ii_subnet()
        .with_icp_features(IcpFeatures {
            registry: Some(IcpFeaturesConfig::DefaultConfig),
            cycles_minting: Some(IcpFeaturesConfig::DefaultConfig),
            cycles_token: Some(IcpFeaturesConfig::DefaultConfig),
            ii: Some(IcpFeaturesConfig::DefaultConfig),
            ..IcpFeatures::default()
        });
    for _ in 0..config.application_subnets {
        builder = builder.with_application_subnet();
    }
    let builder = builder
        .with_http_gateway(InstanceHttpGatewayConfig {
            ip_addr: Some("127.0.0.1".into()),
            port: Some(config.gateway_port),
            domains: None,
            https_config: None,
            domain_custom_provider_local_file: None,
        })
        .with_auto_progress()
        .with_state_dir(directory.to_path_buf())
        .with_max_request_time_ms(Some(u64::from(config.request_timeout_secs) * 1000));
    builder
        .try_build(PocketIcStartupConfig::connect(
            server.url(),
            Duration::from_secs(u64::from(config.request_timeout_secs)),
        ))
        .map_err(|source| {
            let output = server.output();
            LocalFleetError::InstanceStartup {
                source: Box::new(source),
                stdout: output.stdout().into(),
                stderr: output.stderr().into(),
            }
        })
}

/// Expose one selected loopback port; the instance owns its gateway lifecycle.
pub fn gateway(pic: &mut PocketIc, port: u16) -> Result<String, LocalFleetError> {
    guarded(|| pic.make_live(Some(port)).to_string())
}

/// Stop the instance's gateway before deterministic clock operations or restart.
pub fn stop_gateway(pic: &mut PocketIc) -> Result<(), LocalFleetError> {
    guarded(|| pic.stop_live())
}

/// Submit the retained Ledger request; duplicates return the same exact created canister.
pub fn create(
    pic: &PocketIc,
    allocation: &LocalAllocationIntentRecord,
    config: &LocalFleetConfig,
) -> Result<candid::Principal, LocalFleetError> {
    if let Some(id) = allocation.canister_id {
        if !guarded(|| pic.canister_exists(id))?
            || guarded(|| pic.get_subnet(id))? != Some(allocation.subnet_id)
        {
            return Err(LocalFleetError::Identity);
        }
        return Ok(id);
    }
    let ledger = Principal::from_text("um5iw-rqaaa-aaaaq-qaaba-cai")
        .map_err(|_| LocalFleetError::Identity)?;
    let args = CreateCanisterArgs {
        amount: Nat::from(config.allocation_debit_cycles),
        created_at_time: Some(allocation.created_at_time),
        from_subaccount: None,
        creation_args: Some(CmcCreateCanisterArgs {
            settings: Some(CanisterSettings {
                controllers: Some(vec![allocation.input.controller]),
                compute_allocation: None,
                memory_allocation: Some(config.canister_memory_bytes.into()),
                freezing_threshold: None,
                reserved_cycles_limit: None,
            }),
            subnet_selection: Some(SubnetSelection::Subnet {
                subnet: allocation.subnet_id,
            }),
        }),
    };
    let payload =
        candid::encode_one(args).map_err(|error| LocalFleetError::Platform(error.to_string()))?;
    let bytes =
        guarded(|| pic.update_call(ledger, Principal::anonymous(), "create_canister", payload))?
            .map_err(|error| LocalFleetError::Platform(format!("{error:?}")))?;
    let result: Result<CreateCanisterSuccess, CreateCanisterError> =
        candid::decode_one(&bytes).map_err(|error| LocalFleetError::Platform(error.to_string()))?;
    let id = match result {
        Ok(value) => value.canister_id,
        Err(CreateCanisterError::Duplicate {
            canister_id: Some(id),
            ..
        }) => id,
        Err(error) => {
            return Err(LocalFleetError::Platform(format!(
                "local Ledger creation: {error:?}"
            )));
        }
    };
    if guarded(|| pic.get_subnet(id))? != Some(allocation.subnet_id) {
        return Err(LocalFleetError::Identity);
    }
    Ok(id)
}

/// Advance only this instance's deterministic time; automatic progress resumes separately.
pub fn advance(pic: &PocketIc, seconds: u32) -> Result<(), LocalFleetError> {
    guarded(|| pic.advance_time(Duration::from_secs(u64::from(seconds))))
}

/// Convert upstream panicking transport boundaries to one owned local failure.
pub fn guarded<T>(action: impl FnOnce() -> T) -> Result<T, LocalFleetError> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(action))
        .map_err(|_| LocalFleetError::Platform("PocketIC operation did not complete".into()))
}

/// Set one new local canister's final controller set, reconciling a lost response by observation.
pub fn controllers(
    pic: &PocketIc,
    target: candid::Principal,
    operator: candid::Principal,
    expected: &[candid::Principal],
) -> Result<(), LocalFleetError> {
    let mut observed = guarded(|| pic.get_controllers(target))?;
    observed.sort();
    if observed == expected {
        return Ok(());
    }
    if observed != [operator] {
        return Err(LocalFleetError::Identity);
    }
    guarded(|| pic.set_controllers(target, Some(operator), expected.to_vec()))?
        .map_err(|error| LocalFleetError::Platform(format!("{error:?}")))?;
    Ok(())
}

/// Submit one initial installation after workflow retained the exact attempt and observed no module.
pub fn install_root(
    pic: &PocketIc,
    target: &crate::local_fleet::view::LocalRootInstallationView,
) -> Result<(), LocalFleetError> {
    guarded(|| {
        pic.install_canister(
            target.canister_id,
            target.wasm.clone(),
            target.arguments.clone(),
            Some(target.operator),
        );
    })
}

/// Read and compare the complete Root authority after installation or a lost response.
pub fn verify_root_authority(
    pic: &PocketIc,
    target: &crate::local_fleet::view::LocalRootInstallationView,
) -> Result<(), LocalFleetError> {
    #[derive(CandidType)]
    enum Request {
        FleetAuthority,
    }
    #[derive(CandidType, Deserialize)]
    enum Response {
        FleetAuthority(Box<FleetSubnetRootAuthority>),
    }
    let request = candid::encode_one(Request::FleetAuthority)
        .map_err(|error| LocalFleetError::Platform(error.to_string()))?;
    let bytes = guarded(|| {
        pic.query_call(
            target.canister_id,
            target.operator,
            canic_core::protocol::CANIC_ROOT_STATUS,
            request,
        )
    })?
    .map_err(|error| LocalFleetError::Platform(format!("{error:?}")))?;
    let response = candid::decode_one::<Result<Response, canic_core::dto::error::Error>>(&bytes)
        .map_err(|error| LocalFleetError::Platform(error.to_string()))?;
    let Response::FleetAuthority(actual) = response.map_err(LocalFleetError::Root)?;
    if *actual != target.authority {
        return Err(LocalFleetError::Identity);
    }
    Ok(())
}

/// Reconcile a possibly lost successful installation before consuming another attempt.
pub fn root_needs_install(
    pic: &PocketIc,
    target: &crate::local_fleet::view::LocalRootInstallationView,
) -> Result<bool, LocalFleetError> {
    let status = guarded(|| pic.canister_status(target.canister_id, Some(target.operator)))?
        .map_err(|error| LocalFleetError::Platform(format!("{error:?}")))?;
    match status.module_hash {
        None => Ok(true),
        Some(hash) if canic_core::cdk::utils::hash::hex_bytes(&hash) == target.wasm_sha256 => {
            Ok(false)
        }
        Some(_) => Err(LocalFleetError::Identity),
    }
}
