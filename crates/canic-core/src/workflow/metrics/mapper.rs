use crate::{
    dto::metrics::{
        AccessMetricEntry, EndpointHealthView, HttpMetricEntry, IccMetricEntry, SystemMetricEntry,
        TimerMetricEntry,
    },
    ids::SystemMetricKind,
    ops::runtime::metrics::{
        access::AccessMetricKey,
        endpoint::{EndpointAttemptCounts, EndpointResultCounts},
        http::HttpMetricKey,
        icc::IccMetricKey,
        timer::{TimerMetricKey, TimerMode},
    },
};
use std::collections::{BTreeSet, HashMap};

///
/// MetricsMapper
///

pub struct MetricsMapper;

impl MetricsMapper {
    #[must_use]
    pub fn system_metrics_to_view(
        raw: impl IntoIterator<Item = (SystemMetricKind, u64)>,
    ) -> Vec<SystemMetricEntry> {
        raw.into_iter()
            .map(|(kind, count)| SystemMetricEntry {
                kind: Self::kind_to_string(kind),
                count,
            })
            .collect()
    }

    #[must_use]
    pub fn http_metrics_to_view(
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

    #[must_use]
    pub fn icc_metrics_to_view(
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

    #[must_use]
    pub fn timer_metrics_to_view(
        raw: impl IntoIterator<Item = (TimerMetricKey, u64)>,
    ) -> Vec<TimerMetricEntry> {
        raw.into_iter()
            .map(|(key, count)| TimerMetricEntry {
                mode: Self::mode_to_string(key.mode),
                delay_ms: key.delay_ms,
                label: key.label,
                count,
            })
            .collect()
    }

    #[must_use]
    pub fn access_metrics_to_view(
        raw: impl IntoIterator<Item = (AccessMetricKey, u64)>,
    ) -> Vec<AccessMetricEntry> {
        raw.into_iter()
            .map(|(key, count)| AccessMetricEntry {
                endpoint: key.endpoint,
                kind: key.kind,
                count,
            })
            .collect()
    }

    #[must_use]
    pub fn endpoint_health_to_view(
        attempts: impl IntoIterator<Item = (&'static str, EndpointAttemptCounts)>,
        results: impl IntoIterator<Item = (&'static str, EndpointResultCounts)>,
        access: impl IntoIterator<Item = (AccessMetricKey, u64)>,
        exclude_endpoint: Option<&str>,
    ) -> Vec<EndpointHealthView> {
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

                EndpointHealthView {
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

    fn kind_to_string(kind: SystemMetricKind) -> String {
        match kind {
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
        .to_string()
    }

    fn mode_to_string(mode: TimerMode) -> String {
        match mode {
            TimerMode::Once => "once",
            TimerMode::Interval => "interval",
        }
        .to_string()
    }
}
