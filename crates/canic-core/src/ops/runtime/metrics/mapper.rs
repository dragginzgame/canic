use crate::{
    cdk::types::Principal,
    dto::metrics::{
        AccessMetricEntry, CyclesFundingMetricEntry, DelegationMetricEntry, HttpMetricEntry,
        IccMetricEntry, RootCapabilityMetricEntry, SystemMetricEntry, TimerMetricEntry,
    },
    ids::SystemMetricKind,
    ops::runtime::metrics::{
        access::AccessMetricKey,
        cycles_funding::{CyclesFundingDeniedReason, CyclesFundingMetricKey},
        http::HttpMetricKey,
        icc::IccMetricKey,
        root_capability::{
            RootCapabilityMetricEventType, RootCapabilityMetricKey, RootCapabilityMetricOutcome,
            RootCapabilityMetricProofMode,
        },
        timer::{TimerMetricKey, TimerMode},
    },
};

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
