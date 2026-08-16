//! Module: workflow::rpc::authority
//!
//! Responsibility: carry exact root-RPC topology evidence into core orchestration.
//! Does not own: Component Registry lookup, endpoint admission, or capability execution.
//! Boundary: the control plane resolves protected membership; core validates and consumes it.

use crate::{
    InternalError,
    cdk::types::Principal,
    dto::component_registry::ComponentRegistryHead,
    ids::{CanisterRole, ComponentInstanceId, ManagedCanisterBinding},
};

///
/// RootCapabilityCallerAuthority
///
/// Exact authenticated caller admitted to one Fleet Subnet Root capability.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RootCapabilityCallerAuthority {
    FleetSubnetRoot { canister_id: Principal },
    ComponentMember(RootCapabilityMemberAuthority),
}

///
/// RootCapabilityMemberAuthority
///
/// Compact protected facts extracted from one exact active Component Registry member.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootCapabilityMemberAuthority {
    canister_id: Principal,
    parent_canister_id: Principal,
    role: CanisterRole,
    component: ComponentInstanceId,
    registry: ComponentRegistryHead,
}

impl RootCapabilityMemberAuthority {
    pub fn try_from_active_member(
        member: ManagedCanisterBinding,
        registry: ComponentRegistryHead,
    ) -> Result<Self, InternalError> {
        let authority = match member {
            ManagedCanisterBinding::Component(binding) => Self {
                canister_id: binding.canister_id,
                parent_canister_id: binding.fleet_subnet_root,
                role: binding.role,
                component: binding.component,
                registry,
            },
            ManagedCanisterBinding::ComponentChild(binding) => Self {
                canister_id: binding.canister_id,
                parent_canister_id: binding.parent_canister_id,
                role: binding.role,
                component: binding.component.component,
                registry,
            },
        };
        if authority.component != authority.registry.component {
            return Err(InternalError::invariant());
        }
        Ok(authority)
    }

    #[must_use]
    pub const fn canister_id(&self) -> Principal {
        self.canister_id
    }

    #[must_use]
    pub const fn component(&self) -> ComponentInstanceId {
        self.component
    }
}

///
/// RootCapabilityParentAuthority
///
/// Exact active parent selected for one root-executed provision request.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RootCapabilityParentAuthority {
    ComponentMember { canister_id: Principal },
}

impl From<RootCapabilityMemberAuthority> for RootCapabilityParentAuthority {
    fn from(member: RootCapabilityMemberAuthority) -> Self {
        Self::ComponentMember {
            canister_id: member.canister_id,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RootCapabilityTargetAuthority {
    canister_id: Principal,
    parent_canister_id: Principal,
}

///
/// RootCapabilityAuthority
///
/// Protected Component Registry evidence bound to one root capability request.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootCapabilityAuthority {
    caller: RootCapabilityCallerAuthority,
    provision_parent: Option<RootCapabilityParentAuthority>,
    target: Option<RootCapabilityTargetAuthority>,
}

impl RootCapabilityAuthority {
    #[must_use]
    pub const fn new(caller: RootCapabilityCallerAuthority) -> Self {
        Self {
            caller,
            provision_parent: None,
            target: None,
        }
    }

    #[must_use]
    pub const fn with_provision_parent(mut self, parent: RootCapabilityParentAuthority) -> Self {
        self.provision_parent = Some(parent);
        self
    }

    #[must_use]
    pub fn with_target(mut self, target: ManagedCanisterBinding) -> Self {
        self.target = Some(RootCapabilityTargetAuthority::from(target));
        self
    }

    /// Attach target authority recovered from the exact durable removal journal.
    #[must_use]
    pub const fn with_recovery_target(
        mut self,
        canister_id: Principal,
        parent_canister_id: Principal,
    ) -> Self {
        self.target = Some(RootCapabilityTargetAuthority {
            canister_id,
            parent_canister_id,
        });
        self
    }

    pub(super) const fn caller_canister_id(&self) -> Principal {
        match &self.caller {
            RootCapabilityCallerAuthority::FleetSubnetRoot { canister_id } => *canister_id,
            RootCapabilityCallerAuthority::ComponentMember(member) => member.canister_id,
        }
    }

    pub(super) const fn caller_is_fleet_subnet_root(&self) -> bool {
        matches!(
            &self.caller,
            RootCapabilityCallerAuthority::FleetSubnetRoot { .. }
        )
    }

    pub(super) const fn caller_parent_canister_id(&self) -> Option<Principal> {
        match &self.caller {
            RootCapabilityCallerAuthority::FleetSubnetRoot { .. } => None,
            RootCapabilityCallerAuthority::ComponentMember(member) => {
                Some(member.parent_canister_id)
            }
        }
    }

    pub(super) const fn caller_role(&self) -> Option<&CanisterRole> {
        match &self.caller {
            RootCapabilityCallerAuthority::FleetSubnetRoot { .. } => None,
            RootCapabilityCallerAuthority::ComponentMember(member) => Some(&member.role),
        }
    }

    pub(super) const fn caller_component(&self) -> Option<ComponentInstanceId> {
        match &self.caller {
            RootCapabilityCallerAuthority::FleetSubnetRoot { .. } => None,
            RootCapabilityCallerAuthority::ComponentMember(member) => Some(member.component),
        }
    }

    pub(super) const fn caller_registry(&self) -> Option<&ComponentRegistryHead> {
        match &self.caller {
            RootCapabilityCallerAuthority::FleetSubnetRoot { .. } => None,
            RootCapabilityCallerAuthority::ComponentMember(member) => Some(&member.registry),
        }
    }

    pub(super) fn provision_parent_canister_id(&self) -> Option<Principal> {
        self.provision_parent.as_ref().map(|parent| match parent {
            RootCapabilityParentAuthority::ComponentMember { canister_id } => *canister_id,
        })
    }

    pub(super) const fn has_provision_parent(&self) -> bool {
        self.provision_parent.is_some()
    }

    pub(super) fn target_canister_id(&self) -> Option<Principal> {
        self.target.as_ref().map(|target| target.canister_id)
    }

    pub(super) fn target_parent_canister_id(&self) -> Option<Principal> {
        self.target.as_ref().map(|target| target.parent_canister_id)
    }

    pub(super) const fn has_target(&self) -> bool {
        self.target.is_some()
    }
}

impl From<ManagedCanisterBinding> for RootCapabilityTargetAuthority {
    fn from(member: ManagedCanisterBinding) -> Self {
        match member {
            ManagedCanisterBinding::Component(binding) => Self {
                canister_id: binding.canister_id,
                parent_canister_id: binding.fleet_subnet_root,
            },
            ManagedCanisterBinding::ComponentChild(binding) => Self {
                canister_id: binding.canister_id,
                parent_canister_id: binding.parent_canister_id,
            },
        }
    }
}
