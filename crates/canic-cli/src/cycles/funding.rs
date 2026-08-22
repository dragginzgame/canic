//! Module: cycles::funding
//!
//! Responsibility: render protected Coordinator/Root funding diagnostics for one installed Fleet.
//! Does not own: funding decisions, installed authority, or value-transfer mutation.
//! Boundary: verified host authority selects exact infrastructure targets and protocol bindings.

use crate::{
    cli::{
        clap::{flag_arg, parse_matches, render_usage, required_string, value_arg},
        globals::{internal_environment_arg, internal_icp_arg},
    },
    cycles::CyclesCommandError,
    support::icp_target::IcpTargetOptions,
};
use candid::{CandidType, decode_one};
use canic_core::{
    cdk::types::{Cycles, Principal},
    cdk::utils::hash::{decode_hex, hex_bytes},
    dto::{
        error::Error,
        fleet_funding::{FleetRootFundingRequest, FleetRootFundingResponse},
        fleet_registry::FleetSubnetRootStatus,
        icp_refill::{IcpRefillResponse, IcpRefillTrigger},
    },
    ids::{
        FleetCoordinatorRootFundingPolicy, FleetFundingProfile, FleetSubnetRootFundingPolicy,
        FleetSubnetRootIcpRefillPolicy,
    },
};
use canic_host::{
    format::cycles_tc,
    icp_config::resolve_current_canic_icp_root,
    installed_fleet::{
        InstalledFleetFundingResolution, InstalledFleetRequest,
        InstalledFleetRootFundingResolution, resolve_installed_fleet_funding_from_root,
    },
    protocol_binding::{ResolvedProtocolBinding, resolve_infrastructure_protocol_binding},
    release_set::{CanicInfrastructureRole, load_persisted_canic_infrastructure_artifact_manifest},
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
    Funding(RemoteCoordinatorFundingStatus),
}

#[derive(CandidType, Deserialize)]
enum RemoteRootStatusResponse {
    Funding(RemoteRootFundingStatus),
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
    automatic_grants: u32,
    automatic_cycles: Cycles,
    last_successful_grant_at_ns: Option<u64>,
    current_operation: Option<FleetRootFundingRequest>,
    last_result: Option<FleetRootFundingResponse>,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
struct RemoteCoordinatorFundingStatus {
    coordinator: Principal,
    current_cycles: Cycles,
    funding_enabled: bool,
    funding_profile: Option<FleetFundingProfile>,
    policy: Option<FleetCoordinatorRootFundingPolicy>,
    fleet_window: Option<RemoteFundingWindowStatus>,
    automatic_grants: u32,
    automatic_cycles: Cycles,
    roots: Vec<RemoteCoordinatorRootFundingStatus>,
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
    funding_profile: FleetFundingProfile,
    policy_hash: [u8; 32],
    root_policy: FleetSubnetRootFundingPolicy,
    current_operation: Option<FleetRootFundingRequest>,
    last_result: Option<FleetRootFundingResponse>,
    automatic_grants: u32,
    automatic_cycles: Cycles,
    icp_refill_policy: Option<FleetSubnetRootIcpRefillPolicy>,
    icp_window_start_secs: Option<u64>,
    icp_window_reserved_e8s: u64,
    automatic_icp_refills: u32,
    automatic_icp_refill_e8s: u64,
    latest_icp_refill: Option<RemoteRootIcpRefillStatus>,
}

#[derive(Clone, Debug, Serialize)]
struct PlacementReport {
    subnet: String,
    specialization: String,
    node_count: u64,
    cost_multiplier_numerator: u64,
    cost_multiplier_denominator: u64,
    acknowledge_fiduciary_cost: bool,
    warning: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct RootFundingReport {
    canister_id: Principal,
    placement: PlacementReport,
    status: RemoteRootFundingStatus,
    direct_topup_command: String,
}

#[derive(Clone, Debug, Serialize)]
struct FundingReport {
    fleet: String,
    environment: String,
    coordinator_canister_id: Principal,
    coordinator_placement: PlacementReport,
    coordinator: RemoteCoordinatorFundingStatus,
    coordinator_direct_topup_command: String,
    roots: Vec<RootFundingReport>,
}

pub(super) fn run(args: Vec<OsString>) -> Result<(), CyclesCommandError> {
    let options = FundingOptions::parse(args)?;
    let report = collect_report(&options)?;
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
        .about("Inspect protected Coordinator and Root funding headroom")
        .disable_help_flag(true)
        .arg(value_arg(FLEET_ARG).value_name(FLEET_ARG).required(true))
        .arg(flag_arg(JSON_ARG).long(JSON_ARG))
        .arg(internal_environment_arg())
        .arg(internal_icp_arg())
        .after_help("Examples:\n  canic cycles funding demo\n  canic cycles funding demo --json")
}

fn collect_report(options: &FundingOptions) -> Result<FundingReport, CyclesCommandError> {
    let root = resolve_current_canic_icp_root().map_err(CyclesCommandError::IcpRoot)?;
    let installed = resolve_installed_fleet_funding_from_root(
        &InstalledFleetRequest {
            fleet: options.fleet.clone(),
            environment: options.target.environment.clone(),
        },
        &root,
    )?;
    let manifest = load_persisted_canic_infrastructure_artifact_manifest(
        &root,
        installed.fleet.release_build_id,
    )
    .map_err(|error| CyclesCommandError::Usage(error.to_string()))?;
    let coordinator_binding = infrastructure_binding(
        &root,
        &options.target.environment,
        &manifest.manifest.entries,
        CanicInfrastructureRole::FleetCoordinator,
    )?;
    let root_binding = infrastructure_binding(
        &root,
        &options.target.environment,
        &manifest.manifest.entries,
        CanicInfrastructureRole::FleetSubnetRoot,
    )?;
    let icp = options.target.icp_cli(&root);
    let coordinator = query_coordinator(
        &icp,
        installed.coordinator_canister_id,
        &coordinator_binding,
    )?;
    validate_coordinator_status(&installed, &coordinator)?;
    let mut roots = Vec::new();
    for installed_root in installed
        .roots
        .iter()
        .filter(|root| root.status != FleetSubnetRootStatus::Removed)
    {
        let status = query_root(&icp, installed_root.fleet_subnet_root, &root_binding)?;
        if !root_status_matches_installed(&status, installed_root) {
            return Err(CyclesCommandError::FundingAuthority(
                "Root funding status conflicts with verified installed authority".to_string(),
            ));
        }
        roots.push(RootFundingReport {
            canister_id: installed_root.fleet_subnet_root,
            placement: placement_report(&installed_root.placement_cost),
            status,
            direct_topup_command: format!(
                "canic cycles topup {} {} <amount>",
                options.fleet, installed_root.fleet_subnet_root
            ),
        });
    }
    Ok(FundingReport {
        fleet: options.fleet.clone(),
        environment: options.target.environment.clone(),
        coordinator_canister_id: installed.coordinator_canister_id,
        coordinator_placement: placement_report(&installed.coordinator_placement_cost),
        coordinator,
        coordinator_direct_topup_command: format!(
            "canic cycles topup {} coordinator <amount>",
            options.fleet
        ),
        roots,
    })
}

fn infrastructure_binding(
    root: &std::path::Path,
    environment: &str,
    entries: &[canic_host::release_set::CanicInfrastructureArtifactEntry],
    role: CanicInfrastructureRole,
) -> Result<ResolvedProtocolBinding, CyclesCommandError> {
    let artifact = entries
        .iter()
        .find(|entry| entry.role == role)
        .ok_or_else(|| {
            CyclesCommandError::FundingAuthority(format!(
                "installed release is missing {} protocol metadata",
                role.as_str()
            ))
        })?;
    resolve_infrastructure_protocol_binding(root, environment, artifact)
        .map_err(|error| CyclesCommandError::FundingAuthority(error.to_string()))
}

fn query_coordinator(
    icp: &canic_host::icp::IcpCli,
    coordinator: Principal,
    binding: &ResolvedProtocolBinding,
) -> Result<RemoteCoordinatorFundingStatus, CyclesCommandError> {
    let output = icp.canister_query_arg_output_with_candid(
        &coordinator.to_text(),
        canic_core::protocol::CANIC_STATUS,
        "(variant { Funding })",
        Some("hex"),
        Some(binding.candid_path()),
    )?;
    let bytes = decode_hex(output.trim()).map_err(CyclesCommandError::FundingResponseHex)?;
    let response = decode_one::<Result<RemoteCoordinatorStatusResponse, Error>>(&bytes)
        .map_err(CyclesCommandError::FundingResponseCandid)?;
    match response {
        Ok(RemoteCoordinatorStatusResponse::Funding(status)) => Ok(status),
        Err(error) => Err(funding_rejected(error)),
    }
}

fn root_status_matches_installed(
    status: &RemoteRootFundingStatus,
    installed: &InstalledFleetRootFundingResolution,
) -> bool {
    let identity_is_exact = status.fleet_subnet_root == installed.fleet_subnet_root;
    let lifecycle_is_exact = status.lifecycle_status == installed.status;
    let funding_is_exact = status.root_policy == installed.funding.root_funding;
    let icp_refill_is_exact = status.icp_refill_policy == installed.funding.icp_refill;
    [
        identity_is_exact,
        lifecycle_is_exact,
        funding_is_exact,
        icp_refill_is_exact,
    ]
    .into_iter()
    .all(|is_exact| is_exact)
}

fn coordinator_root_status_matches_installed(
    status: &RemoteCoordinatorRootFundingStatus,
    installed: &InstalledFleetRootFundingResolution,
) -> bool {
    let identity_is_exact = status.fleet_subnet_root == installed.fleet_subnet_root;
    let lifecycle_is_exact = status.lifecycle_status == installed.status;
    let funding_is_exact = status.policy == installed.funding.root_funding;
    [identity_is_exact, lifecycle_is_exact, funding_is_exact]
        .into_iter()
        .all(|is_exact| is_exact)
}

fn query_root(
    icp: &canic_host::icp::IcpCli,
    root: Principal,
    binding: &ResolvedProtocolBinding,
) -> Result<RemoteRootFundingStatus, CyclesCommandError> {
    let output = icp.canister_query_arg_output_with_candid(
        &root.to_text(),
        canic_core::protocol::CANIC_STATUS,
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

fn funding_rejected(error: Error) -> CyclesCommandError {
    let code = error.code();
    CyclesCommandError::FundingRejected {
        code,
        diagnostic: canic_host::diagnostics::render_diagnostic(code),
    }
}

fn validate_coordinator_status(
    installed: &InstalledFleetFundingResolution,
    status: &RemoteCoordinatorFundingStatus,
) -> Result<(), CyclesCommandError> {
    if status.coordinator != installed.coordinator_canister_id
        || status.policy != installed.coordinator_root_funding
        || status.roots.len() != installed.roots.len()
        || installed.roots.iter().any(|root| {
            !status
                .roots
                .iter()
                .any(|entry| coordinator_root_status_matches_installed(entry, root))
        })
    {
        return Err(CyclesCommandError::FundingAuthority(
            "Coordinator funding status conflicts with verified installed authority".to_string(),
        ));
    }
    Ok(())
}

fn placement_report(
    placement: &canic_host::fleet_install_plan::PlannedSubnetPlacementCostEvidence,
) -> PlacementReport {
    PlacementReport {
        subnet: placement.subnet.to_string(),
        specialization: placement.subnet_specialization.clone(),
        node_count: placement.node_count,
        cost_multiplier_numerator: placement.cost_multiplier_numerator,
        cost_multiplier_denominator: placement.cost_multiplier_denominator,
        acknowledge_fiduciary_cost: placement.acknowledge_fiduciary_cost,
        warning: placement.warning.clone(),
    }
}

fn render_report(report: &FundingReport) -> String {
    let mut lines = vec![
        format!(
            "Fleet: {} (environment {})",
            report.fleet, report.environment
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
        format!(
            "  placement: subnet={} nodes={} multiplier={}/{} acknowledgement={}{}",
            report.coordinator_placement.subnet,
            report.coordinator_placement.node_count,
            report.coordinator_placement.cost_multiplier_numerator,
            report.coordinator_placement.cost_multiplier_denominator,
            report.coordinator_placement.acknowledge_fiduciary_cost,
            warning_suffix(report.coordinator_placement.warning.as_deref()),
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
        "  recovery: {}",
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
        "Root: {} | lifecycle={:?} | balance={} | funding={} | eligible={}",
        root.canister_id,
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
            |window| {
                format!(
                    "{}+{}/{}",
                    cycles_tc(window.spent_cycles.to_u128()),
                    cycles_tc(window.reserved_cycles.to_u128()),
                    cycles_tc(status.root_policy.budget.maximum_cycles.to_u128())
                )
            }
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
    lines.push(format!(
        "  placement: subnet={} nodes={} multiplier={}/{} acknowledgement={}{}",
        root.placement.subnet,
        root.placement.node_count,
        root.placement.cost_multiplier_numerator,
        root.placement.cost_multiplier_denominator,
        root.placement.acknowledge_fiduciary_cost,
        warning_suffix(root.placement.warning.as_deref()),
    ));
    lines.push(format!("  ICP fallback: {}", icp_refill_label(status)));
    lines.push(format!("  recovery: {}", root.direct_topup_command));
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
                    "/recovery_required"
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
        FleetFundingProfile::MultiSubnet => "multi_subnet",
    }
}

fn warning_suffix(warning: Option<&str>) -> String {
    warning.map_or_else(String::new, |warning| format!(" | {warning}"))
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
    fn funding_result_labels_are_explicit() {
        assert_eq!(funding_result_label(None), "none");
    }
}
