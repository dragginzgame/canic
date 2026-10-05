//! Module: testing::managed_component_group::model
//!
//! Responsibility: define the bounded public inputs and read-only fixture catalogue.
//! Does not own: authority compilation, PocketIC effects, or application assertions.
//! Boundary: callers provide role Wasms; Canic derives every protected identity from config.

use crate::ids::{CanisterRole, ComponentGroupMemberPath, ComponentSpecId};
use candid::Principal;

use super::DEFAULT_INSTALL_CYCLES;

/// Application-owned init bytes, optionally bound to the newly allocated canister.
///
/// This test input never changes Canic's protected payload or allocated identity.
#[derive(Clone, Debug)]
pub enum ManagedApplicationInit {
    /// No nested application arguments.
    None,
    /// Already encoded application arguments shared by instances of this role.
    Encoded(Vec<u8>),
    /// Encode application arguments after the actual canister ID is known.
    /// The encoder runs synchronously; its panic propagates to the fixture caller.
    ForCanister(fn(Principal) -> Vec<u8>),
}

impl ManagedApplicationInit {
    pub(super) fn for_canister(&self, canister: Principal) -> Option<Vec<u8>> {
        match self {
            Self::None => None,
            Self::Encoded(bytes) => Some(bytes.clone()),
            Self::ForCanister(encode) => Some(encode(canister)),
        }
    }
}

/// Exact built Wasm and optional application init bytes for one configured role.
#[derive(Clone, Debug)]
pub struct ManagedRoleQualificationArtifact {
    /// Application init bytes passed after Canic's protected init payload.
    pub application_init_args: ManagedApplicationInit,
    /// Cycles added to each fixture canister using this role.
    pub install_cycles: u128,
    /// Exact configured role implemented by the Wasm.
    pub role: CanisterRole,
    /// Exact managed Wasm under qualification.
    pub wasm: Vec<u8>,
}

impl ManagedRoleQualificationArtifact {
    /// Construct one role artifact with bounded test defaults.
    #[must_use]
    pub const fn new(role: CanisterRole, wasm: Vec<u8>) -> Self {
        Self {
            application_init_args: ManagedApplicationInit::None,
            install_cycles: DEFAULT_INSTALL_CYCLES,
            role,
            wasm,
        }
    }
}

/// Exact downstream inputs for one managed Component Group qualification fixture.
pub struct ManagedComponentGroupQualificationInput<'a> {
    /// Fleet-admitted callers embedded in every enrolled local projection.
    pub admitted_principals: Vec<Principal>,
    /// Exact Canic configuration source compiled into every tested Wasm.
    /// The caller must retain and compare these build-input bytes before constructing this input;
    /// the release manifest binds artifacts and topology, not every runtime publication setting.
    /// This source never overrides configuration embedded in the supplied Wasms.
    pub app_config_source: &'a str,
    /// Exact Component Group deployment materialized by the fixture.
    pub component_group_deployment: &'a str,
    /// Placement ordinal used by the synthetic local group authority.
    pub component_group_ordinal: u32,
    /// Exact artifact for every role contained by the selected Component Specs.
    pub role_artifacts: Vec<ManagedRoleQualificationArtifact>,
    /// Finalized release-build identity embedded in every managed init payload.
    pub release_build_id: &'a str,
}

impl<'a> ManagedComponentGroupQualificationInput<'a> {
    /// Construct one complete group input with the first placement ordinal.
    #[must_use]
    pub const fn new(
        app_config_source: &'a str,
        component_group_deployment: &'a str,
        release_build_id: &'a str,
        admitted_principals: Vec<Principal>,
        role_artifacts: Vec<ManagedRoleQualificationArtifact>,
    ) -> Self {
        Self {
            admitted_principals,
            app_config_source,
            component_group_deployment,
            component_group_ordinal: 0,
            role_artifacts,
            release_build_id,
        }
    }
}

/// One installed node in the caller-readable managed Component tree catalogue.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedComponentNode {
    /// Complete protected binding installed into the node.
    pub binding: crate::ids::ManagedCanisterBinding,
    /// Exact installed Principal.
    pub canister_id: Principal,
    /// Component Group member path owning the complete Component tree.
    pub component_group_member: ComponentGroupMemberPath,
    /// Exact Component Spec owning the tree.
    pub component_spec: ComponentSpecId,
    /// Direct parent for a managed child; absent for a top-level Component.
    pub parent_canister_id: Option<Principal>,
    /// Exact configured role installed at this Principal.
    pub role: CanisterRole,
}
