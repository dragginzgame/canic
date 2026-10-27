//! Exact native/external observation conversion; no controller, balance or recovery policy.
use super::unsigned_magnitude::{from_nat, to_nat};
use crate::dto::{canister as external, inspection as wire};
use ic_wire::Error;

impl TryFrom<external::CanisterStatusResponse> for wire::Status {
    type Error = Error;
    fn try_from(value: external::CanisterStatusResponse) -> Result<Self, Error> {
        Ok(Self {
            version: value.version,
            status: value.status,
            settings: value.settings.try_into()?,
            module_hash: value.module_hash,
            memory_size: from_nat(&value.memory_size)?,
            memory_metrics: value.memory_metrics.try_into()?,
            cycles: from_nat(&value.cycles)?,
            reserved_cycles: from_nat(&value.reserved_cycles)?,
            idle_cycles_burned_per_day: from_nat(&value.idle_cycles_burned_per_day)?,
            query_stats: value.query_stats.try_into()?,
        })
    }
}

impl TryFrom<wire::Status> for external::CanisterStatusResponse {
    type Error = Error;
    fn try_from(value: wire::Status) -> Result<Self, Error> {
        Ok(Self {
            version: value.version,
            status: value.status,
            settings: value.settings.try_into()?,
            module_hash: value.module_hash,
            memory_size: to_nat(&value.memory_size)?,
            memory_metrics: value.memory_metrics.try_into()?,
            cycles: to_nat(&value.cycles)?,
            reserved_cycles: to_nat(&value.reserved_cycles)?,
            idle_cycles_burned_per_day: to_nat(&value.idle_cycles_burned_per_day)?,
            query_stats: value.query_stats.try_into()?,
        })
    }
}

impl TryFrom<external::CanisterSettings> for wire::Settings {
    type Error = Error;
    fn try_from(value: external::CanisterSettings) -> Result<Self, Error> {
        Ok(Self {
            controllers: value.controllers.into_iter().map(Into::into).collect(),
            compute_allocation: from_nat(&value.compute_allocation)?,
            memory_allocation: from_nat(&value.memory_allocation)?,
            freezing_threshold: from_nat(&value.freezing_threshold)?,
            reserved_cycles_limit: from_nat(&value.reserved_cycles_limit)?,
            log_visibility: value.log_visibility.into(),
            log_memory_limit: from_nat(&value.log_memory_limit)?,
            wasm_memory_limit: from_nat(&value.wasm_memory_limit)?,
            wasm_memory_threshold: from_nat(&value.wasm_memory_threshold)?,
            environment_variables: value.environment_variables,
        })
    }
}

impl TryFrom<wire::Settings> for external::CanisterSettings {
    type Error = Error;
    fn try_from(value: wire::Settings) -> Result<Self, Error> {
        Ok(Self {
            controllers: value.controllers.into_iter().map(Into::into).collect(),
            compute_allocation: to_nat(&value.compute_allocation)?,
            memory_allocation: to_nat(&value.memory_allocation)?,
            freezing_threshold: to_nat(&value.freezing_threshold)?,
            reserved_cycles_limit: to_nat(&value.reserved_cycles_limit)?,
            log_visibility: value.log_visibility.into(),
            log_memory_limit: to_nat(&value.log_memory_limit)?,
            wasm_memory_limit: to_nat(&value.wasm_memory_limit)?,
            wasm_memory_threshold: to_nat(&value.wasm_memory_threshold)?,
            environment_variables: value.environment_variables,
        })
    }
}

impl TryFrom<external::MemoryMetrics> for wire::MemoryMetrics {
    type Error = Error;
    fn try_from(value: external::MemoryMetrics) -> Result<Self, Error> {
        Ok(Self {
            wasm_memory_size: from_nat(&value.wasm_memory_size)?,
            stable_memory_size: from_nat(&value.stable_memory_size)?,
            global_memory_size: from_nat(&value.global_memory_size)?,
            wasm_binary_size: from_nat(&value.wasm_binary_size)?,
            custom_sections_size: from_nat(&value.custom_sections_size)?,
            canister_history_size: from_nat(&value.canister_history_size)?,
            wasm_chunk_store_size: from_nat(&value.wasm_chunk_store_size)?,
            snapshots_size: from_nat(&value.snapshots_size)?,
        })
    }
}

impl TryFrom<wire::MemoryMetrics> for external::MemoryMetrics {
    type Error = Error;
    fn try_from(value: wire::MemoryMetrics) -> Result<Self, Error> {
        Ok(Self {
            wasm_memory_size: to_nat(&value.wasm_memory_size)?,
            stable_memory_size: to_nat(&value.stable_memory_size)?,
            global_memory_size: to_nat(&value.global_memory_size)?,
            wasm_binary_size: to_nat(&value.wasm_binary_size)?,
            custom_sections_size: to_nat(&value.custom_sections_size)?,
            canister_history_size: to_nat(&value.canister_history_size)?,
            wasm_chunk_store_size: to_nat(&value.wasm_chunk_store_size)?,
            snapshots_size: to_nat(&value.snapshots_size)?,
        })
    }
}

impl TryFrom<external::QueryStats> for wire::QueryStats {
    type Error = Error;
    fn try_from(value: external::QueryStats) -> Result<Self, Error> {
        Ok(Self {
            num_calls_total: from_nat(&value.num_calls_total)?,
            num_instructions_total: from_nat(&value.num_instructions_total)?,
            request_payload_bytes_total: from_nat(&value.request_payload_bytes_total)?,
            response_payload_bytes_total: from_nat(&value.response_payload_bytes_total)?,
        })
    }
}

impl TryFrom<wire::QueryStats> for external::QueryStats {
    type Error = Error;
    fn try_from(value: wire::QueryStats) -> Result<Self, Error> {
        Ok(Self {
            num_calls_total: to_nat(&value.num_calls_total)?,
            num_instructions_total: to_nat(&value.num_instructions_total)?,
            request_payload_bytes_total: to_nat(&value.request_payload_bytes_total)?,
            response_payload_bytes_total: to_nat(&value.response_payload_bytes_total)?,
        })
    }
}

impl From<crate::values::canister::LogVisibility> for wire::LogVisibility {
    fn from(value: crate::values::canister::LogVisibility) -> Self {
        use crate::values::canister::LogVisibility as External;
        match value {
            External::Controllers => Self::Controllers,
            External::Public => Self::Public,
            External::AllowedViewers(viewers) => {
                Self::AllowedViewers(viewers.into_iter().map(Into::into).collect())
            }
        }
    }
}
impl From<wire::LogVisibility> for crate::values::canister::LogVisibility {
    fn from(value: wire::LogVisibility) -> Self {
        match value {
            wire::LogVisibility::Controllers => Self::Controllers,
            wire::LogVisibility::Public => Self::Public,
            wire::LogVisibility::AllowedViewers(viewers) => {
                Self::AllowedViewers(viewers.into_iter().map(Into::into).collect())
            }
        }
    }
}
impl TryFrom<external::CanisterInspectionOutcome> for wire::Response {
    type Error = Error;
    fn try_from(value: external::CanisterInspectionOutcome) -> Result<Self, Error> {
        match value {
            external::CanisterInspectionOutcome::Status(status) => {
                Ok(Self::InspectCanister(Box::new((*status).try_into()?)))
            }
            external::CanisterInspectionOutcome::ReserveRequired(evidence) => {
                Ok(Self::InspectionReserveRequired(evidence.into()))
            }
        }
    }
}
impl TryFrom<wire::Response> for external::CanisterInspectionOutcome {
    type Error = Error;
    fn try_from(value: wire::Response) -> Result<Self, Error> {
        match value {
            wire::Response::InspectCanister(status) => {
                Ok(Self::Status(Box::new((*status).try_into()?)))
            }
            wire::Response::InspectionReserveRequired(evidence) => {
                Ok(Self::ReserveRequired(evidence.into()))
            }
        }
    }
}

#[cfg(test)]
mod tests;
