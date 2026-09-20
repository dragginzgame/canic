//! Exact role-specific queries and bounded conversion into passive host facts.

mod process;

use crate::{
    icp::{IcpCli, IcpJsonResponseError, response_bytes},
    observatory::view::{
        CostSamplesView, CostWindowView, ObservationFailure, RoleFundingView, RoleOverviewView,
        RootEstateView, StoreInventoryView,
    },
    protocol_binding::resolve_registry_protocol_binding,
    registry::RegistryEntry,
};
use candid::{CandidType, Deserialize, Principal};
use canic_control_plane::{
    dto::{
        fleet_coordinator::CoordinatorFundingStatusResponse, root::RootFundingStatusResponse,
        template::WasmStoreStatusResponse,
    },
    ids::WasmStoreGcMode,
};
use canic_core::{
    dto::{
        page::PageRequest,
        pool::{CanisterPoolResponse, CanisterPoolStatusRequest},
        public_status::{
            PublicHistoryRequest, PublicHistorySnapshot, PublicMetricFamily, PublicMetricsRequest,
            PublicMetricsSnapshot,
        },
        role::RoleOverviewResponse,
    },
    protocol,
};
use std::path::Path;

/// Independent role observations; implementations must not retry or substitute another role.
pub trait ObservatoryTransport {
    fn overview(&mut self, entry: &RegistryEntry) -> Result<RoleOverviewView, ObservationFailure>;
    fn funding(&mut self, entry: &RegistryEntry) -> Result<RoleFundingView, ObservationFailure>;
    fn estate(&mut self, entry: &RegistryEntry) -> Result<RootEstateView, ObservationFailure>;
    fn store(&mut self, entry: &RegistryEntry) -> Result<StoreInventoryView, ObservationFailure>;
    fn cost_samples(
        &mut self,
        entry: &RegistryEntry,
        family: PublicMetricFamily,
    ) -> Result<CostSamplesView, ObservationFailure>;
    fn cost_window(&mut self, entry: &RegistryEntry) -> Result<CostWindowView, ObservationFailure>;
    fn attempts(&self) -> u64;
}

/// Query-only ICP adapter. The budget bounds envelope admission before Candid decoding.
pub struct IcpObservatoryTransport<'a> {
    pub icp: &'a IcpCli,
    pub root: &'a Path,
    pub environment: &'a str,
    pub maximum_response_bytes: usize,
    pub attempted_queries: u64,
    pub query_timeout: std::time::Duration,
    pub deadline: std::time::Instant,
    pub compatibility: Option<Result<(), ObservationFailure>>,
}

#[derive(CandidType)]
enum OverviewRequest {
    Overview,
}
#[derive(CandidType, Deserialize)]
enum OverviewResponse {
    Overview(RoleOverviewResponse),
}
#[derive(CandidType)]
enum FundingRequest {
    Funding,
}
#[derive(CandidType, Deserialize)]
enum CoordinatorResponse {
    Funding(CoordinatorFundingStatusResponse),
}
#[derive(CandidType, Deserialize)]
enum RootFundingResponse {
    Funding(RootFundingStatusResponse),
}
#[derive(CandidType)]
enum EstateRequest {
    Pool(CanisterPoolStatusRequest),
}
#[derive(CandidType, Deserialize)]
enum EstateResponse {
    Pool(CanisterPoolResponse),
}
#[derive(CandidType)]
enum StoreRequest {
    Storage,
}
#[derive(CandidType, Deserialize)]
enum StoreResponse {
    Storage(WasmStoreStatusResponse),
}

#[derive(CandidType)]
enum CostRequest {
    Metrics(PublicMetricsRequest),
    History(PublicHistoryRequest),
}

#[derive(CandidType, Deserialize)]
enum CostResponse {
    Metrics(PublicMetricsSnapshot),
    History(PublicHistorySnapshot),
}

impl IcpObservatoryTransport<'_> {
    fn query<I: CandidType, O: CandidType + serde::de::DeserializeOwned>(
        &mut self,
        entry: &RegistryEntry,
        method: &'static str,
        request: &I,
    ) -> Result<O, ObservationFailure> {
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
        let binding = resolve_registry_protocol_binding(self.root, self.environment, entry)
            .map_err(|_| ObservationFailure::BindingUnavailable)?;
        let canister =
            Principal::from_text(&entry.pid).map_err(|_| ObservationFailure::InvalidResponse)?;
        let argument =
            candid::encode_one(request).map_err(|_| ObservationFailure::InvalidResponse)?;
        let argument = crate::icp::write_candid_argument_file(&argument)
            .map_err(|_| ObservationFailure::TransportUnavailable)?;
        let mut command = self.icp.bounded_query_command(
            &canister.to_text(),
            method,
            &argument,
            binding.candid_path(),
        );
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
        let cleanup = std::fs::remove_file(argument);
        let envelope = result?;
        cleanup.map_err(|_| ObservationFailure::TransportUnavailable)?;
        decode_reply(&envelope, self.maximum_response_bytes)
    }
}

impl ObservatoryTransport for IcpObservatoryTransport<'_> {
    fn overview(&mut self, entry: &RegistryEntry) -> Result<RoleOverviewView, ObservationFailure> {
        let OverviewResponse::Overview(reply) = self.query(
            entry,
            protocol::CANIC_PUBLIC_STATUS,
            &OverviewRequest::Overview,
        )?;
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
                let CoordinatorResponse::Funding(reply) = self.query(
                    entry,
                    protocol::CANIC_OBSERVABILITY,
                    &FundingRequest::Funding,
                )?;
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
                let RootFundingResponse::Funding(reply) =
                    self.query(entry, protocol::CANIC_ROOT_STATUS, &FundingRequest::Funding)?;
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
        let EstateResponse::Pool(reply) = self.query(
            entry,
            protocol::CANIC_ROOT_STATUS,
            &EstateRequest::Pool(CanisterPoolStatusRequest {
                start_after: None,
                limit: 1,
            }),
        )?;
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
        let StoreResponse::Storage(reply) = self.query(
            entry,
            protocol::CANIC_WASM_STORE_CATALOG,
            &StoreRequest::Storage,
        )?;
        Ok(store_inventory(reply))
    }

    fn cost_samples(
        &mut self,
        entry: &RegistryEntry,
        family: PublicMetricFamily,
    ) -> Result<CostSamplesView, ObservationFailure> {
        let reply: CostResponse = self.query(
            entry,
            protocol::CANIC_PUBLIC_STATUS,
            &CostRequest::Metrics(PublicMetricsRequest {
                family,
                page: PageRequest {
                    limit: 256,
                    offset: 0,
                },
            }),
        )?;
        let CostResponse::Metrics(reply) = reply else {
            return Err(ObservationFailure::InvalidResponse);
        };
        crate::observatory::ops::cost::samples(reply, family)
    }

    fn cost_window(&mut self, entry: &RegistryEntry) -> Result<CostWindowView, ObservationFailure> {
        let reply: CostResponse = self.query(
            entry,
            protocol::CANIC_PUBLIC_STATUS,
            &CostRequest::History(PublicHistoryRequest {
                family: PublicMetricFamily::Cycles,
                name: "balance".into(),
                canister_id: Some(
                    Principal::from_text(&entry.pid)
                        .map_err(|_| ObservationFailure::InvalidResponse)?,
                ),
                page: PageRequest {
                    limit: 1,
                    offset: 0,
                },
            }),
        )?;
        let CostResponse::History(reply) = reply else {
            return Err(ObservationFailure::InvalidResponse);
        };
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

fn decode_reply<O: CandidType + serde::de::DeserializeOwned>(
    envelope: &str,
    maximum_bytes: usize,
) -> Result<O, ObservationFailure> {
    let bytes = response_bytes(envelope).map_err(response_failure)?;
    let mut config = candid::de::DecoderConfig::new();
    // Bound decoder work as well as captured bytes, including skipped Candid fields.
    config.set_decoding_quota(maximum_bytes.saturating_mul(64));
    config.set_skipping_quota(maximum_bytes);
    let reply: Result<O, canic_core::dto::error::Error> =
        candid::utils::decode_one_with_config(&bytes, &config)
            .map_err(|_| ObservationFailure::InvalidResponse)?;
    reply.map_err(|error| ObservationFailure::Rejected {
        code: u32::from(error.raw_code()),
    })
}
