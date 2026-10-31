//! Module: metrics::transport
//!
//! Responsibility: collect typed metric observations for current Fleet canisters.
//! Does not own: metric DTOs, report rendering, or Fleet registry authority.
//! Boundary: preserves query causes until projecting per-canister report diagnostics.

use crate::{
    metrics::{
        MetricsCommandError,
        model::{
            MetricValue, MetricsCanisterReport, MetricsCanisterStatus, MetricsKind, MetricsPage,
            MetricsReport,
        },
        options::MetricsOptions,
        parse::metric_page,
    },
    support::parallel::collect_ordered_reports,
};
use canic_contracts::dto::observability::{
    CanisterObservabilityRequest, CanisterObservabilityResponse,
};
use canic_host::{
    CanisterProtocolError,
    fleet_ensure::{CurrentFleetResolution, resolve_current_fleet},
    icp::{IcpCli, IcpDiagnostic, IcpWireQueryError},
    icp_config::resolve_current_canic_icp_root,
    observability::{FleetObservabilityError, observe_fleet_canister},
    registry::RegistryEntry,
};
use std::path::Path;
use thiserror::Error as ThisError;

const METRICS_UNAVAILABLE_HINT: &str =
    "role-owned Metrics status unavailable; check deployed Wasm and metrics profile";
const METRICS_EMPTY_HINT: &str =
    "no metrics rows; check whether this tier is enabled by the deployed role profile";
const METRICS_NONZERO_EMPTY_HINT: &str = "no nonzero rows among fetched metrics; rerun without --nonzero or check the deployed role profile";
const METRICS_WORKER_PANIC: &str = "metrics query worker panicked";

#[derive(Debug, ThisError)]
enum MetricsQueryError {
    #[error(transparent)]
    Observability(#[from] FleetObservabilityError),
}

pub(super) fn metrics_report(
    options: &MetricsOptions,
) -> Result<MetricsReport, MetricsCommandError> {
    let root = resolve_current_canic_icp_root().map_err(MetricsCommandError::IcpRoot)?;
    let fleet = load_registry(options)?;
    let canisters = collect_metrics_reports(options, &fleet, &root);

    Ok(MetricsReport {
        fleet: options.fleet.clone(),
        environment: options.environment.clone(),
        kind: options.kind,
        canisters,
    })
}

fn load_registry(options: &MetricsOptions) -> Result<CurrentFleetResolution, MetricsCommandError> {
    resolve_metrics_fleet(options)
}

fn matches_metrics_filter(options: &MetricsOptions, entry: &RegistryEntry) -> bool {
    if let Some(role) = &options.role
        && entry.role.as_deref() != Some(role.as_str())
    {
        return false;
    }
    if let Some(canister) = &options.canister
        && entry.pid != *canister
    {
        return false;
    }
    true
}

fn collect_metrics_reports(
    options: &MetricsOptions,
    fleet: &CurrentFleetResolution,
    icp_root: &Path,
) -> Vec<MetricsCanisterReport> {
    let entries = fleet
        .registry
        .entries
        .iter()
        .filter(|entry| matches_metrics_filter(options, entry))
        .collect::<Vec<_>>();
    collect_metrics_entry_reports(&entries, |entry| {
        metrics_canister_report(options, fleet, icp_root, entry)
    })
}

fn collect_metrics_entry_reports(
    entries: &[&RegistryEntry],
    query: impl Fn(&RegistryEntry) -> MetricsCanisterReport + Sync,
) -> Vec<MetricsCanisterReport> {
    collect_ordered_reports(
        entries,
        |entry| query(entry),
        |entry| metrics_message_error_report(entry, METRICS_WORKER_PANIC),
    )
}

fn metrics_canister_report(
    options: &MetricsOptions,
    fleet: &CurrentFleetResolution,
    icp_root: &Path,
    entry: &RegistryEntry,
) -> MetricsCanisterReport {
    match query_metrics(options, fleet, icp_root, entry) {
        Ok(page) => metrics_success_report(entry, page, options.nonzero),
        Err(error) => metrics_query_error_report(entry, &error),
    }
}

fn metrics_success_report(
    entry: &RegistryEntry,
    page: MetricsPage,
    nonzero: bool,
) -> MetricsCanisterReport {
    let mut entries = page.entries;
    let fetched = entries.len() as u64;
    if nonzero {
        entries.retain(|entry| !metric_value_is_zero(&entry.value));
    }
    let empty = entries.is_empty();
    MetricsCanisterReport {
        role: entry.role.clone().unwrap_or_else(|| "-".to_string()),
        canister_id: entry.pid.clone(),
        status: if empty {
            MetricsCanisterStatus::Empty
        } else {
            MetricsCanisterStatus::Ok
        },
        total: Some(page.total),
        fetched,
        truncated: fetched < page.total,
        entries,
        error: empty.then(|| {
            if nonzero {
                METRICS_NONZERO_EMPTY_HINT
            } else {
                METRICS_EMPTY_HINT
            }
            .to_string()
        }),
    }
}

fn metrics_query_error_report(
    entry: &RegistryEntry,
    error: &MetricsQueryError,
) -> MetricsCanisterReport {
    if matches!(
        error,
        MetricsQueryError::Observability(FleetObservabilityError::Protocol(
            CanisterProtocolError::WireUpdate { source: IcpWireQueryError::Icp(source), .. }
                | CanisterProtocolError::WireQuery { source: IcpWireQueryError::Icp(source), .. },
        )) if matches!(source.diagnostic(), Some(IcpDiagnostic::MethodMissing))
    ) {
        return metrics_failure_report(
            entry,
            MetricsCanisterStatus::Unavailable,
            METRICS_UNAVAILABLE_HINT,
        );
    }

    let error = error.to_string();
    metrics_failure_report(
        entry,
        MetricsCanisterStatus::Error,
        error.lines().next().unwrap_or(&error),
    )
}

fn metrics_message_error_report(entry: &RegistryEntry, error: &str) -> MetricsCanisterReport {
    metrics_failure_report(
        entry,
        MetricsCanisterStatus::Error,
        error.lines().next().unwrap_or(error),
    )
}

fn metrics_failure_report(
    entry: &RegistryEntry,
    status: MetricsCanisterStatus,
    error: &str,
) -> MetricsCanisterReport {
    MetricsCanisterReport {
        role: entry.role.clone().unwrap_or_else(|| "-".to_string()),
        canister_id: entry.pid.clone(),
        status,
        total: None,
        fetched: 0,
        truncated: false,
        entries: Vec::new(),
        error: Some(error.to_string()),
    }
}

const fn metric_value_is_zero(value: &MetricValue) -> bool {
    match value {
        MetricValue::Count { count, exact } => *count == 0 && *exact,
        MetricValue::Measurement {
            count,
            total,
            count_overflowed,
            total_overflowed,
        } => *count == 0 && *total == 0 && !*count_overflowed && !*total_overflowed,
        MetricValue::U128 { value, exact } => *value == 0 && *exact,
    }
}

const fn metrics_kind_dto(kind: MetricsKind) -> canic_contracts::dto::metrics::MetricsKind {
    match kind {
        MetricsKind::Core => canic_contracts::dto::metrics::MetricsKind::Core,
        MetricsKind::Placement => canic_contracts::dto::metrics::MetricsKind::Placement,
        MetricsKind::Platform => canic_contracts::dto::metrics::MetricsKind::Platform,
        MetricsKind::Runtime => canic_contracts::dto::metrics::MetricsKind::Runtime,
        MetricsKind::Security => canic_contracts::dto::metrics::MetricsKind::Security,
        MetricsKind::Storage => canic_contracts::dto::metrics::MetricsKind::Storage,
    }
}

fn query_metrics(
    options: &MetricsOptions,
    fleet: &CurrentFleetResolution,
    icp_root: &Path,
    entry: &RegistryEntry,
) -> Result<MetricsPage, MetricsQueryError> {
    let mut icp = IcpCli::new(&options.icp, Some(options.environment.clone()));
    icp = icp.with_cwd(icp_root);
    collect_metric_pages(options.limit, |page| {
        let request = canic_contracts::dto::role::MetricsStatusRequest {
            kind: metrics_kind_dto(options.kind),
            page,
        };
        let response = observe_fleet_canister(
            &icp,
            icp_root,
            fleet,
            entry,
            CanisterObservabilityRequest::Metrics(request),
        )?;
        let CanisterObservabilityResponse::Metrics(page) = response else {
            unreachable!("Metrics request returned a different observability response");
        };
        Ok(metric_page(page))
    })
}

/// Bound total fetched rows by the selected limit, including server-clamped pages.
fn collect_metric_pages(
    limit: u64,
    mut query: impl FnMut(
        canic_contracts::dto::page::PageRequest,
    ) -> Result<MetricsPage, MetricsQueryError>,
) -> Result<MetricsPage, MetricsQueryError> {
    let mut entries = Vec::new();
    loop {
        let offset = entries.len() as u64;
        let remaining = limit.saturating_sub(offset);
        let mut page = query(canic_contracts::dto::page::PageRequest {
            offset,
            limit: remaining,
        })?;
        let received = page.entries.len() as u64;
        page.entries
            .truncate(usize::try_from(remaining).unwrap_or(usize::MAX));
        entries.extend(page.entries);
        let fetched = entries.len() as u64;
        if received == 0 || fetched >= limit || fetched >= page.total {
            return Ok(MetricsPage {
                entries,
                total: page.total,
            });
        }
    }
}

fn resolve_metrics_fleet(
    options: &MetricsOptions,
) -> Result<CurrentFleetResolution, MetricsCommandError> {
    let root = resolve_current_canic_icp_root().map_err(MetricsCommandError::IcpRoot)?;
    resolve_current_fleet(&root, &options.environment, &options.fleet)
        .map_err(MetricsCommandError::from)
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use canic_host::icp::{IcpCommandError, IcpJsonResponseError};

    fn registry_entry() -> RegistryEntry {
        RegistryEntry {
            pid: "aaaaa-aa".to_string(),
            role: Some("wasm_store".to_string()),
            parent_pid: None,
            module_hash: None,
            protocol_binding: None,
        }
    }

    // Ensure method-missing responses do not stretch the table with raw ICP output.
    #[test]
    fn shortens_metrics_unavailable_errors() {
        let error = MetricsQueryError::Observability(FleetObservabilityError::Protocol(
            CanisterProtocolError::WireUpdate {
                canister: ic_platform::principal::ExternalPrincipal::management_canister(),
                method: canic_contracts::protocol::inspection::METHOD,
                source: IcpWireQueryError::Icp(IcpCommandError::Failed {
                    command: "icp canister call".to_string(),
                    stderr: "Canister has no query method 'canic_coordinator_funding'.".to_string(),
                }),
            },
        ));
        let report = metrics_query_error_report(&registry_entry(), &error);

        assert_eq!(report.status, MetricsCanisterStatus::Unavailable);
        assert_eq!(
            serde_json::to_value(&report).expect("serialize metrics report")["status"],
            "unavailable"
        );
        assert_eq!(report.error.as_deref(), Some(METRICS_UNAVAILABLE_HINT));
    }

    // Ensure empty successful metric tiers point operators at profile/deployed-Wasm checks.
    #[test]
    fn empty_metrics_reports_carry_profile_hint() {
        let report = metrics_success_report(
            &registry_entry(),
            MetricsPage {
                entries: Vec::new(),
                total: 0,
            },
            false,
        );

        assert_eq!(report.status, MetricsCanisterStatus::Empty);
        assert_eq!(
            serde_json::to_value(&report).expect("serialize metrics report")["status"],
            "empty"
        );
        assert_eq!(report.error.as_deref(), Some(METRICS_EMPTY_HINT));

        let filtered = metrics_success_report(
            &registry_entry(),
            MetricsPage {
                entries: Vec::new(),
                total: 0,
            },
            true,
        );
        assert_eq!(filtered.status, MetricsCanisterStatus::Empty);
        assert_eq!(filtered.error.as_deref(), Some(METRICS_NONZERO_EMPTY_HINT));
    }

    // Ensure zero filtering treats every payload shape consistently.
    #[test]
    fn detects_zero_metric_values() {
        assert!(metric_value_is_zero(&MetricValue::Count {
            count: 0,
            exact: true
        }));
        assert!(metric_value_is_zero(&MetricValue::Measurement {
            count: 0,
            total: 0,
            count_overflowed: false,
            total_overflowed: false,
        }));
        assert!(!metric_value_is_zero(&MetricValue::U128 {
            value: 1,
            exact: true
        }));
    }

    #[test]
    fn nonzero_filter_retains_unknown_scalar_quality() {
        for value in [
            MetricValue::Count {
                count: 0,
                exact: false,
            },
            MetricValue::U128 {
                value: 0,
                exact: false,
            },
            MetricValue::Count {
                count: u128::MAX,
                exact: true,
            },
        ] {
            assert!(!metric_value_is_zero(&value));
        }
        assert!(metric_value_is_zero(&MetricValue::U128 {
            value: 0,
            exact: true
        }));
    }

    // Ensure transport preserves the Candid metric kind vocabulary.
    #[test]
    fn maps_metric_kind_to_observability_dto() {
        assert!(matches!(
            metrics_kind_dto(MetricsKind::Security),
            canic_contracts::dto::metrics::MetricsKind::Security
        ));
    }

    #[test]
    fn panicked_metrics_worker_becomes_an_explicit_canister_error() {
        let entry = registry_entry();
        let reports =
            collect_metrics_entry_reports(&[&entry], |_| panic!("simulated metrics worker panic"));

        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].canister_id, entry.pid);
        assert_eq!(reports[0].status, MetricsCanisterStatus::Error);
        assert_eq!(reports[0].error.as_deref(), Some(METRICS_WORKER_PANIC));
    }

    #[test]
    fn metrics_response_failure_preserves_typed_cause_until_projection() {
        let error = MetricsQueryError::Observability(FleetObservabilityError::Protocol(
            CanisterProtocolError::Response {
                canister: ic_platform::principal::ExternalPrincipal::management_canister(),
                method: canic_contracts::protocol::inspection::METHOD,
                source: IcpJsonResponseError::Rejected(
                    canic_contracts::dto::error::Error::from_registered(
                        canic_contracts::diagnostics::codes::AUTHORITY_UNAUTHORIZED,
                    ),
                ),
            },
        ));
        let mut source = std::error::Error::source(&error);
        let mut preserved = false;
        while let Some(cause) = source {
            if matches!(
                cause.downcast_ref::<IcpJsonResponseError>(),
                Some(IcpJsonResponseError::Rejected(error))
                    if error.code() == canic_contracts::diagnostics::codes::AUTHORITY_UNAUTHORIZED.raw_code()
            ) {
                preserved = true;
                break;
            }
            source = cause.source();
        }

        assert!(preserved, "typed response cause must remain in the chain");
    }

    fn count_page(counts: &[u64], total: u64) -> MetricsPage {
        MetricsPage {
            entries: counts
                .iter()
                .map(|&count| crate::metrics::model::MetricEntry {
                    labels: vec!["sample".into()],
                    principal: None,
                    value: MetricValue::Count {
                        count: u128::from(count),
                        exact: true,
                    },
                })
                .collect(),
            total,
        }
    }

    #[test]
    fn metrics_pagination_follows_short_pages_within_the_selected_cap() {
        for limit in [3, 5, 10] {
            let mut requests = Vec::new();
            let page = collect_metric_pages(limit, |request| {
                requests.push(request);
                let remaining = 5_u64.saturating_sub(request.offset);
                let count = request.limit.min(remaining).min(2);
                Ok(count_page(
                    &vec![1; usize::try_from(count).expect("small count")],
                    5,
                ))
            })
            .expect("collect pages");
            assert_eq!(page.total, 5);
            assert_eq!(page.entries.len() as u64, limit.min(5));
            assert_eq!(requests[0].offset, 0);
            for (index, request) in requests.iter().enumerate() {
                assert_eq!(request.offset, index as u64 * 2);
                assert_eq!(request.limit, limit - request.offset);
            }
            let report = metrics_success_report(&registry_entry(), page, false);
            assert_eq!(report.truncated, limit < 5);
            assert_eq!(report.fetched, limit.min(5));
        }
    }

    #[test]
    fn nonzero_filter_preserves_pre_filter_coverage_in_json() {
        let report = metrics_success_report(&registry_entry(), count_page(&[0, 0], 1500), true);
        assert_eq!(report.status, MetricsCanisterStatus::Empty);
        assert!(report.entries.is_empty());
        let json = serde_json::to_value(report).expect("serialize report");
        assert_eq!(json["total"], 1500);
        assert_eq!(json["fetched"], 2);
        assert_eq!(json["truncated"], true);

        let complete = metrics_success_report(&registry_entry(), count_page(&[0, 7], 2), true);
        assert_eq!(complete.entries.len(), 1);
        assert_eq!(complete.fetched, 2);
        assert!(!complete.truncated);
        let failure = metrics_message_error_report(&registry_entry(), "unavailable");
        assert_eq!(failure.total, None);
        assert_eq!(failure.fetched, 0);
        assert!(!failure.truncated);
    }

    #[test]
    fn nonzero_rows_after_the_first_page_are_collected() {
        let page = collect_metric_pages(3, |request| {
            Ok(if request.offset == 0 {
                count_page(&[0, 0], 3)
            } else {
                count_page(&[7], 3)
            })
        })
        .expect("collect nonzero row from the later page");
        let report = metrics_success_report(&registry_entry(), page, true);
        assert_eq!(report.status, MetricsCanisterStatus::Ok);
        assert_eq!(report.entries.len(), 1);
        assert_eq!(
            report.entries[0].value,
            MetricValue::Count {
                count: 7,
                exact: true
            }
        );
        assert_eq!(report.fetched, 3);
        assert!(!report.truncated);
    }

    #[test]
    fn metrics_pagination_stops_on_empty_pages_and_keeps_latest_total() {
        let mut calls = 0;
        let page = collect_metric_pages(10, |_| {
            calls += 1;
            Ok(if calls == 1 {
                count_page(&[0, 1], 5)
            } else {
                count_page(&[], 4)
            })
        })
        .expect("empty page ends pagination");
        assert_eq!(calls, 2);
        assert_eq!(page.total, 4);
        assert_eq!(page.entries.len(), 2);
        assert!(metrics_success_report(&registry_entry(), page, false).truncated);
    }

    #[test]
    fn metrics_pagination_preserves_typed_failures_and_enforces_cap() {
        let error = collect_metric_pages(4, |_| {
            Err(MetricsQueryError::Observability(
                FleetObservabilityError::Protocol(CanisterProtocolError::WireUpdate {
                    canister: ic_platform::principal::ExternalPrincipal::management_canister(),
                    method: canic_contracts::protocol::inspection::METHOD,
                    source: IcpWireQueryError::Icp(IcpCommandError::Failed {
                        command: "query".into(),
                        stderr: String::new(),
                    }),
                }),
            ))
        });
        std::assert_matches!(error, Err(MetricsQueryError::Observability(_)));
        let page = collect_metric_pages(1, |_| Ok(count_page(&[1, 2], 2)))
            .expect("bound a page exceeding its requested limit");
        assert_eq!(page.entries.len(), 1);
        assert!(metrics_success_report(&registry_entry(), page, false).truncated);
    }
}
