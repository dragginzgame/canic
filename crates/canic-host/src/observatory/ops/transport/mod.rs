//! Exact role-specific queries and bounded conversion into passive host facts.

mod process;
#[cfg(test)]
mod tests;

use crate::{
    icp::{IcpCli, IcpJsonResponseError, cleanup_binary_argument_file, response_bytes},
    observatory::view::{
        CostWindowView, MetricSamplesView, ObservationFailure, RoleFundingView, RoleOverviewView,
        RootEstateView, StoreInventoryView,
    },
    registry::RegistryEntry,
};
use canic_contracts::{
    dto::{
        page::PageRequest,
        public_status::{
            PublicHistoryRequest, PublicHistorySnapshot, PublicMetricFamily, PublicMetricsRequest,
        },
        template::WasmStoreStatusResponse,
    },
    ids::WasmStoreGcMode,
    protocol,
};
use canic_core::role_contract::RoleCapabilityKey;
use ic_platform::principal::ExternalPrincipal as Principal;
use std::path::Path;

/// Independent role observations; implementations must not retry or substitute another role.
pub trait ObservatoryTransport {
    fn overview(&mut self, entry: &RegistryEntry) -> Result<RoleOverviewView, ObservationFailure>;
    fn funding(&mut self, entry: &RegistryEntry) -> Result<RoleFundingView, ObservationFailure>;
    fn estate(&mut self, entry: &RegistryEntry) -> Result<RootEstateView, ObservationFailure>;
    fn store(&mut self, entry: &RegistryEntry) -> Result<StoreInventoryView, ObservationFailure>;
    fn metric_samples(
        &mut self,
        entry: &RegistryEntry,
        family: PublicMetricFamily,
    ) -> Result<MetricSamplesView, ObservationFailure>;
    fn cost_window(&mut self, entry: &RegistryEntry) -> Result<CostWindowView, ObservationFailure>;
    fn attempts(&self) -> u64;
}

/// Query-only ICP adapter. The budget bounds envelope admission before payload decoding.
pub struct IcpObservatoryTransport<'a> {
    pub icp: &'a IcpCli,
    pub root: &'a Path,
    pub fleet: &'a str,
    pub environment: &'a str,
    pub maximum_response_bytes: usize,
    pub attempted_queries: u64,
    pub query_timeout: std::time::Duration,
    pub deadline: std::time::Instant,
    pub compatibility: Option<Result<(), ObservationFailure>>,
}

/// Exact declaration admitted before a native query is sent.
struct NativeQueryContract {
    method_id: u64,
    request: ic_wire::Limits,
    response: ic_wire::Limits,
}

impl IcpObservatoryTransport<'_> {
    fn query_encoded(
        &mut self,
        entry: &RegistryEntry,
        method: &'static str,
        argument: &[u8],
        contract: NativeQueryContract,
    ) -> Result<String, ObservationFailure> {
        let remaining = self
            .deadline
            .checked_duration_since(std::time::Instant::now())
            .ok_or(ObservationFailure::TimedOut)?;
        let timeout = remaining.min(self.query_timeout);
        if self.compatibility.is_none() {
            let mut command = self.icp.command();
            command.arg("--version");
            self.compatibility = Some(process::query_output(&mut command, 4096, timeout).and_then(
                |version| {
                    if IcpCli::accepts_version_output(&version) {
                        Ok(())
                    } else {
                        Err(ObservationFailure::Unsupported)
                    }
                },
            ));
        }
        self.compatibility
            .as_ref()
            .expect("checked version")
            .clone()?;
        let current =
            crate::fleet_ensure::resolve_current_fleet(self.root, self.environment, self.fleet)
                .map_err(|_| ObservationFailure::BindingUnavailable)?;
        let schema = crate::protocol_binding::resolve_current_registry_wire_schema(
            self.root, &current, entry,
        )
        .map_err(|_| ObservationFailure::BindingUnavailable)?;
        crate::artifact_io::require_wire_method(
            &schema.schema,
            method,
            contract.method_id,
            protocol::wire_declaration::MethodKind::Query,
            contract.request,
            contract.response,
        )
        .map_err(|_| ObservationFailure::BindingUnavailable)?;
        let canister =
            Principal::from_text(&entry.pid).map_err(|_| ObservationFailure::InvalidResponse)?;
        let argument = crate::icp::write_binary_argument_file(argument)
            .map_err(|_| ObservationFailure::TransportUnavailable)?;
        let mut command =
            self.icp
                .bounded_query_command(&canister.to_text(), method, &argument, None);
        self.attempted_queries += 1;
        let result = self
            .deadline
            .checked_duration_since(std::time::Instant::now())
            .ok_or(ObservationFailure::TimedOut)
            .and_then(|remaining| {
                process::query_output(
                    &mut command,
                    self.maximum_response_bytes,
                    remaining.min(self.query_timeout),
                )
            });
        cleanup_binary_argument_file(&argument);
        result
    }
}

impl ObservatoryTransport for IcpObservatoryTransport<'_> {
    fn overview(&mut self, entry: &RegistryEntry) -> Result<RoleOverviewView, ObservationFailure> {
        let argument = ic_wire::native::encode_request(
            protocol::public_overview::METHOD_ID,
            &(),
            protocol::public_overview::REQUEST_LIMITS,
        )
        .map_err(|_| ObservationFailure::InvalidResponse)?;
        let envelope = self.query_encoded(
            entry,
            protocol::CANIC_PUBLIC_OVERVIEW,
            &argument,
            NativeQueryContract {
                method_id: protocol::public_overview::METHOD_ID,
                request: protocol::public_overview::REQUEST_LIMITS,
                response: protocol::public_overview::RESPONSE_LIMITS,
            },
        )?;
        let response: Result<
            canic_contracts::dto::role::RoleOverviewResponse,
            canic_contracts::dto::error::Error,
        > = decode_public_reply(
            &envelope,
            self.maximum_response_bytes,
            protocol::public_overview::METHOD_ID,
            protocol::public_overview::RESPONSE_LIMITS,
        )?;
        let reply =
            response.map_err(|error| response_failure(IcpJsonResponseError::Rejected(error)))?;
        let binding = entry
            .protocol_binding
            .as_ref()
            .ok_or(ObservationFailure::BindingUnavailable)?;
        if reply.role != binding.role
            || reply.protocol_profile_digest != *binding.protocol_profile_digest.as_bytes()
        {
            return Err(ObservationFailure::ProfileMismatch);
        }
        if reply.metadata.canic_version.len() > 128
            || reply.metadata.canic_version.chars().any(char::is_control)
        {
            return Err(ObservationFailure::InvalidResponse);
        }
        Ok(RoleOverviewView {
            bootstrap_ready: reply.bootstrap.ready,
            canic_version: reply.metadata.canic_version,
            canister_version: reply.metadata.canister_version,
        })
    }

    fn funding(&mut self, entry: &RegistryEntry) -> Result<RoleFundingView, ObservationFailure> {
        match entry.role.as_deref() {
            Some("fleet_coordinator") => {
                let argument = ic_wire::native::encode_request(
                    protocol::funding_read::coordinator_status::METHOD_ID,
                    &(),
                    protocol::funding_read::REQUEST_LIMITS,
                )
                .map_err(|_| ObservationFailure::InvalidResponse)?;
                let envelope = self.query_encoded(
                    entry,
                    protocol::funding_read::coordinator_status::METHOD,
                    &argument,
                    NativeQueryContract {
                        method_id: protocol::funding_read::coordinator_status::METHOD_ID,
                        request: protocol::funding_read::REQUEST_LIMITS,
                        response: protocol::funding_read::RESPONSE_LIMITS,
                    },
                )?;
                let response: Result<
                    canic_contracts::dto::funding_read::CoordinatorStatus,
                    canic_contracts::dto::error::Error,
                > = decode_public_reply(
                    &envelope,
                    self.maximum_response_bytes,
                    protocol::funding_read::coordinator_status::METHOD_ID,
                    protocol::funding_read::RESPONSE_LIMITS,
                )?;
                let reply = response
                    .map_err(|error| response_failure(IcpJsonResponseError::Rejected(error)))?;
                if reply.coordinator.to_text() != entry.pid {
                    return Err(ObservationFailure::InvalidResponse);
                }
                Ok(RoleFundingView {
                    native_cycles: reply.current_cycles.to_u128().to_string(),
                    automatic_funding_enabled: reply.funding_enabled,
                    policy_generation: reply.policy_generation,
                    pending_operations: reply
                        .roots
                        .iter()
                        .filter(|root| root.current_operation.is_some())
                        .count() as u64,
                })
            }
            Some("root") => {
                let argument = ic_wire::native::encode_request(
                    protocol::funding_read::root_status::METHOD_ID,
                    &(),
                    protocol::funding_read::REQUEST_LIMITS,
                )
                .map_err(|_| ObservationFailure::InvalidResponse)?;
                let envelope = self.query_encoded(
                    entry,
                    protocol::funding_read::root_status::METHOD,
                    &argument,
                    NativeQueryContract {
                        method_id: protocol::funding_read::root_status::METHOD_ID,
                        request: protocol::funding_read::REQUEST_LIMITS,
                        response: protocol::funding_read::RESPONSE_LIMITS,
                    },
                )?;
                let response: Result<
                    canic_contracts::dto::funding_read::RootStatus,
                    canic_contracts::dto::error::Error,
                > = decode_public_reply(
                    &envelope,
                    self.maximum_response_bytes,
                    protocol::funding_read::root_status::METHOD_ID,
                    protocol::funding_read::RESPONSE_LIMITS,
                )?;
                let native = response
                    .map_err(|error| response_failure(IcpJsonResponseError::Rejected(error)))?;
                let reply: canic_contracts::dto::root::RootFundingStatusResponse = native
                    .try_into()
                    .map_err(|_| ObservationFailure::InvalidResponse)?;
                if reply.fleet_subnet_root.to_text() != entry.pid {
                    return Err(ObservationFailure::InvalidResponse);
                }
                Ok(RoleFundingView {
                    native_cycles: reply.current_cycles.to_u128().to_string(),
                    automatic_funding_enabled: reply.cycles_funding_enabled,
                    policy_generation: reply.policy_generation,
                    pending_operations: u64::from(reply.current_operation.is_some()),
                })
            }
            _ => Err(ObservationFailure::Unsupported),
        }
    }

    fn estate(&mut self, entry: &RegistryEntry) -> Result<RootEstateView, ObservationFailure> {
        if entry.role.as_deref() != Some("root") {
            return Err(ObservationFailure::Unsupported);
        }
        let argument = ic_wire::native::encode_request(
            protocol::pool_inventory::METHOD_ID,
            &canic_contracts::dto::pool_inventory::Request {
                start_after: None,
                limit: 1,
            },
            protocol::pool_inventory::REQUEST_LIMITS,
        )
        .map_err(|_| ObservationFailure::InvalidResponse)?;
        let envelope = self.query_encoded(
            entry,
            protocol::pool_inventory::METHOD,
            &argument,
            NativeQueryContract {
                method_id: protocol::pool_inventory::METHOD_ID,
                request: protocol::pool_inventory::REQUEST_LIMITS,
                response: protocol::pool_inventory::RESPONSE_LIMITS,
            },
        )?;
        let response: Result<
            canic_contracts::dto::pool_inventory::Page,
            canic_contracts::dto::error::Error,
        > = decode_public_reply(
            &envelope,
            self.maximum_response_bytes,
            protocol::pool_inventory::METHOD_ID,
            protocol::pool_inventory::RESPONSE_LIMITS,
        )?;
        let reply =
            response.map_err(|error| response_failure(IcpJsonResponseError::Rejected(error)))?;
        if reply.entries.len() > 1 {
            return Err(ObservationFailure::InvalidResponse);
        }
        Ok(RootEstateView {
            tracked: reply.tracked,
            stores: reply.store,
            workloads: reply.workload,
            ready: reply.ready,
            failed: reply.failed,
            resetting: reply.pending_reset,
            pending_operations: u32::from(reply.pending_creation.is_some())
                + u32::from(reply.pending_handoff.is_some()),
        })
    }

    fn store(&mut self, entry: &RegistryEntry) -> Result<StoreInventoryView, ObservationFailure> {
        if entry.role.as_deref() != Some("wasm_store") {
            return Err(ObservationFailure::Unsupported);
        }
        let argument = ic_wire::native::encode_request(
            protocol::store_inventory::storage::METHOD_ID,
            &(),
            protocol::store_inventory::REQUEST_LIMITS,
        )
        .map_err(|_| ObservationFailure::InvalidResponse)?;
        let envelope = self.query_encoded(
            entry,
            protocol::store_inventory::storage::METHOD,
            &argument,
            NativeQueryContract {
                method_id: protocol::store_inventory::storage::METHOD_ID,
                request: protocol::store_inventory::REQUEST_LIMITS,
                response: protocol::store_inventory::RESPONSE_LIMITS,
            },
        )?;
        let response: Result<WasmStoreStatusResponse, canic_contracts::dto::error::Error> =
            decode_public_reply(
                &envelope,
                self.maximum_response_bytes,
                protocol::store_inventory::storage::METHOD_ID,
                protocol::store_inventory::RESPONSE_LIMITS,
            )?;
        let reply =
            response.map_err(|error| response_failure(IcpJsonResponseError::Rejected(error)))?;
        Ok(store_inventory(reply))
    }

    fn metric_samples(
        &mut self,
        entry: &RegistryEntry,
        family: PublicMetricFamily,
    ) -> Result<MetricSamplesView, ObservationFailure> {
        let binding = entry
            .protocol_binding
            .as_ref()
            .ok_or(ObservationFailure::BindingUnavailable)?;
        if !binding
            .capabilities
            .contains(&RoleCapabilityKey::ObservabilityMetrics)
        {
            return Err(ObservationFailure::Unsupported);
        }
        let request = PublicMetricsRequest {
            family,
            page: PageRequest {
                limit: 256,
                offset: 0,
            },
        };
        let argument = ic_wire::native::encode_request(
            protocol::public_metrics::METHOD_ID,
            &request,
            protocol::public_metrics::REQUEST_LIMITS,
        )
        .map_err(|_| ObservationFailure::InvalidResponse)?;
        let envelope = self.query_encoded(
            entry,
            protocol::CANIC_PUBLIC_METRICS,
            &argument,
            NativeQueryContract {
                method_id: protocol::public_metrics::METHOD_ID,
                request: protocol::public_metrics::REQUEST_LIMITS,
                response: protocol::public_metrics::RESPONSE_LIMITS,
            },
        )?;
        let reply = decode_public_reply(
            &envelope,
            self.maximum_response_bytes,
            protocol::public_metrics::METHOD_ID,
            protocol::public_metrics::RESPONSE_LIMITS,
        )?;
        crate::observatory::ops::metrics::samples(reply, family)
    }

    fn cost_window(&mut self, entry: &RegistryEntry) -> Result<CostWindowView, ObservationFailure> {
        let binding = entry
            .protocol_binding
            .as_ref()
            .ok_or(ObservationFailure::BindingUnavailable)?;
        if !binding
            .capabilities
            .contains(&RoleCapabilityKey::ObservabilityHistory)
        {
            return Err(ObservationFailure::Unsupported);
        }
        let request = PublicHistoryRequest {
            family: PublicMetricFamily::Cycles,
            name: "balance".into(),
            canister_id: Some(
                ic_platform::Principal::from_text(&entry.pid)
                    .map_err(|_| ObservationFailure::InvalidResponse)?,
            ),
            page: PageRequest {
                limit: 1,
                offset: 0,
            },
        };
        let argument = ic_wire::native::encode_request(
            protocol::public_history::METHOD_ID,
            &request,
            protocol::public_history::REQUEST_LIMITS,
        )
        .map_err(|_| ObservationFailure::InvalidResponse)?;
        let envelope = self.query_encoded(
            entry,
            protocol::CANIC_PUBLIC_HISTORY,
            &argument,
            NativeQueryContract {
                method_id: protocol::public_history::METHOD_ID,
                request: protocol::public_history::REQUEST_LIMITS,
                response: protocol::public_history::RESPONSE_LIMITS,
            },
        )?;
        let reply: PublicHistorySnapshot = decode_public_reply(
            &envelope,
            self.maximum_response_bytes,
            protocol::public_history::METHOD_ID,
            protocol::public_history::RESPONSE_LIMITS,
        )?;
        if reply.points.entries.len() > 1 {
            return Err(ObservationFailure::InvalidResponse);
        }
        Ok(CostWindowView {
            canister_version: reply.canister_version,
            heap_started_at_ns: reply.heap_started_at_ns,
        })
    }

    fn attempts(&self) -> u64 {
        self.attempted_queries
    }
}

fn store_inventory(reply: WasmStoreStatusResponse) -> StoreInventoryView {
    let gc_state = match reply.gc.mode {
        WasmStoreGcMode::Normal => "normal",
        WasmStoreGcMode::Prepared => "prepared",
        WasmStoreGcMode::InProgress => "in_progress",
        WasmStoreGcMode::Clearing => "clearing",
        WasmStoreGcMode::Complete => "complete",
    };
    StoreInventoryView {
        occupied_bytes: reply.occupied_store_bytes,
        maximum_bytes: reply.max_store_bytes,
        remaining_bytes: reply.remaining_store_bytes,
        retained_templates: reply.template_count,
        retained_releases: reply.release_count,
        approved_catalog_entries: reply.inventory.approved_catalog_entries,
        expected_template_chunks: reply.inventory.expected_template_chunks,
        stored_template_chunks: reply.inventory.stored_template_chunks,
        fixture_sources: reply.inventory.fixture_sources,
        expected_fixture_chunks: reply.inventory.expected_fixture_chunks,
        stored_fixture_chunks: reply.inventory.stored_fixture_chunks,
        gc_state: gc_state.into(),
    }
}

fn response_failure(error: IcpJsonResponseError) -> ObservationFailure {
    match error {
        IcpJsonResponseError::Rejected(error) => ObservationFailure::Rejected {
            code: u32::from(error.raw_code()),
        },
        _ => ObservationFailure::InvalidResponse,
    }
}

fn decode_public_reply<T: serde::Serialize + serde::de::DeserializeOwned>(
    envelope: &str,
    maximum_bytes: usize,
    method_id: u64,
    mut limits: ic_wire::Limits,
) -> Result<T, ObservationFailure> {
    if envelope.len() > maximum_bytes {
        return Err(ObservationFailure::InvalidResponse);
    }
    let bytes = response_bytes(envelope).map_err(response_failure)?;
    limits.bytes = limits.bytes.min(maximum_bytes);
    ic_wire::native::decode_response(method_id, &bytes, limits)
        .map_err(|_| ObservationFailure::InvalidResponse)
}
