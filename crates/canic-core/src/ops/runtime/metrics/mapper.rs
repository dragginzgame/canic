use crate::{
    cdk::types::Principal,
    dto::metrics::{
        AccessMetricEntry, DelegationMetricEntry, EndpointHealth, HttpMetricEntry, IccMetricEntry,
        RootCapabilityMetricEntry, SystemMetricEntry, TimerMetricEntry,
    },
    ids::SystemMetricKind,
    ops::runtime::metrics::{
        access::AccessMetricKey,
        endpoint::{EndpointAttemptCounts, EndpointResultCounts},
        http::HttpMetricKey,
        icc::IccMetricKey,
        root_capability::{RootCapabilityMetricEvent, RootCapabilityMetricKey},
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
        raw: impl IntoIterator<Item = (RootCapabilityMetricKey, RootCapabilityMetricEvent, u64)>,
    ) -> Vec<RootCapabilityMetricEntry> {
        raw.into_iter()
            .map(|(capability, event, count)| RootCapabilityMetricEntry {
                capability: capability.metric_label().to_string(),
                event: event.metric_label().to_string(),
                count,
            })
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
