//! Module: blob_service::ops::gateways
//!
//! Responsibility: adapt gateway registry borrowing and bounded provider queries.
//! Does not own: gateway membership policy or refresh orchestration.
//! Boundary: the shared gateway workflow rechecks state after external calls.

use crate::ops;

use ic_blob_storage::ops::{
    caffeine::gateway::GatewayReplyLimits,
    service::gateways::{StableGatewayRegistry, access::GatewayRegistryAccess},
};
pub(crate) struct GatewayHost;
impl GatewayRegistryAccess for GatewayHost {
    type Memory = ops::memory::Memory;
    fn with_gateway_registry<R>(
        &self,
        operation: impl FnOnce(&mut StableGatewayRegistry<Self::Memory>) -> R,
    ) -> R {
        ops::mutate(|stores| operation(&mut stores.gateways))
    }
}
pub(crate) fn limits() -> GatewayReplyLimits {
    GatewayReplyLimits {
        max_bytes: 65_536.try_into().unwrap(),
        decoding_quota: 500_000.try_into().unwrap(),
        skipping_quota: 1000.try_into().unwrap(),
        max_type_entries: 32.try_into().unwrap(),
    }
}
