use candid::Principal;
use canic_core::cdk::utils::hash::decode_hex;
use canic_host::fleet_subnet_root_deletion::{
    FleetSubnetRootDeletionHostRequest, execute_fleet_subnet_root_deletion,
    prepare_fleet_subnet_root_deletion_execution,
};
use std::{env, path::PathBuf};

const USAGE: &str = "usage: cargo run -p canic-host --example fleet_subnet_root_deletion -- \
    <prepare|execute> --confirm-disposable-root-deletion \
    <icp-executable> <icp-root> <environment> \
    <coordinator-principal> <fleet-subnet-root-principal> <operation-id-hex>";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let (Some(command), Some(confirmation), Some(icp_executable), Some(icp_root)) =
        (args.next(), args.next(), args.next(), args.next())
    else {
        return Err(USAGE.into());
    };
    if confirmation != "--confirm-disposable-root-deletion" {
        return Err(USAGE.into());
    }
    let (Some(environment), Some(coordinator), Some(fleet_subnet_root), Some(operation_id)) =
        (args.next(), args.next(), args.next(), args.next())
    else {
        return Err(USAGE.into());
    };
    if args.next().is_some() {
        return Err(USAGE.into());
    }

    let icp_root = PathBuf::from(icp_root).canonicalize()?;
    let coordinator = Principal::from_text(coordinator)?;
    let fleet_subnet_root = Principal::from_text(fleet_subnet_root)?;
    let operation_id = parse_operation_id(&operation_id)?;
    let request = FleetSubnetRootDeletionHostRequest {
        icp_executable: &icp_executable,
        icp_root: &icp_root,
        environment: &environment,
        local_replica: None,
        coordinator,
        fleet_subnet_root,
        operation_id,
    };

    let output = match command.as_str() {
        "prepare" => serde_json::to_value(prepare_fleet_subnet_root_deletion_execution(request)?)?,
        "execute" => serde_json::to_value(execute_fleet_subnet_root_deletion(request)?)?,
        _ => return Err(USAGE.into()),
    };
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn parse_operation_id(value: &str) -> Result<[u8; 32], Box<dyn std::error::Error>> {
    let bytes = decode_hex(value.strip_prefix("0x").unwrap_or(value))?;
    bytes
        .try_into()
        .map_err(|_| "operation-id-hex must contain exactly 32 bytes".into())
}
