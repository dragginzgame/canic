use crate::{
    cdk::types::Principal,
    dto::metrics::{
        AccessMetricEntry, AuthMetricEntry, AuthRolloutMetricClass, AuthRolloutMetricEntry,
        CyclesFundingMetricEntry, DelegationMetricEntry, EndpointHealth, HttpMetricEntry,
        IccMetricEntry, RootCapabilityMetricEntry, SystemMetricEntry, TimerMetricEntry,
    },
    ids::{AccessMetricKind, SystemMetricKind},
    ops::runtime::metrics::{
        access::AccessMetricKey,
        cycles_funding::{CyclesFundingDeniedReason, CyclesFundingMetricKey},
        endpoint::{EndpointAttemptCounts, EndpointResultCounts},
        http::HttpMetricKey,
        icc::IccMetricKey,
        root_capability::{
            RootCapabilityMetricEventType, RootCapabilityMetricKey, RootCapabilityMetricOutcome,
            RootCapabilityMetricProofMode,
        },
        timer::{TimerMetricKey, TimerMode},
    },
};
use std::collections::{BTreeSet, HashMap};

///
/// SystemMetricEntryMapper
///

pub struct SystemMetricEntryMapper;

impl SystemMetricEntryMapper {
    #[must_use]
    pub fn record_to_view(
        raw: impl IntoIterator<Item = (SystemMetricKind, u64)>,
    ) -> Vec<SystemMetricEntry> {
        raw.into_iter()
            .map(|(kind, count)| SystemMetricEntry {
                kind: match kind {
                    SystemMetricKind::CanisterCall => "CanisterCall",
                    SystemMetricKind::CanisterStatus => "CanisterStatus",
                    SystemMetricKind::CreateCanister => "CreateCanister",
                    SystemMetricKind::DeleteCanister => "DeleteCanister",
                    SystemMetricKind::DepositCycles => "DepositCycles",
                    SystemMetricKind::HttpOutcall => "HttpOutcall",
                    SystemMetricKind::InstallCode => "InstallCode",
                    SystemMetricKind::RawRand => "RawRand",
                    SystemMetricKind::ReinstallCode => "ReinstallCode",
                    SystemMetricKind::TimerScheduled => "TimerScheduled",
                    SystemMetricKind::UninstallCode => "UninstallCode",
                    SystemMetricKind::UpdateSettings => "UpdateSettings",
                    SystemMetricKind::UpgradeCode => "UpgradeCode",
                }
                .to_string(),
                count,
            })
            .collect()
    }
}

///
/// HttpMetricEntryMapper
///

pub struct HttpMetricEntryMapper;

impl HttpMetricEntryMapper {
    #[must_use]
    pub fn record_to_view(
        raw: impl IntoIterator<Item = (HttpMetricKey, u64)>,
    ) -> Vec<HttpMetricEntry> {
        raw.into_iter()
            .map(|(key, count)| HttpMetricEntry {
                method: key.method.as_str().to_string(),
                label: key.label,
                count,
            })
            .collect()
    }
}

///
/// IccMetricEntryMapper
///

pub struct IccMetricEntryMapper;

impl IccMetricEntryMapper {
    #[must_use]
    pub fn record_to_view(
        raw: impl IntoIterator<Item = (IccMetricKey, u64)>,
    ) -> Vec<IccMetricEntry> {
        raw.into_iter()
            .map(|(key, count)| IccMetricEntry {
                target: key.target,
                method: key.method,
                count,
            })
            .collect()
    }
}

///
/// TimerMetricEntryMapper
///

pub struct TimerMetricEntryMapper;

impl TimerMetricEntryMapper {
    #[must_use]
    pub fn record_to_view(
        raw: impl IntoIterator<Item = (TimerMetricKey, u64)>,
    ) -> Vec<TimerMetricEntry> {
        raw.into_iter()
            .map(|(key, count)| TimerMetricEntry {
                mode: match key.mode {
                    TimerMode::Once => "once",
                    TimerMode::Interval => "interval",
                }
                .to_string(),
                delay_ms: key.delay_ms,
                label: key.label,
                count,
            })
            .collect()
    }
}

///
/// AccessMetricEntryMapper
///

pub struct AccessMetricEntryMapper;

impl AccessMetricEntryMapper {
    #[must_use]
    pub fn record_to_view(
        raw: impl IntoIterator<Item = (AccessMetricKey, u64)>,
    ) -> Vec<AccessMetricEntry> {
        raw.into_iter()
            .map(|(key, count)| AccessMetricEntry {
                endpoint: key.endpoint,
                kind: key.kind,
                predicate: key.predicate,
                count,
            })
            .collect()
    }
}

///
/// AuthMetricEntryMapper
///

pub struct AuthMetricEntryMapper;

impl AuthMetricEntryMapper {
    #[must_use]
    pub fn record_to_view(
        raw: impl IntoIterator<Item = (AccessMetricKey, u64)>,
    ) -> Vec<AuthMetricEntry> {
        raw.into_iter()
            .filter_map(|(key, count)| {
                (key.kind == AccessMetricKind::Auth).then_some(AuthMetricEntry {
                    endpoint: key.endpoint,
                    predicate: key.predicate,
                    count,
                })
            })
            .collect()
    }
}

///
/// AuthRolloutMetricEntryMapper
///

pub struct AuthRolloutMetricEntryMapper;

#[derive(Clone, Copy)]
struct AuthRolloutSignalSpec {
    signal: &'static str,
    class: AuthRolloutMetricClass,
    matches: fn(&str) -> bool,
}

const AUTH_ROLLOUT_SIGNAL_SPECS: [AuthRolloutSignalSpec; 7] = [
    AuthRolloutSignalSpec {
        signal: "proof_miss",
        class: AuthRolloutMetricClass::HardGate,
        matches: predicate_is_proof_miss,
    },
    AuthRolloutSignalSpec {
        signal: "proof_mismatch",
        class: AuthRolloutMetricClass::HardGate,
        matches: predicate_is_proof_mismatch,
    },
    AuthRolloutSignalSpec {
        signal: "active_proof_eviction",
        class: AuthRolloutMetricClass::HardGate,
        matches: predicate_is_active_proof_eviction,
    },
    AuthRolloutSignalSpec {
        signal: "repair_failure",
        class: AuthRolloutMetricClass::HardGate,
        matches: predicate_is_repair_failure,
    },
    AuthRolloutSignalSpec {
        signal: "cache_saturation",
        class: AuthRolloutMetricClass::HardGate,
        matches: predicate_is_cache_saturation,
    },
    AuthRolloutSignalSpec {
        signal: "cold_proof_eviction",
        class: AuthRolloutMetricClass::Operational,
        matches: predicate_is_cold_proof_eviction,
    },
    AuthRolloutSignalSpec {
        signal: "prewarm_failure",
        class: AuthRolloutMetricClass::Operational,
        matches: predicate_is_prewarm_failure,
    },
];

impl AuthRolloutMetricEntryMapper {
    #[must_use]
    pub fn record_to_view(
        raw: impl IntoIterator<Item = (AccessMetricKey, u64)>,
    ) -> Vec<AuthRolloutMetricEntry> {
        let mut counts = [0u64; AUTH_ROLLOUT_SIGNAL_SPECS.len()];

        for (key, count) in raw {
            if key.kind != AccessMetricKind::Auth {
                continue;
            }

            for (index, spec) in AUTH_ROLLOUT_SIGNAL_SPECS.iter().enumerate() {
                if (spec.matches)(&key.predicate) {
                    counts[index] = counts[index].saturating_add(count);
                }
            }
        }

        AUTH_ROLLOUT_SIGNAL_SPECS
            .iter()
            .enumerate()
            .map(|(index, spec)| AuthRolloutMetricEntry {
                signal: spec.signal.to_string(),
                class: spec.class,
                count: counts[index],
            })
            .collect()
    }
}

fn predicate_is_proof_miss(predicate: &str) -> bool {
    predicate == "token_rejected_proof_miss"
}

fn predicate_is_proof_mismatch(predicate: &str) -> bool {
    predicate == "token_rejected_proof_mismatch"
}

fn predicate_is_active_proof_eviction(predicate: &str) -> bool {
    predicate == "proof_cache_evictions_total{class=\"active\"}"
}

fn predicate_is_cold_proof_eviction(predicate: &str) -> bool {
    predicate == "proof_cache_evictions_total{class=\"cold\"}"
}

fn predicate_is_repair_failure(predicate: &str) -> bool {
    (predicate.starts_with("delegation_push_failed{") && predicate.contains("origin=\"repair\""))
        || (predicate.starts_with("delegation_install_normalization_rejected{")
            && predicate.contains("intent=\"repair\""))
        || (predicate.starts_with("delegation_install_validation_failed{")
            && predicate.contains("intent=\"repair\""))
}

fn predicate_is_prewarm_failure(predicate: &str) -> bool {
    (predicate.starts_with("delegation_push_failed{") && predicate.contains("origin=\"prewarm\""))
        || (predicate.starts_with("delegation_install_normalization_rejected{")
            && predicate.contains("intent=\"prewarm\""))
        || (predicate.starts_with("delegation_install_validation_failed{")
            && predicate.contains("intent=\"prewarm\""))
}

fn predicate_is_cache_saturation(predicate: &str) -> bool {
    predicate == "proof_cache_utilization{bucket=\"85_94\"}"
        || predicate == "proof_cache_utilization{bucket=\"95_100\"}"
}

///
/// DelegationMetricEntryMapper
///

pub struct DelegationMetricEntryMapper;

impl DelegationMetricEntryMapper {
    #[must_use]
    pub fn record_to_view(
        raw: impl IntoIterator<Item = (Principal, u64)>,
    ) -> Vec<DelegationMetricEntry> {
        raw.into_iter()
            .map(|(authority, count)| DelegationMetricEntry { authority, count })
            .collect()
    }
}

///
/// RootCapabilityMetricEntryMapper
///

pub struct RootCapabilityMetricEntryMapper;

impl RootCapabilityMetricEntryMapper {
    #[must_use]
    pub fn record_to_view(
        raw: impl IntoIterator<
            Item = (
                RootCapabilityMetricKey,
                RootCapabilityMetricEventType,
                RootCapabilityMetricOutcome,
                RootCapabilityMetricProofMode,
                u64,
            ),
        >,
    ) -> Vec<RootCapabilityMetricEntry> {
        raw.into_iter()
            .map(
                |(capability, event_type, outcome, proof_mode, count)| RootCapabilityMetricEntry {
                    capability: capability.metric_label().to_string(),
                    event_type: event_type.metric_label().to_string(),
                    outcome: outcome.metric_label().to_string(),
                    proof_mode: proof_mode.metric_label().to_string(),
                    count,
                },
            )
            .collect()
    }
}

///
/// CyclesFundingMetricEntryMapper
///

pub struct CyclesFundingMetricEntryMapper;

impl CyclesFundingMetricEntryMapper {
    #[must_use]
    pub fn record_to_view(
        raw: impl IntoIterator<
            Item = (
                CyclesFundingMetricKey,
                Option<Principal>,
                Option<CyclesFundingDeniedReason>,
                u128,
            ),
        >,
    ) -> Vec<CyclesFundingMetricEntry> {
        raw.into_iter()
            .map(
                |(metric, child_principal, reason, cycles)| CyclesFundingMetricEntry {
                    metric: metric.metric_label().to_string(),
                    child_principal,
                    reason: reason.map(|label| label.metric_label().to_string()),
                    cycles,
                },
            )
            .collect()
    }
}

///
/// EndpointHealthMapper
///

pub struct EndpointHealthMapper;

impl EndpointHealthMapper {
    #[must_use]
    pub fn record_to_view(
        attempts: impl IntoIterator<Item = (&'static str, EndpointAttemptCounts)>,
        results: impl IntoIterator<Item = (&'static str, EndpointResultCounts)>,
        access: impl IntoIterator<Item = (AccessMetricKey, u64)>,
        exclude_endpoint: Option<&str>,
    ) -> Vec<EndpointHealth> {
        // Aggregate access-kind denials (guard/auth/env/rule/custom) per endpoint.
        let mut denied: HashMap<String, u64> = HashMap::new();

        for (key, count) in access {
            denied
                .entry(key.endpoint)
                .and_modify(|v| *v = v.saturating_add(count))
                .or_insert(count);
        }

        let mut endpoints = BTreeSet::<String>::new();

        let attempts: HashMap<&'static str, EndpointAttemptCounts> = attempts
            .into_iter()
            .inspect(|(ep, _)| {
                endpoints.insert((*ep).to_string());
            })
            .collect();

        let results: HashMap<&'static str, EndpointResultCounts> = results
            .into_iter()
            .inspect(|(ep, _)| {
                endpoints.insert((*ep).to_string());
            })
            .collect();

        endpoints.extend(denied.keys().cloned());

        endpoints
            .into_iter()
            .filter(|endpoint| match exclude_endpoint {
                Some(excluded) => endpoint != excluded,
                None => true,
            })
            .map(|endpoint| {
                let (attempted, completed) = attempts
                    .get(endpoint.as_str())
                    .map_or((0, 0), |c| (c.attempted, c.completed));

                let (ok, err) = results
                    .get(endpoint.as_str())
                    .map_or((0, 0), |c| (c.ok, c.err));

                let denied = denied.get(&endpoint).copied().unwrap_or(0);

                EndpointHealth {
                    endpoint,
                    attempted,
                    denied,
                    completed,
                    ok,
                    err,
                }
            })
            .collect()
    }
}
