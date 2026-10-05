//! Module: ids::caller_authority
//!
//! Exact immutable installation identities used by managed-caller publication.

use crate::ids::{ComponentBinding, FleetRegistryAuthority, ManagedCanisterBinding};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

/// Registered source or receiver identity for one physical installation.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CallerInstallation {
    pub binding: ManagedCanisterBinding,
    pub install_id: [u8; 32],
    pub component_install_id: [u8; 32],
}

/// Exact owning Component installation covered by a whole-tree denial fence.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CallerComponentInstallation {
    pub binding: ComponentBinding,
    pub install_id: [u8; 32],
}

impl CallerInstallation {
    /// Return the exact physical caller; descendants retain their own Principal.
    #[must_use]
    pub const fn canister(&self) -> Principal {
        match &self.binding {
            ManagedCanisterBinding::Component(binding) => binding.canister_id,
            ManagedCanisterBinding::ComponentChild(binding) => binding.canister_id,
        }
    }

    /// Return the owning Component without replacing a descendant's own role.
    #[must_use]
    pub const fn component(&self) -> &ComponentBinding {
        match &self.binding {
            ManagedCanisterBinding::Component(binding) => binding,
            ManagedCanisterBinding::ComponentChild(binding) => &binding.component,
        }
    }

    /// Return the registered role of this exact canister.
    #[must_use]
    pub const fn role(&self) -> &crate::ids::CanisterRole {
        match &self.binding {
            ManagedCanisterBinding::Component(binding) => &binding.role,
            ManagedCanisterBinding::ComponentChild(binding) => &binding.role,
        }
    }
}

/// Issuing Root installation and its exact Fleet Registry authority.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CallerRootAuthority {
    pub registry: FleetRegistryAuthority,
    pub root: Principal,
    pub install_id: [u8; 32],
}

/// Complete receiver publication audience, including immutable compiled policy identity.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CallerReceiverAuthority {
    pub receiver: CallerInstallation,
    pub issuer: CallerRootAuthority,
    pub policy_digest: [u8; 32],
}
