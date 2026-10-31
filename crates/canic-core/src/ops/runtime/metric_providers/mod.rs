//! Module: ops::runtime::metric_providers
//!
//! Responsibility: own the bounded provider registry and project upstream metric values.
//! Does not own: metric arithmetic, codecs, authorization or scheduling.
//! Boundary: one host-owned optional governed map.

pub mod host;
#[cfg(test)]
mod tests;

use crate::config::schema::{MetricStorage, MetricStorageConfig};

use std::collections::BTreeSet;

use ic_memory::Memory;
use ic_metrics::{MeasurementAggregate, MeasurementCounter};
use ic_metrics_stable::{MetricIdentity, MetricKey, MetricsStore, StoreError};

/// Maximum component namespaces admitted by one canister.
pub const MAX_PROVIDERS: usize = 16;
/// Maximum fixed metric declarations across all providers, not per caller.
pub const MAX_COUNTERS: usize = 256;
/// The optional host allocation. Components must not open their own Metrics maps.
pub const MEMORY_KEY: &str =
    crate::role_contract::allocation::memory::metrics::PROVIDER_METRICS_KEY;

///
/// MetricKind
///
/// The fixed declaration selects the canonical Metrics arithmetic/storage family.
///

#[derive(Clone, Copy, Debug)]
pub enum MetricKind {
    /// A scalar total with sticky overflow quality.
    Counter,
    /// Measured samples with independent total/sample quality and an optional maximum.
    Measurement,
}
impl MetricKind {
    const fn empty(self) -> ProviderMetricValue {
        match self {
            Self::Counter => ProviderMetricValue::Counter(MeasurementCounter::ZERO),
            Self::Measurement => ProviderMetricValue::Measurement(MeasurementAggregate::EMPTY),
        }
    }
}
///
/// ProviderMetricValue
///
/// Source values retain sample counts, units and independent overflow quality.
///

#[derive(Clone, Copy, Debug)]
pub enum ProviderMetricValue {
    Counter(MeasurementCounter),
    Measurement(MeasurementAggregate),
}
impl ProviderMetricValue {
    #[must_use]
    pub const fn counter(self) -> Option<MeasurementCounter> {
        if let Self::Counter(value) = self {
            Some(value)
        } else {
            None
        }
    }
    #[must_use]
    pub const fn measurement(self) -> Option<MeasurementAggregate> {
        if let Self::Measurement(value) = self {
            Some(value)
        } else {
            None
        }
    }
}

///
/// MetricDefinition
///
/// A compile-selected scalar observation. Do not derive names from request data.
///

#[derive(Clone, Copy, Debug)]
pub struct MetricDefinition {
    pub kind: MetricKind,
    pub name: &'static str,
    pub unit: &'static str,
}
///
/// MetricObservation
///
/// A report preserves the original unit, numeric quality and reset window.
///

#[derive(Clone, Debug)]
pub struct MetricObservation {
    pub provider: &'static str,
    pub name: &'static str,
    pub unit: &'static str,
    pub storage: MetricStorage,
    pub window: u64,
    pub value: ProviderMetricValue,
}
///
/// ProviderMetricsError
///
/// Admission/storage failures are diagnostics, never business-effect evidence.
///

#[derive(Debug, thiserror::Error)]
pub enum ProviderMetricsError {
    #[error("metric provider registry is not in the required lifecycle phase")]
    Lifecycle,

    #[error("metric provider/counter roster exceeds its bound")]
    Capacity,

    #[error("duplicate metric provider or counter declaration")]
    Duplicate,

    #[error("metrics configuration selects an unregistered provider")]
    UnknownProvider,

    #[error("metric counter is not declared by this provider")]
    UnknownCounter,

    #[error("metric recording method disagrees with its fixed declaration")]
    WrongKind,

    #[error("retained metric counter is absent")]
    Missing,

    #[error("metric backing does not match the host lifecycle")]
    Backing,

    #[error("metric provider storage has a recorded failure")]
    Unavailable,

    #[error(transparent)]
    Store(#[from] StoreError),
}
struct Counter {
    provider: &'static str,
    definition: MetricDefinition,
    key: MetricKey,
    value: ProviderMetricValue,
    storage: MetricStorage,
}
///
/// ProviderMetrics
///
/// Canic's host creates one instance, registers components, then activates once.
/// Stable records checkpoint synchronously on each accepted update. Heap counters
/// never open the store. No query endpoint, timer or bootstrap is registered here.
///

pub struct ProviderMetrics<M: Memory> {
    counters: Vec<Counter>,
    providers: BTreeSet<&'static str>,
    stable: Option<MetricsStore<M>>,
    window: Option<u64>,
    failed: bool,
}
impl<M: Memory> Default for ProviderMetrics<M> {
    fn default() -> Self {
        Self {
            counters: Vec::new(),
            providers: BTreeSet::new(),
            stable: None,
            window: None,
            failed: false,
        }
    }
}
impl<M: Memory> ProviderMetrics<M> {
    /// Declare a fixed component roster before any recording or retained reads.
    pub fn register(
        &mut self,
        provider: &'static str,
        definitions: &'static [MetricDefinition],
    ) -> Result<(), ProviderMetricsError> {
        if self.window.is_some() {
            return Err(ProviderMetricsError::Lifecycle);
        }
        if self.providers.contains(provider) {
            return Err(ProviderMetricsError::Duplicate);
        }
        if self.providers.len() >= MAX_PROVIDERS
            || definitions.is_empty()
            || definitions.len() > MAX_COUNTERS - self.counters.len()
        {
            return Err(ProviderMetricsError::Capacity);
        }
        let mut names = BTreeSet::new();
        let rows = definitions
            .iter()
            .map(|&definition| {
                if !names.insert(definition.name) {
                    return Err(ProviderMetricsError::Duplicate);
                }
                let key = MetricKey::new(provider, definition.name)?;
                MetricIdentity::new(definition.unit, 0)?;
                Ok(Counter {
                    provider,
                    definition,
                    key,
                    value: definition.kind.empty(),
                    storage: MetricStorage::Heap,
                })
            })
            .collect::<Result<Vec<_>, ProviderMetricsError>>()?;
        self.providers.insert(provider);
        self.counters.extend(rows);
        Ok(())
    }
    /// Admit all overrides before opening memory. Fresh hosts save explicit zeros;
    /// restoration requires every selected stable record, without an init fallback.
    pub fn activate(
        &mut self,
        config: &MetricStorageConfig,
        fresh: bool,
        heap_window: u64,
        open: impl FnOnce() -> Result<MetricsStore<M>, ProviderMetricsError>,
    ) -> Result<(), ProviderMetricsError> {
        if self.window.is_some() {
            return Err(ProviderMetricsError::Lifecycle);
        }
        if config
            .providers
            .keys()
            .any(|p| !self.providers.contains(p.as_str()))
        {
            return Err(ProviderMetricsError::UnknownProvider);
        }
        let needs_stable = self
            .providers
            .iter()
            .any(|p| config.storage_for(p) == MetricStorage::Stable);
        let mut stable = if needs_stable { Some(open()?) } else { None };
        let values = self
            .counters
            .iter()
            .map(|row| {
                if config.storage_for(row.provider) == MetricStorage::Heap {
                    return Ok(row.definition.kind.empty());
                }
                let store = stable.as_mut().ok_or(ProviderMetricsError::Backing)?;
                let identity = MetricIdentity::new(row.definition.unit, 0)?;
                if fresh {
                    let value = row.definition.kind.empty();
                    save(store, &row.key, &identity, &value)?;
                    Ok(value)
                } else {
                    match row.definition.kind {
                        MetricKind::Counter => store
                            .load_counter(&row.key, &identity)?
                            .map(ProviderMetricValue::Counter),
                        MetricKind::Measurement => store
                            .load_aggregate(&row.key, &identity)?
                            .map(ProviderMetricValue::Measurement),
                    }
                    .ok_or(ProviderMetricsError::Missing)
                }
            })
            .collect::<Result<Vec<_>, ProviderMetricsError>>()?;
        for (row, value) in self.counters.iter_mut().zip(values) {
            row.storage = config.storage_for(row.provider);
            row.value = value;
        }
        self.stable = stable;
        self.window = Some(heap_window);
        Ok(())
    }
    /// Record only a declared scalar. A typed storage refusal keeps the old value
    /// and marks reporting unavailable, rather than publishing a silently low total.
    pub fn add(
        &mut self,
        provider: &str,
        name: &str,
        amount: u128,
    ) -> Result<(), ProviderMetricsError> {
        self.update(provider, name, |value| {
            let ProviderMetricValue::Counter(counter) = value else {
                return Err(ProviderMetricsError::WrongKind);
            };
            counter.add(amount);
            Ok(())
        })
    }
    /// Record one measured observation; a real zero is a sample, unlike absence.
    pub fn record(
        &mut self,
        provider: &str,
        name: &str,
        value: u64,
    ) -> Result<(), ProviderMetricsError> {
        self.update(provider, name, |state| {
            let ProviderMetricValue::Measurement(aggregate) = state else {
                return Err(ProviderMetricsError::WrongKind);
            };
            aggregate.record(value);
            Ok(())
        })
    }
    fn update(
        &mut self,
        provider: &str,
        name: &str,
        update: impl FnOnce(&mut ProviderMetricValue) -> Result<(), ProviderMetricsError>,
    ) -> Result<(), ProviderMetricsError> {
        if self.window.is_none() {
            return Err(ProviderMetricsError::Lifecycle);
        }
        if self.failed {
            return Err(ProviderMetricsError::Unavailable);
        }
        let row = self
            .counters
            .iter_mut()
            .find(|r| r.provider == provider && r.definition.name == name)
            .ok_or(ProviderMetricsError::UnknownCounter)?;
        let mut candidate = row.value;
        update(&mut candidate)?;
        if row.storage == MetricStorage::Stable {
            let identity = MetricIdentity::new(row.definition.unit, 0)?;
            if let Err(error) = save(
                self.stable.as_mut().ok_or(ProviderMetricsError::Backing)?,
                &row.key,
                &identity,
                &candidate,
            ) {
                self.failed = true;
                return Err(error.into());
            }
        }
        row.value = candidate;
        Ok(())
    }
    /// Heap and stable providers use this same source projection; live gauges
    /// stay with their authoritative service records and are not copied here.
    pub fn snapshot(&self, provider: &str) -> Result<Vec<MetricObservation>, ProviderMetricsError> {
        let window = self.window.ok_or(ProviderMetricsError::Lifecycle)?;
        if self.failed {
            return Err(ProviderMetricsError::Unavailable);
        }
        if !self.providers.contains(provider) {
            return Err(ProviderMetricsError::UnknownProvider);
        }
        Ok(self
            .counters
            .iter()
            .filter(|r| r.provider == provider)
            .map(|row| MetricObservation {
                provider: row.provider,
                name: row.definition.name,
                unit: row.definition.unit,
                storage: row.storage,
                window,
                value: row.value,
            })
            .collect())
    }
}
fn save<M: Memory>(
    store: &mut MetricsStore<M>,
    key: &MetricKey,
    identity: &MetricIdentity,
    value: &ProviderMetricValue,
) -> Result<(), StoreError> {
    match value {
        ProviderMetricValue::Counter(value) => store.save_counter(key, identity, value),
        ProviderMetricValue::Measurement(value) => store.save_aggregate(key, identity, value),
    }
}
