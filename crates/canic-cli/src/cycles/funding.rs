//! Module: cycles::funding
//!
//! Responsibility: render protected Coordinator/Root funding diagnostics for one current Fleet.
//! Does not own: funding decisions, policy rotation, or value-transfer mutation.
//! Boundary: terminal ensure inventory selects exact infrastructure targets and Candid bindings.

use crate::{
    cli::{
        clap::{flag_arg, parse_matches, render_usage, required_string, value_arg},
        globals::{internal_environment_arg, internal_icp_arg},
    },
    cycles::CyclesCommandError,
    support::icp_target::IcpTargetOptions,
};
use candid::{CandidType, Reserved, decode_one};
use canic_core::{
    cdk::{
        types::{Cycles, Principal},
        utils::hash::{decode_hex, hex_bytes},
    },
    dto::{
        error::Error,
        fleet_funding::{FleetRootFundingRequest, FleetRootFundingResponse},
        fleet_registry::{FleetRegistry, FleetSubnetRootEntry, FleetSubnetRootStatus},
        icp_refill::{IcpRefillResponse, IcpRefillTrigger},
    },
    ids::{
        FleetCoordinatorRootFundingPolicy, FleetFundingProfile, FleetSubnetRootFundingAuthority,
        FleetSubnetRootFundingPolicy, FleetSubnetRootIcpRefillPolicy,
    },
    shared_support::fleet_funding_policy::fleet_subnet_root_funding_policy_hash,
};
use canic_host::{
    fleet_ensure::{CurrentFleetResolution, resolve_current_fleet},
    format::cycles_tc,
    icp_config::resolve_current_canic_icp_root,
    protocol_binding::{ResolvedProtocolBinding, resolve_registry_protocol_binding},
};
use clap::Command as ClapCommand;
use serde::{Deserialize, Serialize};
use std::ffi::OsString;

const FLEET_ARG: &str = "fleet";
const JSON_ARG: &str = "json";

#[derive(Clone, Debug, Eq, PartialEq)]
struct FundingOptions {
    target: IcpTargetOptions,
    fleet: String,
    json: bool,
}

#[derive(CandidType, Deserialize)]
enum RemoteCoordinatorStatusResponse {
    Funding(Box<RemoteCoordinatorFundingStatus>),
    Registry(Box<FleetRegistry>),
}

#[derive(CandidType)]
enum RemoteCoordinatorStatusRequest {
    Registry,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
struct RemoteFundingWindowStatus {
    window_start_secs: u64,
    spent_cycles: Cycles,
    reserved_cycles: Cycles,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
struct RemoteCoordinatorRootFundingStatus {
    fleet_subnet_root: Principal,
    lifecycle_status: FleetSubnetRootStatus,
    policy_hash: [u8; 32],
    policy: FleetSubnetRootFundingPolicy,
    window: RemoteFundingWindowStatus,
    historical_automatic_grants: u64,
    historical_automatic_cycles: Cycles,
    automatic_grants: u32,
    automatic_cycles: Cycles,
    last_successful_grant_at_ns: Option<u64>,
    current_operation: Option<FleetRootFundingRequest>,
    last_result: Option<FleetRootFundingResponse>,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
enum RemoteFundingPolicyRotationPhase {
    ActivatingRoots {
        activated_root_count: u32,
        expected_root_count: u32,
        successor_registry: canic_core::dto::fleet_registry::FleetRegistryVersion,
    },
    Completed(Reserved),
    PreparingRoots {
        prepared_root_count: u32,
        expected_root_count: u32,
    },
    Staging {
        staged_root_count: u32,
        expected_root_count: u32,
    },
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
struct RemoteFundingPolicyRotationStatus {
    operation_id: [u8; 32],
    plan_digest: [u8; 32],
    predecessor_generation: u64,
    successor_generation: u64,
    phase: RemoteFundingPolicyRotationPhase,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
struct RemoteCoordinatorFundingStatus {
    coordinator: Principal,
    current_cycles: Cycles,
    policy_generation: u64,
    funding_enabled: bool,
    funding_profile: Option<FleetFundingProfile>,
    policy: Option<FleetCoordinatorRootFundingPolicy>,
    fleet_window: Option<RemoteFundingWindowStatus>,
    historical_automatic_grants: u64,
    historical_automatic_cycles: Cycles,
    automatic_grants: u32,
    automatic_cycles: Cycles,
    rotation_checkpoint_count: u32,
    rotation_checkpoint_root_count: u32,
    rotation_checkpoint_root_capacity_remaining: u32,
    rotation: Option<RemoteFundingPolicyRotationStatus>,
    roots: Vec<RemoteCoordinatorRootFundingStatus>,
}

#[derive(CandidType, Deserialize)]
enum RemoteRootStatusResponse {
    Funding(RemoteRootFundingStatus),
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
struct RemoteRootIcpRefillStatus {
    trigger: IcpRefillTrigger,
    amount_e8s: u64,
    fee_e8s: u64,
    budget_window_start_secs: u64,
    resumable: bool,
    response: IcpRefillResponse,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
struct RemoteRootFundingStatus {
    fleet_subnet_root: Principal,
    lifecycle_status: FleetSubnetRootStatus,
    funding_eligible: bool,
    cycles_funding_enabled: bool,
    current_cycles: Cycles,
    policy_generation: u64,
    funding_profile: FleetFundingProfile,
    policy_hash: [u8; 32],
    root_policy: FleetSubnetRootFundingPolicy,
    current_operation: Option<FleetRootFundingRequest>,
    last_result: Option<FleetRootFundingResponse>,
    historical_automatic_grants: u64,
    historical_automatic_cycles: Cycles,
    automatic_grants: u32,
    automatic_cycles: Cycles,
    rotation_current: Option<Reserved>,
    rotation_last: Option<Reserved>,
    icp_refill_policy: Option<FleetSubnetRootIcpRefillPolicy>,
    icp_window_start_secs: Option<u64>,
    icp_window_reserved_e8s: u64,
    automatic_icp_refills: u32,
    automatic_icp_refill_e8s: u64,
    latest_icp_refill: Option<RemoteRootIcpRefillStatus>,
}

#[derive(Clone, Debug, Serialize)]
struct RootFundingReport {
    canister_id: Principal,
    placement_subnet: String,
    status: RemoteRootFundingStatus,
    direct_topup_command: String,
}

#[derive(Clone, Debug, Serialize)]
struct FundingReport {
    fleet: String,
    environment: String,
    source: &'static str,
    coordinator_canister_id: Principal,
    coordinator: RemoteCoordinatorFundingStatus,
    coordinator_direct_topup_command: String,
    roots: Vec<RootFundingReport>,
}

pub(super) fn run(args: Vec<OsString>) -> Result<(), CyclesCommandError> {
    let options = FundingOptions::parse(args)?;
    let report = collect_funding(&options)?;
    if options.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", render_report(&report));
    }
    Ok(())
}

pub(super) fn usage() -> String {
    render_usage(command)
}

impl FundingOptions {
    fn parse<I>(args: I) -> Result<Self, CyclesCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(command(), args).map_err(|_| CyclesCommandError::Usage(usage()))?;
        Ok(Self {
            target: IcpTargetOptions::parse(&matches),
            fleet: required_string(&matches, FLEET_ARG),
            json: matches.get_flag(JSON_ARG),
        })
    }
}

fn command() -> ClapCommand {
    ClapCommand::new("funding")
        .bin_name("canic cycles funding")
        .about("Inspect protected current Coordinator and Root funding headroom")
        .disable_help_flag(true)
        .arg(value_arg(FLEET_ARG).value_name(FLEET_ARG).required(true))
        .arg(flag_arg(JSON_ARG).long(JSON_ARG).help("Print JSON output"))
        .arg(internal_environment_arg())
        .arg(internal_icp_arg())
        .after_help("Examples:\n  canic cycles funding demo\n  canic cycles funding demo --json")
}

fn collect_funding(options: &FundingOptions) -> Result<FundingReport, CyclesCommandError> {
    let root = resolve_current_canic_icp_root().map_err(CyclesCommandError::IcpRoot)?;
    let current = resolve_current_fleet(&root, &options.target.environment, &options.fleet)?;
    let initial_registry = current.initial_active_registry(&options.fleet)?;
    let coordinator = initial_registry.authority.binding.coordinator;
    let coordinator_binding = current_binding(
        &current,
        &root,
        &options.target.environment,
        coordinator,
        &options.fleet,
    )?;
    let icp = options.target.icp_cli(&root);
    let live_registry = query_registry(&icp, coordinator, &coordinator_binding)?;
    validate_registry_authority(&current, initial_registry, &live_registry, coordinator)?;
    let coordinator_status = query_coordinator(&icp, coordinator, &coordinator_binding)?;
    validate_coordinator_status(&live_registry, &coordinator_status, coordinator)?;

    let mut roots = Vec::with_capacity(live_registry.fleet_subnet_roots.len());
    for entry in &live_registry.fleet_subnet_roots {
        let binding = current_binding(
            &current,
            &root,
            &options.target.environment,
            entry.fleet_subnet_root,
            &options.fleet,
        )?;
        let status = query_root(&icp, entry.fleet_subnet_root, &binding)?;
        validate_root_status(entry, &status, &coordinator_status)?;
        roots.push(RootFundingReport {
            canister_id: entry.fleet_subnet_root,
            placement_subnet: entry.placement_subnet.to_string(),
            status,
            direct_topup_command: format!(
                "canic cycles topup {} {} <amount>",
                options.fleet, entry.fleet_subnet_root
            ),
        });
    }
    roots.sort_by_key(|entry| entry.canister_id);

    Ok(FundingReport {
        fleet: options.fleet.clone(),
        environment: options.target.environment.clone(),
        source: "current_ensure_inventory",
        coordinator_canister_id: coordinator,
        coordinator: coordinator_status,
        coordinator_direct_topup_command: format!(
            "canic cycles topup {} coordinator <amount>",
            options.fleet
        ),
        roots,
    })
}

fn current_binding(
    current: &CurrentFleetResolution,
    root: &std::path::Path,
    environment: &str,
    principal: Principal,
    fleet: &str,
) -> Result<ResolvedProtocolBinding, CyclesCommandError> {
    let principal = principal.to_text();
    let entry = current
        .registry
        .entries
        .iter()
        .find(|entry| entry.pid == principal)
        .ok_or_else(|| {
            funding_authority_error(format!(
                "Fleet {fleet} omits current infrastructure participant {principal}"
            ))
        })?;
    resolve_registry_protocol_binding(root, environment, entry)
        .map_err(|error| funding_authority_error(error.to_string()))
}

fn query_registry(
    icp: &canic_host::icp::IcpCli,
    coordinator: Principal,
    binding: &ResolvedProtocolBinding,
) -> Result<FleetRegistry, CyclesCommandError> {
    let response: Result<RemoteCoordinatorStatusResponse, Error> =
        canic_host::query_canister_with_arg(
            icp,
            binding,
            coordinator,
            canic_core::protocol::CANIC_COORDINATOR_STATUS,
            &RemoteCoordinatorStatusRequest::Registry,
        )?;
    match response {
        Ok(RemoteCoordinatorStatusResponse::Registry(registry)) => Ok(*registry),
        Ok(RemoteCoordinatorStatusResponse::Funding(_)) => Err(funding_authority_error(
            "Coordinator returned the wrong protected Registry status variant",
        )),
        Err(error) => Err(funding_rejected(error)),
    }
}

fn query_coordinator(
    icp: &canic_host::icp::IcpCli,
    coordinator: Principal,
    binding: &ResolvedProtocolBinding,
) -> Result<RemoteCoordinatorFundingStatus, CyclesCommandError> {
    let output = icp.canister_query_arg_output_with_candid(
        &coordinator.to_text(),
        canic_core::protocol::CANIC_COORDINATOR_STATUS,
        "(variant { Funding })",
        Some("hex"),
        Some(binding.candid_path()),
    )?;
    let bytes = decode_hex(output.trim()).map_err(CyclesCommandError::FundingResponseHex)?;
    let response = decode_one::<Result<RemoteCoordinatorStatusResponse, Error>>(&bytes)
        .map_err(CyclesCommandError::FundingResponseCandid)?;
    match response {
        Ok(RemoteCoordinatorStatusResponse::Funding(status)) => Ok(*status),
        Ok(RemoteCoordinatorStatusResponse::Registry(_)) => Err(funding_authority_error(
            "Coordinator returned the wrong protected funding status variant",
        )),
        Err(error) => Err(funding_rejected(error)),
    }
}

fn query_root(
    icp: &canic_host::icp::IcpCli,
    root: Principal,
    binding: &ResolvedProtocolBinding,
) -> Result<RemoteRootFundingStatus, CyclesCommandError> {
    let output = icp.canister_query_arg_output_with_candid(
        &root.to_text(),
        canic_core::protocol::CANIC_ROOT_STATUS,
        "(variant { Funding })",
        Some("hex"),
        Some(binding.candid_path()),
    )?;
    let bytes = decode_hex(output.trim()).map_err(CyclesCommandError::FundingResponseHex)?;
    let response = decode_one::<Result<RemoteRootStatusResponse, Error>>(&bytes)
        .map_err(CyclesCommandError::FundingResponseCandid)?;
    match response {
        Ok(RemoteRootStatusResponse::Funding(status)) => Ok(status),
        Err(error) => Err(funding_rejected(error)),
    }
}

fn validate_registry_authority(
    current: &CurrentFleetResolution,
    initial: &FleetRegistry,
    live: &FleetRegistry,
    coordinator: Principal,
) -> Result<(), CyclesCommandError> {
    let mut expected_roots = current.topology.fleet_subnet_root_canister_ids.clone();
    expected_roots.sort_unstable();
    let mut live_roots = live
        .fleet_subnet_roots
        .iter()
        .map(|entry| entry.fleet_subnet_root.to_text())
        .collect::<Vec<_>>();
    live_roots.sort_unstable();
    let exact = live.authority == initial.authority
        && live.authority.binding.coordinator == coordinator
        && live.revision >= initial.revision
        && live_roots == expected_roots;
    if exact {
        Ok(())
    } else {
        Err(funding_authority_error(
            "live Registry authority or Root set conflicts with terminal current ensure authority",
        ))
    }
}

fn validate_coordinator_status(
    registry: &FleetRegistry,
    status: &RemoteCoordinatorFundingStatus,
    coordinator: Principal,
) -> Result<(), CyclesCommandError> {
    let roots_are_exact = registry.fleet_subnet_roots.iter().all(|entry| {
        status.roots.iter().any(|root| {
            coordinator_root_identity_matches_registry(entry, root)
                && coordinator_policy_matches_registry(entry, root, status.rotation.is_some())
        })
    });
    let exact = status.coordinator == coordinator
        && status.policy_generation != 0
        && status.roots.len() == registry.fleet_subnet_roots.len()
        && roots_are_exact;
    if exact {
        Ok(())
    } else {
        Err(funding_authority_error(
            "Coordinator funding status conflicts with the exact current Registry authority",
        ))
    }
}

fn coordinator_root_identity_matches_registry(
    entry: &FleetSubnetRootEntry,
    status: &RemoteCoordinatorRootFundingStatus,
) -> bool {
    let identity_matches = status.fleet_subnet_root == entry.fleet_subnet_root;
    let lifecycle_matches = status.lifecycle_status == entry.status;
    identity_matches && lifecycle_matches
}

fn coordinator_policy_matches_registry(
    entry: &FleetSubnetRootEntry,
    status: &RemoteCoordinatorRootFundingStatus,
    rotation_active: bool,
) -> bool {
    if rotation_active {
        return status.policy_hash == funding_authority_hash(&entry.funding, status.policy.clone());
    }
    status.policy == entry.funding.root_funding
        && status.policy_hash == fleet_subnet_root_funding_policy_hash(&entry.funding)
}

fn validate_root_status(
    entry: &FleetSubnetRootEntry,
    status: &RemoteRootFundingStatus,
    coordinator: &RemoteCoordinatorFundingStatus,
) -> Result<(), CyclesCommandError> {
    let Some(coordinator_root) = coordinator
        .roots
        .iter()
        .find(|root| root.fleet_subnet_root == entry.fleet_subnet_root)
    else {
        return Err(funding_authority_error(
            "Coordinator omits one exact current Root funding ledger",
        ));
    };
    let generation_is_admitted = coordinator.rotation.as_ref().map_or_else(
        || status.policy_generation == coordinator.policy_generation,
        |rotation| {
            status.policy_generation == rotation.predecessor_generation
                || status.policy_generation == rotation.successor_generation
        },
    );
    let local_policy_matches_coordinator = status.root_policy == coordinator_root.policy;
    let local_hash_matches_coordinator = status.policy_hash == coordinator_root.policy_hash;
    let local_matches_coordinator =
        local_policy_matches_coordinator && local_hash_matches_coordinator;
    let registry_policy_matches = coordinator.rotation.is_some()
        || (status.root_policy == entry.funding.root_funding
            && status.policy_hash == fleet_subnet_root_funding_policy_hash(&entry.funding));
    let root_identity_matches = status.fleet_subnet_root == entry.fleet_subnet_root;
    let lifecycle_matches = status.lifecycle_status == entry.status;
    let icp_refill_matches = status.icp_refill_policy == entry.funding.icp_refill;
    let exact = root_identity_matches
        && lifecycle_matches
        && icp_refill_matches
        && generation_is_admitted
        && local_matches_coordinator
        && registry_policy_matches;
    if exact {
        Ok(())
    } else {
        Err(funding_authority_error(
            "Root funding status conflicts with the exact current Registry and Coordinator authority",
        ))
    }
}

fn funding_authority_hash(
    authority: &FleetSubnetRootFundingAuthority,
    root_funding: FleetSubnetRootFundingPolicy,
) -> [u8; 32] {
    let mut authority = authority.clone();
    authority.root_funding = root_funding;
    fleet_subnet_root_funding_policy_hash(&authority)
}

fn funding_authority_error(message: impl Into<String>) -> CyclesCommandError {
    CyclesCommandError::FundingAuthority(message.into())
}

fn funding_rejected(error: Error) -> CyclesCommandError {
    let code = error.code();
    CyclesCommandError::FundingRejected {
        code,
        diagnostic: canic_host::diagnostics::render_diagnostic(code),
    }
}

fn render_report(report: &FundingReport) -> String {
    let mut lines = vec![
        format!(
            "Fleet: {} (environment {}, source {})",
            report.fleet, report.environment, report.source
        ),
        format!(
            "Coordinator: {} | balance={} | funding={} | profile={}",
            report.coordinator_canister_id,
            cycles_tc(report.coordinator.current_cycles.to_u128()),
            if report.coordinator.funding_enabled {
                "enabled"
            } else {
                "disabled"
            },
            report
                .coordinator
                .funding_profile
                .map_or("none", funding_profile_label),
        ),
    ];
    if let (Some(policy), Some(window)) = (
        report.coordinator.policy.as_ref(),
        report.coordinator.fleet_window.as_ref(),
    ) {
        lines.push(format!(
            "  treasury: reserve={} window={}+{}/{} auto={}/{} grants, {}/{} cycles",
            cycles_tc(policy.minimum_reserve_cycles.to_u128()),
            cycles_tc(window.spent_cycles.to_u128()),
            cycles_tc(window.reserved_cycles.to_u128()),
            cycles_tc(policy.budget.maximum_cycles.to_u128()),
            report.coordinator.automatic_grants,
            policy.maximum_automatic_grants,
            cycles_tc(report.coordinator.automatic_cycles.to_u128()),
            cycles_tc(policy.maximum_automatic_cycles.to_u128()),
        ));
    }
    lines.push(format!(
        "  direct top-up: {}",
        report.coordinator_direct_topup_command
    ));
    for root in &report.roots {
        append_root_report(&mut lines, report, root);
    }
    lines.join("\n")
}

fn append_root_report(lines: &mut Vec<String>, report: &FundingReport, root: &RootFundingReport) {
    let status = &root.status;
    lines.push(String::new());
    lines.push(format!(
        "Root: {} | subnet={} | lifecycle={:?} | balance={} | funding={} | eligible={}",
        root.canister_id,
        root.placement_subnet,
        status.lifecycle_status,
        cycles_tc(status.current_cycles.to_u128()),
        if status.cycles_funding_enabled {
            "enabled"
        } else {
            "disabled"
        },
        status.funding_eligible,
    ));
    lines.push(format!(
        "  policy: request<={} target={} window={} auto={}/{} grants, {}/{} cycles",
        cycles_tc(status.root_policy.request_threshold.to_u128()),
        cycles_tc(status.root_policy.target_balance.to_u128()),
        coordinator_root_window(report, root).map_or_else(
            || "unavailable".to_string(),
            |window| format!(
                "{}+{}/{}",
                cycles_tc(window.spent_cycles.to_u128()),
                cycles_tc(window.reserved_cycles.to_u128()),
                cycles_tc(status.root_policy.budget.maximum_cycles.to_u128())
            )
        ),
        status.automatic_grants,
        status.root_policy.maximum_automatic_grants,
        cycles_tc(status.automatic_cycles.to_u128()),
        cycles_tc(status.root_policy.maximum_automatic_cycles.to_u128()),
    ));
    lines.push(format!(
        "  last Coordinator result: {} | pending={}",
        funding_result_label(status.last_result.as_ref()),
        status.current_operation.is_some(),
    ));
    lines.push(format!("  ICP fallback: {}", icp_refill_label(status)));
    lines.push(format!("  direct top-up: {}", root.direct_topup_command));
}

fn coordinator_root_window<'a>(
    report: &'a FundingReport,
    root: &RootFundingReport,
) -> Option<&'a RemoteFundingWindowStatus> {
    report
        .coordinator
        .roots
        .iter()
        .find(|status| status.fleet_subnet_root == root.canister_id)
        .map(|status| &status.window)
}

fn funding_result_label(result: Option<&FleetRootFundingResponse>) -> String {
    match result {
        Some(FleetRootFundingResponse::Granted(receipt)) => format!(
            "granted {} at {}",
            cycles_tc(receipt.request.granted_cycles.to_u128()),
            receipt.accepted_at_ns
        ),
        Some(FleetRootFundingResponse::NoGrant(receipt)) => {
            format!("no_grant {:?} at {}", receipt.reason, receipt.decided_at_ns)
        }
        None => "none".to_string(),
    }
}

fn icp_refill_label(status: &RemoteRootFundingStatus) -> String {
    let Some(policy) = status.icp_refill_policy.as_ref() else {
        return "not configured".to_string();
    };
    let automatic = policy.automatic.as_ref().map_or_else(
        || "manual only".to_string(),
        |automatic| {
            format!(
                "automatic<={} target={} used={}/{} refills, {}/{} e8s",
                cycles_tc(automatic.emergency_threshold.to_u128()),
                cycles_tc(automatic.target_balance.to_u128()),
                status.automatic_icp_refills,
                automatic.maximum_automatic_refills,
                status.automatic_icp_refill_e8s,
                automatic.maximum_automatic_refill_e8s,
            )
        },
    );
    let latest = status.latest_icp_refill.as_ref().map_or_else(
        || "none".to_string(),
        |latest| {
            format!(
                "{:?}/{}{}",
                latest.trigger,
                hex_bytes(latest.response.operation_id),
                if latest.resumable {
                    "/resume_required"
                } else {
                    ""
                }
            )
        },
    );
    format!(
        "{automatic}; window={}/{} e8s; last={latest}",
        status.icp_window_reserved_e8s, policy.maximum_refill_e8s
    )
}

const fn funding_profile_label(profile: FleetFundingProfile) -> &'static str {
    match profile {
        FleetFundingProfile::SingleSubnet => "single_subnet",
        FleetFundingProfile::PreviewMultiSubnet => "preview_multi_subnet",
        FleetFundingProfile::MultiSubnet => "multi_subnet",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn funding_options_require_one_fleet_and_preserve_target_context() {
        let options = FundingOptions::parse([
            OsString::from("demo"),
            OsString::from("--json"),
            OsString::from(crate::cli::globals::INTERNAL_ENVIRONMENT_OPTION),
            OsString::from("staging"),
        ])
        .expect("parse funding options");

        assert_eq!(options.fleet, "demo");
        assert_eq!(options.target.environment, "staging");
        assert!(options.json);
        assert!(FundingOptions::parse(std::iter::empty::<OsString>()).is_err());
    }

    #[test]
    fn removed_rotation_flags_are_not_accepted_as_hidden_compatibility() {
        assert!(
            FundingOptions::parse([
                OsString::from("demo"),
                OsString::from("--plan-rotation"),
                OsString::from("funding.json"),
            ])
            .is_err()
        );
        assert!(
            FundingOptions::parse([
                OsString::from("demo"),
                OsString::from("--apply-rotation"),
                OsString::from("funding.json"),
            ])
            .is_err()
        );
    }

    #[test]
    fn funding_result_labels_are_explicit() {
        assert_eq!(funding_result_label(None), "none");
    }
}
