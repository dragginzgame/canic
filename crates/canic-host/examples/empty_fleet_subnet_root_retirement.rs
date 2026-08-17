use candid::{CandidType, Principal};
use canic_control_plane::dto::fleet_coordinator::{
    CoordinatorCommand, CoordinatorCommandResponse, CoordinatorStatusRequest,
    CoordinatorStatusResponse,
};
use canic_core::{
    cdk::utils::hash::{decode_hex, hex_bytes},
    dto::{
        fleet_registry::{FleetRegistry, FleetSubnetRootDrainingReservationRequest},
        role::OperationReceipt,
    },
    protocol,
};
use canic_host::{
    fleet_subnet_root_deletion::{
        FleetSubnetRootDeletionHostRequest, execute_fleet_subnet_root_deletion,
        prepare_fleet_subnet_root_deletion_execution,
    },
    icp::{IcpCli, decode_json_result_response},
};
use serde::de::DeserializeOwned;
use std::{env, fs, path::PathBuf};

const USAGE: &str = "usage: cargo run -p canic-host --example empty_fleet_subnet_root_retirement -- \
    --confirm-disposable-empty-root <icp-executable> <icp-root> <environment> \
    <coordinator-principal> <fleet-subnet-root-principal> <operation-id-hex>";

struct RetirementContext {
    icp_executable: String,
    icp_root: PathBuf,
    environment: String,
    coordinator: Principal,
    fleet_subnet_root: Principal,
    operation_id: [u8; 32],
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let context = parse_context()?;
    let icp = IcpCli::new(
        context.icp_executable.clone(),
        Some(context.environment.clone()),
    )
    .with_cwd(&context.icp_root);
    let registry = coordinator_registry(&icp, context.coordinator)?;
    let expected_root = registry
        .fleet_subnet_roots
        .iter()
        .find(|entry| entry.fleet_subnet_root == context.fleet_subnet_root)
        .cloned()
        .ok_or("Fleet Subnet Root is absent from the Coordinator Registry")?;
    let expected_registry = match query(
        &icp,
        context.coordinator,
        protocol::CANIC_STATUS,
        &CoordinatorStatusRequest::RegistryVersion,
    )? {
        CoordinatorStatusResponse::RegistryVersion(version) => version,
        _ => return Err("Coordinator returned a differently correlated status response".into()),
    };
    let response: CoordinatorCommandResponse = call(
        &icp,
        context.coordinator,
        protocol::CANIC_COMMAND,
        &CoordinatorCommand::RemoveRoot(FleetSubnetRootDrainingReservationRequest {
            operation_id: context.operation_id,
            expected_registry,
            expected_root,
        }),
    )?;
    let expected_receipt = OperationReceipt {
        operation_id: context.operation_id,
    };
    match response {
        CoordinatorCommandResponse::OperationAccepted(receipt) if receipt == expected_receipt => {}
        _ => return Err("Coordinator returned a differently correlated removal response".into()),
    }

    let host_request = || FleetSubnetRootDeletionHostRequest {
        icp_executable: &context.icp_executable,
        icp_root: &context.icp_root,
        environment: &context.environment,
        local_replica: None,
        coordinator: context.coordinator,
        fleet_subnet_root: context.fleet_subnet_root,
        operation_id: context.operation_id,
    };
    let execution = prepare_fleet_subnet_root_deletion_execution(host_request())?;
    let terminal = execute_fleet_subnet_root_deletion(host_request())?;
    println!(
        "removed Fleet Subnet Root {} with operation 0x{}; execution 0x{} completed at {}",
        terminal.fleet_subnet_root,
        hex_bytes(&terminal.operation_id),
        hex_bytes(&execution.execution_hash),
        terminal.completed_at_ns,
    );
    Ok(())
}

fn parse_context() -> Result<RetirementContext, Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let (
        Some(confirmation),
        Some(icp_executable),
        Some(icp_root),
        Some(environment),
        Some(coordinator),
        Some(fleet_subnet_root),
        Some(operation_id_hex),
    ) = (
        args.next(),
        args.next(),
        args.next(),
        args.next(),
        args.next(),
        args.next(),
        args.next(),
    )
    else {
        return Err(USAGE.into());
    };
    if confirmation != "--confirm-disposable-empty-root" || args.next().is_some() {
        return Err(USAGE.into());
    }
    let operation_id = decode_hex(
        operation_id_hex
            .strip_prefix("0x")
            .unwrap_or(&operation_id_hex),
    )?
    .try_into()
    .map_err(|_| "operation-id-hex must contain exactly 32 bytes")?;
    Ok(RetirementContext {
        icp_executable,
        icp_root: PathBuf::from(icp_root).canonicalize()?,
        environment,
        coordinator: Principal::from_text(coordinator)?,
        fleet_subnet_root: Principal::from_text(fleet_subnet_root)?,
        operation_id,
    })
}

fn coordinator_registry(
    icp: &IcpCli,
    coordinator: Principal,
) -> Result<FleetRegistry, Box<dyn std::error::Error>> {
    match query(
        icp,
        coordinator,
        protocol::CANIC_STATUS,
        &CoordinatorStatusRequest::Registry,
    )? {
        CoordinatorStatusResponse::Registry(registry) => Ok(registry),
        _ => Err("Coordinator returned a differently correlated status response".into()),
    }
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
