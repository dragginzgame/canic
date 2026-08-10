use candid::{CandidType, Principal};
use canic_core::{
    dto::{
        fleet_registry::{
            FleetDirectoryProvenance, FleetDirectorySnapshot, FleetRegistry, FleetRegistryVersion,
            FleetSubnetRootDirectoryEntry, FleetSubnetRootDrainingPublicationRequest,
            FleetSubnetRootDrainingPublicationResponse, FleetSubnetRootDrainingReservationRequest,
            FleetSubnetRootDrainingReservationResponse,
            FleetSubnetRootDrainingReservationStatusRequest,
            FleetSubnetRootRegistryMirrorActivationRequest,
            FleetSubnetRootRegistryMirrorActivationResponse,
        },
        fleet_subnet_root::{
            FleetSubnetRootDrainingRequest, FleetSubnetRootDrainingResponse,
            FleetSubnetRootFinalInventoryRequest, FleetSubnetRootFinalInventoryResponse,
            FleetSubnetRootRemovalRequest, FleetSubnetRootStoreBindingFinalizationRequest,
            FleetSubnetRootStoreBindingFinalizationResponse,
            FleetSubnetRootStoreBindingFinalizationStatusRequest,
            FleetSubnetRootStoreDeletionRequest, FleetSubnetRootStoreDeletionResponse,
            FleetSubnetRootStoreReclamationRequest, FleetSubnetRootStoreReclamationResponse,
        },
        pool::{
            CanisterPoolAssetStatus, CanisterPoolResponse, CanisterPoolStatusRequest,
            PoolAdminCommand, PoolAdminResponse,
        },
        root_store::RootStoreBootstrapRequest,
    },
    protocol,
};
use canic_host::icp::{IcpCli, decode_json_result_response};
use serde::{Serialize, de::DeserializeOwned};
use std::{env, fs, path::PathBuf};

const USAGE: &str = "usage: cargo run -p canic-host --example empty_fleet_subnet_root_retirement -- \
    <--confirm-disposable-empty-root|--confirm-resume-store-deletion> \
    <icp-executable> <icp-root> <environment> \
    <coordinator-principal> <fleet-subnet-root-principal> \
    <store-bootstrap-manifest-bytes> <operation-id-hex>";
const CANIC_POOL_LIST: &str = "canic_pool_list";
const CANIC_POOL_ADMIN: &str = "canic_pool_admin";

#[derive(Serialize)]
struct RetirementReceipt {
    operation_id_hex: String,
    coordinator: Principal,
    fleet_subnet_root: Principal,
    handed_off_pool_assets: Vec<Principal>,
    draining_registry: FleetRegistryVersion,
    final_inventory: FleetSubnetRootFinalInventoryResponse,
    store_reclamation: FleetSubnetRootStoreReclamationResponse,
    store_binding_finalization: FleetSubnetRootStoreBindingFinalizationResponse,
    store_deletion: FleetSubnetRootStoreDeletionResponse,
}

#[derive(Serialize)]
struct StoreDeletionResumeReceipt {
    operation_id_hex: String,
    fleet_subnet_root: Principal,
    store_binding_finalization: FleetSubnetRootStoreBindingFinalizationResponse,
    store_deletion: FleetSubnetRootStoreDeletionResponse,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum RetirementCommand {
    Retire,
    ResumeStoreDeletion,
}

struct RetirementContext {
    command: RetirementCommand,
    icp: IcpCli,
    coordinator: Principal,
    fleet_subnet_root: Principal,
    store_bootstrap_manifest_bytes: u64,
    operation_id: [u8; 32],
    operation_id_hex: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let context = parse_context()?;
    if context.command == RetirementCommand::ResumeStoreDeletion {
        return resume_store_deletion(&context);
    }
    let icp = &context.icp;
    let coordinator = context.coordinator;
    let fleet_subnet_root = context.fleet_subnet_root;
    let operation_id = context.operation_id;
    let published = publish_draining(&context)?;

    let handed_off_pool_assets = handoff_pool_assets(icp, fleet_subnet_root, coordinator)?;
    eprintln!("retirement stage complete: prepaid pool handoff");
    let final_inventory: FleetSubnetRootFinalInventoryResponse = call(
        icp,
        fleet_subnet_root,
        protocol::CANIC_FLEET_SUBNET_ROOT_DRAINING_INVENTORY_FINALIZE,
        &FleetSubnetRootFinalInventoryRequest {
            operation_id,
            expected_registry: published.version.clone(),
        },
    )?;
    eprintln!("retirement stage complete: final root inventory");
    let _: canic_core::dto::fleet_registry::FleetSubnetRootRemovalPublicationResponse = call(
        icp,
        fleet_subnet_root,
        protocol::CANIC_FLEET_SUBNET_ROOT_REMOVAL_PUBLISH,
        &FleetSubnetRootRemovalRequest {
            operation_id,
            expected_registry: final_inventory.registry.clone(),
        },
    )?;
    eprintln!("retirement stage complete: logical root removal");
    let store_reclamation: FleetSubnetRootStoreReclamationResponse = call(
        icp,
        fleet_subnet_root,
        protocol::CANIC_FLEET_SUBNET_ROOT_STORE_RECLAIM,
        &FleetSubnetRootStoreReclamationRequest {
            operation_id,
            expected_final_inventory_hash: final_inventory.inventory_hash,
        },
    )?;
    eprintln!("retirement stage complete: Store reclamation");
    let store_binding_finalization: FleetSubnetRootStoreBindingFinalizationResponse = call(
        icp,
        fleet_subnet_root,
        protocol::CANIC_FLEET_SUBNET_ROOT_STORE_BINDING_FINALIZE,
        &FleetSubnetRootStoreBindingFinalizationRequest {
            operation_id,
            expected_reclamation_hash: store_reclamation.reclamation_hash,
        },
    )?;
    eprintln!("retirement stage complete: Store binding finalization");
    let store_deletion: FleetSubnetRootStoreDeletionResponse = call(
        icp,
        fleet_subnet_root,
        protocol::CANIC_FLEET_SUBNET_ROOT_STORE_DELETE,
        &FleetSubnetRootStoreDeletionRequest {
            operation_id,
            expected_binding_finalization_hash: store_binding_finalization.finalization_hash,
        },
    )?;
    eprintln!("retirement stage complete: physical Store deletion");

    let receipt = RetirementReceipt {
        operation_id_hex: context.operation_id_hex,
        coordinator,
        fleet_subnet_root,
        handed_off_pool_assets,
        draining_registry: published.version,
        final_inventory,
        store_reclamation,
        store_binding_finalization,
        store_deletion,
    };
    println!("{}", serde_json::to_string_pretty(&receipt)?);
    Ok(())
}

fn parse_context() -> Result<RetirementContext, Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let (Some(confirmation), Some(icp_executable), Some(icp_root), Some(environment)) =
        (args.next(), args.next(), args.next(), args.next())
    else {
        return Err(USAGE.into());
    };
    let command = match confirmation.as_str() {
        "--confirm-disposable-empty-root" => RetirementCommand::Retire,
        "--confirm-resume-store-deletion" => RetirementCommand::ResumeStoreDeletion,
        _ => return Err(USAGE.into()),
    };
    let (
        Some(coordinator),
        Some(fleet_subnet_root),
        Some(store_bootstrap_manifest_bytes),
        Some(operation_id_hex),
    ) = (args.next(), args.next(), args.next(), args.next())
    else {
        return Err(USAGE.into());
    };
    if args.next().is_some() {
        return Err(USAGE.into());
    }

    let coordinator = Principal::from_text(coordinator)?;
    let fleet_subnet_root = Principal::from_text(fleet_subnet_root)?;
    let store_bootstrap_manifest_bytes = store_bootstrap_manifest_bytes.parse::<u64>()?;
    let operation_id = parse_operation_id(&operation_id_hex)?;
    let icp_root = PathBuf::from(icp_root).canonicalize()?;
    let icp = IcpCli::new(icp_executable, Some(environment)).with_cwd(icp_root);
    Ok(RetirementContext {
        command,
        icp,
        coordinator,
        fleet_subnet_root,
        store_bootstrap_manifest_bytes,
        operation_id,
        operation_id_hex,
    })
}

fn resume_store_deletion(context: &RetirementContext) -> Result<(), Box<dyn std::error::Error>> {
    let store_binding_finalization: FleetSubnetRootStoreBindingFinalizationResponse = query(
        &context.icp,
        context.fleet_subnet_root,
        protocol::CANIC_FLEET_SUBNET_ROOT_STORE_BINDING_FINALIZATION_STATUS,
        &FleetSubnetRootStoreBindingFinalizationStatusRequest {
            operation_id: context.operation_id,
        },
    )?;
    let store_deletion: FleetSubnetRootStoreDeletionResponse = call(
        &context.icp,
        context.fleet_subnet_root,
        protocol::CANIC_FLEET_SUBNET_ROOT_STORE_DELETE,
        &FleetSubnetRootStoreDeletionRequest {
            operation_id: context.operation_id,
            expected_binding_finalization_hash: store_binding_finalization.finalization_hash,
        },
    )?;
    let receipt = StoreDeletionResumeReceipt {
        operation_id_hex: context.operation_id_hex.clone(),
        fleet_subnet_root: context.fleet_subnet_root,
        store_binding_finalization,
        store_deletion,
    };
    println!("{}", serde_json::to_string_pretty(&receipt)?);
    Ok(())
}

fn publish_draining(
    context: &RetirementContext,
) -> Result<FleetSubnetRootDrainingPublicationResponse, Box<dyn std::error::Error>> {
    let reservation = retained_or_new_draining_reservation(context)?;
    eprintln!("retirement stage complete: Coordinator draining reservation");
    let root_draining: FleetSubnetRootDrainingResponse = call(
        &context.icp,
        context.fleet_subnet_root,
        protocol::CANIC_FLEET_SUBNET_ROOT_DRAINING_BEGIN,
        &FleetSubnetRootDrainingRequest {
            operation_id: context.operation_id,
            expected_registry: reservation.request.expected_registry.clone(),
        },
    )?;
    if root_draining.reservation_hash != reservation.reservation_hash {
        return Err("root draining receipt differs from the Coordinator reservation".into());
    }
    require_empty_root(&root_draining)?;
    eprintln!("retirement stage complete: root draining fence");

    let publication_registry: FleetRegistryVersion = query(
        &context.icp,
        context.coordinator,
        protocol::CANIC_FLEET_REGISTRY_VERSION,
        &(),
    )?;
    let published: FleetSubnetRootDrainingPublicationResponse = call(
        &context.icp,
        context.coordinator,
        protocol::CANIC_FLEET_REGISTRY_PUBLISH_ROOT_DRAINING,
        &FleetSubnetRootDrainingPublicationRequest {
            expected_registry: publication_registry,
            root_draining,
        },
    )?;
    eprintln!("retirement stage complete: Coordinator draining publication");
    activate_draining_mirror(context, published.previous_version.clone(), &published)?;
    Ok(published)
}

fn retained_or_new_draining_reservation(
    context: &RetirementContext,
) -> Result<FleetSubnetRootDrainingReservationResponse, Box<dyn std::error::Error>> {
    let retained = call(
        &context.icp,
        context.coordinator,
        protocol::CANIC_FLEET_REGISTRY_ROOT_DRAINING_RESERVATION_STATUS,
        &FleetSubnetRootDrainingReservationStatusRequest {
            operation_id: context.operation_id,
            fleet_subnet_root: context.fleet_subnet_root,
        },
    );
    if let Ok(reservation) = retained {
        return Ok(reservation);
    }
    let active_registry: FleetRegistryVersion = query(
        &context.icp,
        context.coordinator,
        protocol::CANIC_FLEET_REGISTRY_VERSION,
        &(),
    )?;
    let registry: FleetRegistry = query(
        &context.icp,
        context.coordinator,
        protocol::CANIC_FLEET_REGISTRY,
        &(),
    )?;
    let expected_root = registry
        .fleet_subnet_roots
        .iter()
        .find(|entry| entry.fleet_subnet_root == context.fleet_subnet_root)
        .cloned()
        .ok_or("Fleet Subnet Root is absent from the Coordinator Registry")?;
    call(
        &context.icp,
        context.coordinator,
        protocol::CANIC_FLEET_REGISTRY_ROOT_DRAINING_RESERVATION_PREPARE,
        &FleetSubnetRootDrainingReservationRequest {
            operation_id: context.operation_id,
            expected_registry: active_registry,
            expected_root,
        },
    )
}

fn activate_draining_mirror(
    context: &RetirementContext,
    previous_registry: FleetRegistryVersion,
    published: &FleetSubnetRootDrainingPublicationResponse,
) -> Result<(), Box<dyn std::error::Error>> {
    let registry: FleetRegistry = query(
        &context.icp,
        context.coordinator,
        protocol::CANIC_FLEET_REGISTRY,
        &(),
    )?;
    let directory = FleetDirectorySnapshot {
        provenance: FleetDirectoryProvenance {
            registry: published.version.clone(),
            source_fleet_subnet_root: context.fleet_subnet_root,
        },
        fleet_subnet_roots: registry
            .fleet_subnet_roots
            .iter()
            .map(|entry| FleetSubnetRootDirectoryEntry {
                placement_subnet: entry.placement_subnet,
                fleet_subnet_root: entry.fleet_subnet_root,
                status: entry.status,
            })
            .collect(),
        services: vec![],
    };
    let _: FleetSubnetRootRegistryMirrorActivationResponse = call(
        &context.icp,
        context.fleet_subnet_root,
        protocol::CANIC_FLEET_REGISTRY_ACTIVATE_MIRROR,
        &FleetSubnetRootRegistryMirrorActivationRequest {
            previous_registry,
            expected_registry: published.version.clone(),
            expected_directory: directory,
            store_bootstrap: RootStoreBootstrapRequest {
                manifest_payload_size_bytes: context.store_bootstrap_manifest_bytes,
            },
        },
    )?;
    eprintln!("retirement stage complete: root draining mirror activation");
    Ok(())
}

fn require_empty_root(
    draining: &FleetSubnetRootDrainingResponse,
) -> Result<(), Box<dyn std::error::Error>> {
    let is_empty = draining.reserved_component_instances == 0
        && draining.committed_component_instances == 0
        && draining.managed_descendants == 0
        && draining.known_created_component_canisters == 0;
    if !is_empty {
        return Err("Fleet Subnet Root is not empty".into());
    }
    Ok(())
}

fn handoff_pool_assets(
    icp: &IcpCli,
    fleet_subnet_root: Principal,
    coordinator: Principal,
) -> Result<Vec<Principal>, Box<dyn std::error::Error>> {
    let pool: CanisterPoolResponse = query(
        icp,
        fleet_subnet_root,
        CANIC_POOL_LIST,
        &CanisterPoolStatusRequest {
            start_after: None,
            limit: 256,
        },
    )?;
    if pool.next_start_after.is_some() || pool.pending_handoff.is_some() {
        return Err("Canister pool is not in one complete settled page".into());
    }
    let mut handed_off = Vec::with_capacity(pool.entries.len());
    for asset in pool.entries {
        if !matches!(
            asset.status,
            CanisterPoolAssetStatus::Ready | CanisterPoolAssetStatus::Failed { .. }
        ) {
            return Err("Canister pool contains a non-handoff-ready asset".into());
        }
        let response: PoolAdminResponse = call(
            icp,
            fleet_subnet_root,
            CANIC_POOL_ADMIN,
            &PoolAdminCommand::Handoff {
                canister_id: asset.canister_id,
                recipient: coordinator,
            },
        )?;
        if response
            != (PoolAdminResponse::HandedOff {
                canister_id: asset.canister_id,
                recipient: coordinator,
            })
        {
            return Err("Canister pool returned a nonterminal handoff response".into());
        }
        handed_off.push(asset.canister_id);
    }
    Ok(handed_off)
}

fn query<I, O>(
    icp: &IcpCli,
    canister: Principal,
    method: &str,
    input: &I,
) -> Result<O, Box<dyn std::error::Error>>
where
    I: CandidType,
    O: CandidType + DeserializeOwned,
{
    invoke(icp, canister, method, input, true)
}

fn call<I, O>(
    icp: &IcpCli,
    canister: Principal,
    method: &str,
    input: &I,
) -> Result<O, Box<dyn std::error::Error>>
where
    I: CandidType,
    O: CandidType + DeserializeOwned,
{
    invoke(icp, canister, method, input, false)
}

fn invoke<I, O>(
    icp: &IcpCli,
    canister: Principal,
    method: &str,
    input: &I,
    is_query: bool,
) -> Result<O, Box<dyn std::error::Error>>
where
    I: CandidType,
    O: CandidType + DeserializeOwned,
{
    let bytes = candid::encode_one(input)?;
    let path = env::temp_dir().join(format!(
        "canic-empty-root-retirement-{}-{}.bin",
        std::process::id(),
        method
    ));
    fs::write(&path, bytes)?;
    let response = if is_query {
        icp.canister_query_binary_args_output_with_candid(
            &canister.to_text(),
            method,
            &path,
            Some("json"),
            None,
        )
    } else {
        icp.canister_call_binary_args_output_with_candid(
            &canister.to_text(),
            method,
            &path,
            Some("json"),
            None,
        )
    };
    let cleanup = fs::remove_file(path);
    let response = response?;
    cleanup?;
    Ok(decode_json_result_response(&response)?)
}

fn parse_operation_id(value: &str) -> Result<[u8; 32], Box<dyn std::error::Error>> {
    let bytes =
        canic_core::cdk::utils::hash::decode_hex(value.strip_prefix("0x").unwrap_or(value))?;
    bytes
        .try_into()
        .map_err(|_| "operation-id-hex must contain exactly 32 bytes".into())
}
