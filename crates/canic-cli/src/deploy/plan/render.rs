//! Module: canic_cli::deploy::plan::render
//!
//! Responsibility: render and persist deterministic deployment-plan reports.
//! Does not own: plan construction, diagnostic policy, or command parsing.
//! Boundary: consumes the assembled report and emits text or JSON without mutation.

use crate::deploy::{
    DeployCommandError,
    plan::{
        command::DeployPlanOptions,
        report::{DeploymentPlanReport, PlanDiagnostic, PlanStatus, ProposedOperationLabel},
    },
};

use std::path::Path;

use canic_host::{
    durable_io::create_new_bytes,
    fleet_install_input::{
        SubnetCatalogFailureCacheDispositionV1, SubnetCatalogFieldV1,
        SubnetCatalogLoadFailureEvidenceV1, SubnetCatalogLoadStageV1,
        SubnetCatalogRefreshTriggerV1, SubnetCatalogRegistryRecordEvidenceV1,
        SubnetCatalogRegistryRecordKindV1, SubnetCatalogRegistryValueEncodingV1,
        SubnetCatalogRetryabilityV1, SubnetCatalogSourceKindV1, SubnetCatalogSubjectV1,
        SubnetCatalogUnknownRetryReasonV1,
    },
    fleet_install_plan::{
        FreshFleetDeploymentPlanV1, FreshFleetFundingPayerV1, PlannedCanisterCreationFunding,
    },
};

pub(in crate::deploy) fn write_report(
    options: &DeployPlanOptions,
    report: &DeploymentPlanReport,
) -> Result<(), DeployCommandError> {
    if let Some(out) = &options.out {
        write_json_new(out, report)?;
    }

    if options.json {
        print_json(report)
    } else {
        println!("{}", render_text(report));
        Ok(())
    }
}

pub(in crate::deploy) fn command_exit_result(
    report: &DeploymentPlanReport,
) -> Result<(), DeployCommandError> {
    match report.status {
        PlanStatus::Planned | PlanStatus::Warning => Ok(()),
        PlanStatus::Blocked | PlanStatus::Unsupported => Err(DeployCommandError::PlanBlocked(
            report.status.as_str().to_string(),
        )),
    }
}

fn write_json_new(path: &Path, report: &DeploymentPlanReport) -> Result<(), DeployCommandError> {
    let mut data = render_json(report)?.into_bytes();
    data.push(b'\n');
    create_new_bytes(path, &data).map_err(plan_output_error)
}

fn print_json(report: &DeploymentPlanReport) -> Result<(), DeployCommandError> {
    let json = render_json(report)?;
    println!("{json}");
    Ok(())
}

pub(in crate::deploy) fn render_json(
    report: &DeploymentPlanReport,
) -> Result<String, DeployCommandError> {
    serde_json::to_string_pretty(report).map_err(plan_output_error)
}

fn plan_output_error(err: impl std::error::Error + 'static) -> DeployCommandError {
    DeployCommandError::PlanOutput(Box::new(err))
}

pub(in crate::deploy) fn render_text(report: &DeploymentPlanReport) -> String {
    let mut lines = vec![
        "Deployment plan".to_string(),
        format!("schema_version: {}", report.schema_version),
        format!("command: {}", report.command),
        format!("status: {}", report.status.as_str()),
        format!("comparison: {}", report.comparison_status.as_str()),
        format!("fleet: {}", report.fleet),
        format!("app: {}", report.app),
        format!("environment: {}", report.environment),
        format!("fleet_input: {}", report.fleet_input_path),
        format!("config: {}", report.config_path),
        format!("build_profile: {}", report.build_profile),
        format!(
            "release_build: {}",
            report.release_build_id.as_deref().unwrap_or("workspace")
        ),
        format!(
            "no_effects_started: {}",
            report
                .fresh_fleet_plan
                .as_ref()
                .is_some_and(|plan| plan.preflight.effects.no_effects_started())
                || report
                    .catalog_failure
                    .as_ref()
                    .is_some_and(catalog_failure_has_no_effects)
        ),
        String::new(),
    ];

    if let Some(plan) = &report.fresh_fleet_plan {
        append_fresh_fleet_decision(&mut lines, plan);
    }
    if let Some(failure) = &report.catalog_failure {
        append_catalog_failure(&mut lines, failure);
    }

    append_diagnostics(&mut lines, "blockers", &report.blockers);
    append_diagnostics(&mut lines, "warnings", &report.warnings);
    append_diagnostics(&mut lines, "assumptions", &report.assumptions);
    append_diagnostics(&mut lines, "verified facts", &report.verified_facts);
    append_operations(&mut lines, &report.proposed_operations);
    append_next_actions(&mut lines, &report.next_actions);

    lines.join("\n")
}

const fn catalog_failure_has_no_effects(failure: &SubnetCatalogLoadFailureEvidenceV1) -> bool {
    !failure.effects.build_started
        && !failure.effects.workspace_mutation_started
        && !failure.effects.ic_mutation_started
}

fn append_catalog_failure(lines: &mut Vec<String>, failure: &SubnetCatalogLoadFailureEvidenceV1) {
    let (cache_disposition, refresh_trigger) = catalog_cache_disposition(failure.cache_disposition);
    let (retryability, unknown_retry_reason) = catalog_retryability(failure.retryability);
    lines.push("catalog failure provenance".to_string());
    lines.push(format!("  schema_version: {}", failure.schema_version));
    lines.push(format!("  network: {}", failure.network));
    lines.push(format!(
        "  source_kind: {}",
        failure
            .source_kind
            .map_or("not_selected", catalog_source_kind)
    ));
    lines.push(format!(
        "  source_endpoints: {}",
        if failure.source_endpoints.is_empty() {
            "none".to_string()
        } else {
            failure.source_endpoints.join(",")
        }
    ));
    lines.push(format!(
        "  source_assurance: {}",
        failure
            .source_assurance
            .as_deref()
            .unwrap_or("not_selected")
    ));
    lines.push(format!(
        "  minimum_assurance: {}",
        failure.minimum_assurance
    ));
    lines.push(format!("  stage: {}", catalog_load_stage(failure.stage)));
    lines.push(format!(
        "  registry_version: {}",
        failure
            .registry_version
            .map_or_else(|| "unknown".to_string(), |version| version.to_string())
    ));
    lines.push(format!(
        "  returned_registry_value_version: {}",
        failure
            .returned_registry_value_version
            .map_or_else(|| "unknown".to_string(), |version| version.to_string())
    ));
    lines.push(format!(
        "  source_endpoint: {}",
        failure.source_endpoint.as_deref().unwrap_or("unknown")
    ));
    lines.push(format!(
        "  assurance: {}",
        failure.assurance.as_deref().unwrap_or("unknown")
    ));
    lines.push(format!(
        "  completed_registry_record_count: {}",
        failure.registry_records.len()
    ));
    for (index, record) in failure.registry_records.iter().enumerate() {
        append_catalog_registry_record(lines, index, record);
    }
    lines.push(format!("  cache_disposition: {cache_disposition}"));
    lines.push(format!(
        "  refresh_trigger: {}",
        refresh_trigger.unwrap_or("not_applicable")
    ));
    lines.push(format!(
        "  subject: {}",
        failure
            .subject
            .as_ref()
            .map_or_else(|| "unknown".to_string(), catalog_subject)
    ));
    lines.push(format!("  code: {}", failure.code));
    lines.push(format!("  category: {}", failure.category));
    lines.push(format!("  retryability: {retryability}"));
    lines.push(format!(
        "  unknown_retry_reason: {}",
        unknown_retry_reason.unwrap_or("not_applicable")
    ));
    lines.push(format!(
        "  effects: build_started={} workspace_mutation_started={} ic_mutation_started={}",
        failure.effects.build_started,
        failure.effects.workspace_mutation_started,
        failure.effects.ic_mutation_started,
    ));
    lines.push(format!("  source_message: {}", failure.source_message));
    lines.push(String::new());
}

const fn catalog_source_kind(value: SubnetCatalogSourceKindV1) -> &'static str {
    match value {
        SubnetCatalogSourceKindV1::UncertifiedQuery => "uncertified_query",
        SubnetCatalogSourceKindV1::MultiEndpointAgreement => "multi_endpoint_agreement",
    }
}

const fn catalog_load_stage(value: SubnetCatalogLoadStageV1) -> &'static str {
    match value {
        SubnetCatalogLoadStageV1::RequestValidation => "request_validation",
        SubnetCatalogLoadStageV1::CacheOnlyLoad => "cache_only_load",
        SubnetCatalogLoadStageV1::CacheLookup => "cache_lookup",
        SubnetCatalogLoadStageV1::CacheAbsence => "cache_absence",
        SubnetCatalogLoadStageV1::CacheRejection => "cache_rejection",
        SubnetCatalogLoadStageV1::CacheBypass => "cache_bypass",
        SubnetCatalogLoadStageV1::RefreshAttempted => "refresh_attempted",
        SubnetCatalogLoadStageV1::RefreshFailed => "refresh_failed",
        SubnetCatalogLoadStageV1::PostRefreshCacheLoadFailed => "post_refresh_cache_load_failed",
        SubnetCatalogLoadStageV1::RuntimeAdapter => "runtime_adapter",
    }
}

const fn catalog_cache_disposition(
    value: SubnetCatalogFailureCacheDispositionV1,
) -> (&'static str, Option<&'static str>) {
    match value {
        SubnetCatalogFailureCacheDispositionV1::NotExamined => ("not_examined", None),
        SubnetCatalogFailureCacheDispositionV1::CacheOnly => ("cache_only", None),
        SubnetCatalogFailureCacheDispositionV1::CacheBypassed => ("cache_bypassed", None),
        SubnetCatalogFailureCacheDispositionV1::CacheMissing => ("cache_missing", None),
        SubnetCatalogFailureCacheDispositionV1::CacheRejected => ("cache_rejected", None),
        SubnetCatalogFailureCacheDispositionV1::CacheReadFailed => ("cache_read_failed", None),
        SubnetCatalogFailureCacheDispositionV1::RefreshAttempted { trigger } => {
            ("refresh_attempted", Some(catalog_refresh_trigger(trigger)))
        }
        SubnetCatalogFailureCacheDispositionV1::RefreshFailed { trigger } => {
            ("refresh_failed", Some(catalog_refresh_trigger(trigger)))
        }
        SubnetCatalogFailureCacheDispositionV1::PostRefreshLoadFailed { trigger } => (
            "post_refresh_load_failed",
            Some(catalog_refresh_trigger(trigger)),
        ),
    }
}

const fn catalog_refresh_trigger(value: SubnetCatalogRefreshTriggerV1) -> &'static str {
    match value {
        SubnetCatalogRefreshTriggerV1::Missing => "missing",
        SubnetCatalogRefreshTriggerV1::Rejected => "rejected",
        SubnetCatalogRefreshTriggerV1::Stale => "stale",
        SubnetCatalogRefreshTriggerV1::Forced => "forced",
    }
}

fn catalog_subject(value: &SubnetCatalogSubjectV1) -> String {
    match value {
        SubnetCatalogSubjectV1::Network { network } => format!("network={network:?}"),
        SubnetCatalogSubjectV1::Endpoint { endpoint } => format!("endpoint={endpoint:?}"),
        SubnetCatalogSubjectV1::CachePath { path } => format!("cache_path={path:?}"),
        SubnetCatalogSubjectV1::RegistryLatestVersion => "registry_latest_version".to_string(),
        SubnetCatalogSubjectV1::RegistryRecord {
            record_kind,
            key,
            subnet,
            canister_range_start,
        } => format!(
            "registry_record kind={} key={key:?} subnet={} canister_range_start={}",
            catalog_record_kind(*record_kind),
            subnet.as_deref().unwrap_or("not_applicable"),
            canister_range_start.as_deref().unwrap_or("not_applicable")
        ),
        SubnetCatalogSubjectV1::Subnet { subnet, field } => format!(
            "subnet={subnet} field={}",
            field.map_or("not_narrowed", catalog_field)
        ),
        SubnetCatalogSubjectV1::RegistryRoutingTableEntry { index, field } => format!(
            "registry_routing_table_entry={index} field={}",
            field.map_or("not_narrowed", catalog_field)
        ),
        SubnetCatalogSubjectV1::RoutingRange {
            start_canister_id,
            end_canister_id,
            subnet_principal,
            field,
        } => format!(
            "routing_range start={start_canister_id} end={end_canister_id} subnet={subnet_principal} field={}",
            field.map_or("not_narrowed", catalog_field)
        ),
        SubnetCatalogSubjectV1::Field { field } => {
            format!("field={}", catalog_field(*field))
        }
    }
}

fn append_catalog_registry_record(
    lines: &mut Vec<String>,
    index: usize,
    record: &SubnetCatalogRegistryRecordEvidenceV1,
) {
    lines.push(format!(
        "  completed_registry_record[{index}]: kind={} key={:?} subnet={} canister_range_start={} requested_registry_version={} returned_registry_version={} timestamp_nanoseconds={} source_endpoint={:?} assurance={} value_encoding={}",
        catalog_record_kind(record.record_kind),
        record.key,
        record.subnet.as_deref().unwrap_or("not_applicable"),
        record
            .canister_range_start
            .as_deref()
            .unwrap_or("not_applicable"),
        record.requested_registry_version,
        record.returned_registry_version,
        record.timestamp_nanoseconds,
        record.source_endpoint,
        record.assurance,
        catalog_registry_value_encoding(record.value_encoding),
    ));
}

const fn catalog_record_kind(value: SubnetCatalogRegistryRecordKindV1) -> &'static str {
    match value {
        SubnetCatalogRegistryRecordKindV1::SubnetList => "subnet_list",
        SubnetCatalogRegistryRecordKindV1::RoutingTable => "routing_table",
        SubnetCatalogRegistryRecordKindV1::SubnetRecord => "subnet_record",
    }
}

const fn catalog_registry_value_encoding(
    value: SubnetCatalogRegistryValueEncodingV1,
) -> &'static str {
    match value {
        SubnetCatalogRegistryValueEncodingV1::Inline => "inline",
        SubnetCatalogRegistryValueEncodingV1::Chunked => "chunked",
    }
}

const fn catalog_field(value: SubnetCatalogFieldV1) -> &'static str {
    match value {
        SubnetCatalogFieldV1::SubnetListEntry => "subnet_list_entry",
        SubnetCatalogFieldV1::RoutingTableRange => "routing_table_range",
        SubnetCatalogFieldV1::RoutingTableSubnetId => "routing_table_subnet_id",
        SubnetCatalogFieldV1::RoutingRangeStart => "routing_range_start",
        SubnetCatalogFieldV1::RoutingRangeEnd => "routing_range_end",
        SubnetCatalogFieldV1::Network => "network",
        SubnetCatalogFieldV1::RegistryCanister => "registry_canister",
        SubnetCatalogFieldV1::RegistryVersion => "registry_version",
        SubnetCatalogFieldV1::SourceEndpoint => "source_endpoint",
        SubnetCatalogFieldV1::SubnetPrincipal => "subnet_principal",
        SubnetCatalogFieldV1::CollectionTimestamp => "collection_timestamp",
        SubnetCatalogFieldV1::Classification => "classification",
        SubnetCatalogFieldV1::AgreementDigest => "agreement_digest",
        SubnetCatalogFieldV1::CatalogDigest => "catalog_digest",
        SubnetCatalogFieldV1::Provenance => "provenance",
    }
}

const fn catalog_retryability(
    value: SubnetCatalogRetryabilityV1,
) -> (&'static str, Option<&'static str>) {
    match value {
        SubnetCatalogRetryabilityV1::Retryable => ("retryable", None),
        SubnetCatalogRetryabilityV1::NotRetryable => ("not_retryable", None),
        SubnetCatalogRetryabilityV1::Unknown { reason } => {
            ("unknown", Some(catalog_unknown_retry_reason(reason)))
        }
    }
}

const fn catalog_unknown_retry_reason(value: SubnetCatalogUnknownRetryReasonV1) -> &'static str {
    match value {
        SubnetCatalogUnknownRetryReasonV1::CacheOperation => "cache_operation",
        SubnetCatalogUnknownRetryReasonV1::RegistryResponse => "registry_response",
        SubnetCatalogUnknownRetryReasonV1::RegistryTransport => "registry_transport",
        SubnetCatalogUnknownRetryReasonV1::RuntimeAdapter => "runtime_adapter",
    }
}

fn append_fresh_fleet_decision(lines: &mut Vec<String>, plan: &FreshFleetDeploymentPlanV1) {
    let counts = plan.counts;
    let operator = &plan.authority.operator;
    lines.push("canonical fresh-Fleet decision".to_string());
    lines.push(format!("  plan_digest: {}", plan.plan_digest));
    lines.push(format!("  operator_principal: {}", operator.principal));
    lines.push(format!(
        "  operator_funding_account: {}",
        operator.funding_account
    ));
    lines.push(format!(
        "  operator_balance: {}",
        render_funding(&operator.balance)
    ));
    lines.push(format!(
        "  operator_balance_evidence: source={} observed_at={} valid_until={} fresh={} sufficient={}",
        operator.source,
        operator.observed_at_unix_secs,
        operator.valid_until_unix_secs,
        operator.balance_fresh,
        plan.operator_balance_sufficient,
    ));
    lines.push(format!(
        "  maximum_operator_debit: {}",
        render_funding(&plan.maximum_operator_debit)
    ));
    lines.push(format!(
        "  canister_counts: coordinator={} root={} store={} component={} ready_pool={} role={} total={}",
        counts.coordinator_canisters,
        counts.root_canisters,
        counts.wasm_store_canisters,
        counts.component_canisters,
        counts.ready_pool_canisters,
        counts.role_canisters,
        counts.total_canisters,
    ));
    for root in &plan.preflight.fleet_subnet_roots {
        lines.push(format!(
            "  root: subnet={} component={} initial_pool={} pool_creations={} ready_pool={} admissions={}",
            root.placement_subnet,
            root.initial_component_canisters,
            root.initial_pool_canisters,
            root.pool_canister_creations,
            root.remaining_pool_canisters,
            root.component_admissions.len(),
        ));
    }
    for requirement in &plan.funding_requirements {
        let payer = match requirement.payer {
            FreshFleetFundingPayerV1::Operator => "operator",
            FreshFleetFundingPayerV1::FleetSubnetRoot => "fleet_subnet_root",
        };
        lines.push(format!(
            "  funding: category={} owner={} payer={} count={} per_canister={} maximum={}",
            requirement.category,
            requirement.owner,
            payer,
            requirement.canister_count,
            render_funding(&requirement.per_canister),
            render_funding(&requirement.maximum),
        ));
    }
    lines.push(String::new());
}

fn render_funding(funding: &PlannedCanisterCreationFunding) -> String {
    match funding {
        PlannedCanisterCreationFunding::Cycles { cycles } => format!("{cycles} cycles"),
        PlannedCanisterCreationFunding::Icp { e8s } => format!("{e8s} ICP e8s"),
    }
}

fn append_diagnostics(lines: &mut Vec<String>, label: &str, diagnostics: &[PlanDiagnostic]) {
    if diagnostics.is_empty() {
        return;
    }

    lines.push(label.to_string());
    for diagnostic in diagnostics {
        lines.push(format!(
            "  [{}] {} {}",
            diagnostic.severity.label(),
            diagnostic.category.label(),
            diagnostic.code
        ));
        lines.push(format!("    subject: {}", diagnostic.subject));
        lines.push(format!("    detail: {}", diagnostic.detail));
        lines.push(format!("    source: {}", diagnostic.source.label()));
        if let Some(next) = &diagnostic.next {
            lines.push(format!("    next: {next}"));
        }
    }
    lines.push(String::new());
}

fn append_operations(lines: &mut Vec<String>, operations: &[ProposedOperationLabel]) {
    if operations.is_empty() {
        return;
    }

    lines.push("future apply preview (proposed operation labels; not executed)".to_string());
    for operation in operations {
        lines.push(format!(
            "  - phase: {} label: {} subject: {} status: {}",
            operation.phase.label(),
            operation.label.label(),
            operation.subject,
            operation.status.label()
        ));
    }
    lines.push(String::new());
}

fn append_next_actions(lines: &mut Vec<String>, actions: &[String]) {
    if actions.is_empty() {
        return;
    }

    lines.push("next actions".to_string());
    for action in actions {
        lines.push(format!("  - {action}"));
    }
}
