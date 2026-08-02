//! Module: workflow::rpc::lifecycle
//!
//! Responsibility: define the root capability lifecycle driver boundary.
//! Does not own: Component Registry persistence, replay storage, or endpoint authentication.
//! Boundary: core replay orchestration delegates protected child lifecycle to the control plane.

use crate::{
    InternalError,
    cdk::types::Principal,
    dto::component_registry::ComponentRegistryHead,
    ids::{CanisterRole, ComponentInstanceId},
};
use async_trait::async_trait;

///
/// RootComponentChildProvisionRequest
///
/// Exact replay-bound Component Child lifecycle authority passed to the control plane.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentChildProvisionRequest {
    pub operation_id: [u8; 32],
    pub component: ComponentInstanceId,
    pub expected_registry: ComponentRegistryHead,
    pub child_role: CanisterRole,
    pub application_init_args: Option<Vec<u8>>,
}

///
/// RootCapabilityLifecycleExecutor
///
/// Driver implemented by the root control plane for Component-bound lifecycle effects.
///

#[async_trait]
pub trait RootCapabilityLifecycleExecutor: Send + Sync {
    /// Resume one exact child operation through active Registry membership.
    async fn provision_component_child(
        &self,
        request: RootComponentChildProvisionRequest,
    ) -> Result<Principal, InternalError>;
}
