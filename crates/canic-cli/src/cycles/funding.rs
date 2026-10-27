//! Module: cycles::funding
//!
//! Responsibility: render protected Coordinator/Root funding diagnostics for one current Fleet.
//! Does not own: funding decisions, policy rotation, or value-transfer mutation.
//! Boundary: terminal ensure inventory selects exact infrastructure targets and installed artifacts.

use crate::{
    cli::{
        clap::{flag_arg, parse_matches, render_usage, required_string, value_arg},
        globals::{internal_environment_arg, internal_icp_arg},
    },
    cycles::CyclesCommandError,
    support::icp_target::IcpTargetOptions,
};
use canic_contracts::dto::{
    error::Error,
    fleet_coordinator::CoordinatorFundingWindowStatusResponse as RemoteFundingWindowStatus,
};
use canic_contracts::{
    dto::{
        fleet_coordinator::{
            CoordinatorFundingStatusResponse, CoordinatorRootFundingStatusResponse,
        },
        fleet_funding::FleetRootFundingResponse,
        fleet_registry::{FleetRegistry, FleetSubnetRootEntry},
        root::RootFundingStatusResponse,
    },
    ids::{FleetFundingProfile, FleetSubnetRootFundingAuthority, FleetSubnetRootFundingPolicy},
};
use canic_core::{
    cdk::{types::Principal, utils::hash::hex_bytes},
    shared_support::fleet_funding_policy::fleet_subnet_root_funding_policy_hash,
};
use canic_host::{
    fleet_ensure::{CurrentFleetResolution, resolve_current_fleet},
    format::cycles_tc,
    icp_config::resolve_current_canic_icp_root,
    protocol_binding::resolve_current_registry_wire_schema,
};
use clap::Command as ClapCommand;
use serde::Serialize;
use std::ffi::OsString;

const FLEET_ARG: &str = "fleet";
const JSON_ARG: &str = "json";

#[derive(Clone, Debug, Eq, PartialEq)]
struct FundingOptions {
    target: IcpTargetOptions,
    fleet: String,
    json: bool,
}

#[derive(Clone, Debug, Serialize)]
struct RootFundingReport {
    canister_id: Principal,
    placement_subnet: String,
    status: RootFundingStatusResponse,
    direct_topup_command: String,
}

#[derive(Clone, Debug, Serialize)]
struct FundingReport {
    fleet: String,
    environment: String,
    source: &'static str,
    coordinator_canister_id: Principal,
    coordinator: CoordinatorFundingStatusResponse,
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
    require_current_schema(&current, &root, coordinator, &options.fleet)?;
    let icp = options.target.icp_cli(&root);
    let live_registry = query_registry(&icp, coordinator)?;
    validate_registry_authority(&current, initial_registry, &live_registry, coordinator)?;
    let coordinator_status = query_coordinator(&icp, coordinator)?;
    validate_coordinator_status(&live_registry, &coordinator_status, coordinator)?;

    let mut roots = Vec::with_capacity(live_registry.fleet_subnet_roots.len());
    for entry in &live_registry.fleet_subnet_roots {
        require_current_schema(&current, &root, entry.fleet_subnet_root, &options.fleet)?;
        let status = query_root(&icp, entry.fleet_subnet_root)?;
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

fn require_current_schema(
    current: &CurrentFleetResolution,
    root: &std::path::Path,
    principal: Principal,
    fleet: &str,
) -> Result<(), CyclesCommandError> {
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
    resolve_current_registry_wire_schema(root, current, entry)
        .map(|_| ())
        .map_err(|error| funding_authority_error(error.to_string()))
}

fn query_registry(
    icp: &canic_host::icp::IcpCli,
    coordinator: Principal,
) -> Result<FleetRegistry, CyclesCommandError> {
    Ok(canic_host::registry_discovery::snapshot(icp, coordinator)?)
}

fn query_coordinator(
    icp: &canic_host::icp::IcpCli,
    coordinator: Principal,
) -> Result<CoordinatorFundingStatusResponse, CyclesCommandError> {
    match canic_host::funding_read::coordinator_status(icp, coordinator) {
        Ok(status) => Ok(status),
        Err(canic_host::CanisterProtocolError::Response {
            source: canic_host::icp::IcpJsonResponseError::Rejected(error),
            ..
        }) => Err(funding_rejected(error)),
        Err(error) => Err(error.into()),
    }
}

fn query_root(
    icp: &canic_host::icp::IcpCli,
    root: Principal,
) -> Result<RootFundingStatusResponse, CyclesCommandError> {
    match canic_host::funding_read::root_status(icp, root) {
        Ok(status) => Ok(status),
        Err(canic_host::CanisterProtocolError::Response {
            source: canic_host::icp::IcpJsonResponseError::Rejected(error),
            ..
        }) => Err(funding_rejected(error)),
        Err(error) => Err(error.into()),
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
    status: &CoordinatorFundingStatusResponse,
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
    status: &CoordinatorRootFundingStatusResponse,
) -> bool {
    let identity_matches = status.fleet_subnet_root == entry.fleet_subnet_root;
    let lifecycle_matches = status.lifecycle_status == entry.status;
    identity_matches && lifecycle_matches
}

fn coordinator_policy_matches_registry(
    entry: &FleetSubnetRootEntry,
    status: &CoordinatorRootFundingStatusResponse,
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
    status: &RootFundingStatusResponse,
    coordinator: &CoordinatorFundingStatusResponse,
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

fn icp_refill_label(status: &RootFundingStatusResponse) -> String {
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
    fn funding_registry_query_requires_explicit_network_authority() {
        use canic_host::{
            CanisterProtocolError,
            icp::{IcpCli, IcpManagementCallError, IcpQueryError},
        };
        let coordinator = Principal::from_slice(&[2]);
        let icp = IcpCli::new("/missing/query-authority-icp", None);
        assert!(matches!(
            query_registry(&icp, coordinator),
            Err(CyclesCommandError::FundingProtocol(CanisterProtocolError::ReadOnlyQuery {
                canister,
                method: canic_contracts::protocol::registry_discovery::snapshot::METHOD,
                source: IcpQueryError::Authority(source),
            })) if canister == coordinator
                && matches!(*source, IcpManagementCallError::MissingEnvironment)
        ));
    }

    #[cfg(unix)]
    #[test]
    fn root_funding_query_preserves_rejection_and_exact_wire_identity() {
        use canic_contracts::{diagnostics::codes, protocol::funding_read as wire};
        let root = crate::test_support::temp_dir("funding-root-query");
        let icp = registry_query_fixture(&root);
        let canister = Principal::from_slice(&[1]);
        let rejected = Error::from_registered(codes::PLATFORM_UNAVAILABLE);
        let reply: Result<canic_contracts::dto::funding_read::RootStatus, Error> = Err(rejected);
        let bytes = ic_wire::native::encode_response(
            wire::root_status::METHOD_ID,
            &reply,
            wire::RESPONSE_LIMITS,
        )
        .unwrap();
        std::fs::write(root.join("response.json"), hex_bytes(bytes)).unwrap();
        assert!(
            matches!(canic_host::funding_read::root_status(&icp, canister),
            Err(canic_host::CanisterProtocolError::Response {
                canister: actual, method: wire::root_status::METHOD,
                source: canic_host::icp::IcpJsonResponseError::Rejected(error),
            }) if actual == canister && error == rejected)
        );
        let request = std::fs::read(root.join("request.bin")).unwrap();
        ic_wire::native::decode_request::<()>(
            wire::root_status::METHOD_ID,
            &request,
            wire::REQUEST_LIMITS,
        )
        .unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    fn registry_query_fixture(root: &std::path::Path) -> canic_host::icp::IcpCli {
        use std::{fs, os::unix::fs::PermissionsExt};
        fs::create_dir_all(root).unwrap();
        let version = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../ci/ic-tools.tsv"
        ))
        .lines()
        .find_map(|line| {
            line.strip_prefix("icp\t")
                .and_then(|row| row.split('\t').next())
        })
        .unwrap();
        let executable = root.join("icp");
        fs::write(&executable, format!(
            "#!/bin/sh\nset -eu\nif [ \"$1\" = --version ]; then echo 'icp {version}'; exit 0; fi\nwhile [ \"$#\" -gt 0 ]; do\n  if [ \"$1\" = --args-file ]; then shift; cp \"$1\" request.bin; fi\n  shift\ndone\ncat response.json\n",
        )).unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        canic_host::icp::IcpCli::new(executable.to_str().unwrap(), None).with_cwd(root)
    }

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
