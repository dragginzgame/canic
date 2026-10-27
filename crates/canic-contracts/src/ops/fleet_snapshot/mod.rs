//! Lossless native snapshot conversion; storage encoding, hash inputs and policy remain unchanged.
use crate::{
    dto::{fleet_registry, fleet_snapshot as wire},
    ids,
};
impl From<fleet_registry::FleetRegistry> for wire::FleetRegistry {
    fn from(value: fleet_registry::FleetRegistry) -> Self {
        Self {
            authority: value.authority.into(),
            revision: value.revision,
            admission: value.admission.into(),
            component_specs: value.component_specs,
            fleet_subnet_roots: value
                .fleet_subnet_roots
                .into_iter()
                .map(Into::into)
                .collect(),
            services: value.services.into_iter().map(Into::into).collect(),
        }
    }
}
impl From<wire::FleetRegistry> for fleet_registry::FleetRegistry {
    fn from(value: wire::FleetRegistry) -> Self {
        Self {
            authority: value.authority.into(),
            revision: value.revision,
            admission: value.admission.into(),
            component_specs: value.component_specs,
            fleet_subnet_roots: value
                .fleet_subnet_roots
                .into_iter()
                .map(Into::into)
                .collect(),
            services: value.services.into_iter().map(Into::into).collect(),
        }
    }
}
impl From<fleet_registry::FleetSubnetRootEntry> for wire::FleetSubnetRootEntry {
    fn from(value: fleet_registry::FleetSubnetRootEntry) -> Self {
        Self {
            placement_subnet: value.placement_subnet.into_principal().into(),
            fleet_subnet_root: value.fleet_subnet_root.into(),
            component_admissions: value.component_admissions,
            component_topology_digest: value.component_topology_digest,
            active_release_set: value.active_release_set,
            limits: value.limits,
            funding: value.funding.into(),
            status: value.status,
        }
    }
}
impl From<wire::FleetSubnetRootEntry> for fleet_registry::FleetSubnetRootEntry {
    fn from(value: wire::FleetSubnetRootEntry) -> Self {
        Self {
            placement_subnet: ids::SubnetId::from_principal(value.placement_subnet.into()),
            fleet_subnet_root: value.fleet_subnet_root.into(),
            component_admissions: value.component_admissions,
            component_topology_digest: value.component_topology_digest,
            active_release_set: value.active_release_set,
            limits: value.limits,
            funding: value.funding.into(),
            status: value.status,
        }
    }
}
impl From<fleet_registry::FleetServiceBinding> for wire::FleetServiceBinding {
    fn from(value: fleet_registry::FleetServiceBinding) -> Self {
        Self {
            service: value.service,
            role: value.role,
            component_spec: value.component_spec,
            mode: value.mode,
            placement: value.placement,
            members: value.members.into_iter().map(Into::into).collect(),
        }
    }
}
impl From<wire::FleetServiceBinding> for fleet_registry::FleetServiceBinding {
    fn from(value: wire::FleetServiceBinding) -> Self {
        Self {
            service: value.service,
            role: value.role,
            component_spec: value.component_spec,
            mode: value.mode,
            placement: value.placement,
            members: value.members.into_iter().map(Into::into).collect(),
        }
    }
}
impl From<fleet_registry::FleetServiceComponentBinding> for wire::FleetServiceComponentBinding {
    fn from(value: fleet_registry::FleetServiceComponentBinding) -> Self {
        Self {
            member_purpose: value.member_purpose,
            component: value.component,
            fleet_subnet_root: value.fleet_subnet_root.into(),
            canister_id: value.canister_id.into(),
            group_placement: value.group_placement,
            member_path: value.member_path,
        }
    }
}
impl From<wire::FleetServiceComponentBinding> for fleet_registry::FleetServiceComponentBinding {
    fn from(value: wire::FleetServiceComponentBinding) -> Self {
        Self {
            member_purpose: value.member_purpose,
            component: value.component,
            fleet_subnet_root: value.fleet_subnet_root.into(),
            canister_id: value.canister_id.into(),
            group_placement: value.group_placement,
            member_path: value.member_path,
        }
    }
}
impl From<ids::FleetAdmissionSelector> for wire::FleetAdmissionSelector {
    fn from(value: ids::FleetAdmissionSelector) -> Self {
        match value {
            ids::FleetAdmissionSelector::Fleet => Self::Fleet,
            ids::FleetAdmissionSelector::ComponentSpec(value) => Self::ComponentSpec(value),
            ids::FleetAdmissionSelector::ComponentInstance(value) => Self::ComponentInstance(value),
            ids::FleetAdmissionSelector::FleetSubnetRoot(value) => {
                Self::FleetSubnetRoot(value.into_principal().into())
            }
        }
    }
}
impl From<wire::FleetAdmissionSelector> for ids::FleetAdmissionSelector {
    fn from(value: wire::FleetAdmissionSelector) -> Self {
        match value {
            wire::FleetAdmissionSelector::Fleet => Self::Fleet,
            wire::FleetAdmissionSelector::ComponentSpec(value) => Self::ComponentSpec(value),
            wire::FleetAdmissionSelector::ComponentInstance(value) => {
                Self::ComponentInstance(value)
            }
            wire::FleetAdmissionSelector::FleetSubnetRoot(value) => {
                Self::FleetSubnetRoot(ids::SubnetId::from_principal(value.into()))
            }
        }
    }
}
impl From<ids::FleetAdmissionRule> for wire::FleetAdmissionRule {
    fn from(value: ids::FleetAdmissionRule) -> Self {
        Self {
            selector: value.selector.into(),
            principals: value.principals.into_iter().map(Into::into).collect(),
        }
    }
}
impl From<wire::FleetAdmissionRule> for ids::FleetAdmissionRule {
    fn from(value: wire::FleetAdmissionRule) -> Self {
        Self {
            selector: value.selector.into(),
            principals: value.principals.into_iter().map(Into::into).collect(),
        }
    }
}
impl From<ids::FleetAdmissionPolicy> for wire::FleetAdmissionPolicy {
    fn from(value: ids::FleetAdmissionPolicy) -> Self {
        Self {
            schema_version: value.schema_version,
            fleet: value.fleet,
            generation: value.generation,
            fleet_principals: value.fleet_principals.into_iter().map(Into::into).collect(),
            rules: value.rules.into_iter().map(Into::into).collect(),
            policy_digest: value.policy_digest,
        }
    }
}
impl From<wire::FleetAdmissionPolicy> for ids::FleetAdmissionPolicy {
    fn from(value: wire::FleetAdmissionPolicy) -> Self {
        Self {
            schema_version: value.schema_version,
            fleet: value.fleet,
            generation: value.generation,
            fleet_principals: value.fleet_principals.into_iter().map(Into::into).collect(),
            rules: value.rules.into_iter().map(Into::into).collect(),
            policy_digest: value.policy_digest,
        }
    }
}
impl From<ids::FleetSubnetRootFundingAuthority> for wire::FleetSubnetRootFundingAuthority {
    fn from(value: ids::FleetSubnetRootFundingAuthority) -> Self {
        Self {
            root_funding: value.root_funding,
            icp_refill: value.icp_refill.map(Into::into),
        }
    }
}
impl From<wire::FleetSubnetRootFundingAuthority> for ids::FleetSubnetRootFundingAuthority {
    fn from(value: wire::FleetSubnetRootFundingAuthority) -> Self {
        Self {
            root_funding: value.root_funding,
            icp_refill: value.icp_refill.map(Into::into),
        }
    }
}
impl From<ids::FleetSubnetRootIcpRefillPolicy> for wire::FleetSubnetRootIcpRefillPolicy {
    fn from(value: ids::FleetSubnetRootIcpRefillPolicy) -> Self {
        Self {
            max_refill_e8s_per_call: value.max_refill_e8s_per_call,
            window_secs: value.window_secs,
            maximum_refill_e8s: value.maximum_refill_e8s,
            minimum_icp_balance_e8s: value.minimum_icp_balance_e8s,
            min_xdr_permyriad_per_icp: value.min_xdr_permyriad_per_icp,
            ledger_canister_id: value.ledger_canister_id.map(Into::into),
            cmc_canister_id: value.cmc_canister_id.map(Into::into),
            allow_ic_system_canister_overrides: value.allow_ic_system_canister_overrides,
            automatic: value.automatic,
        }
    }
}
impl From<wire::FleetSubnetRootIcpRefillPolicy> for ids::FleetSubnetRootIcpRefillPolicy {
    fn from(value: wire::FleetSubnetRootIcpRefillPolicy) -> Self {
        Self {
            max_refill_e8s_per_call: value.max_refill_e8s_per_call,
            window_secs: value.window_secs,
            maximum_refill_e8s: value.maximum_refill_e8s,
            minimum_icp_balance_e8s: value.minimum_icp_balance_e8s,
            min_xdr_permyriad_per_icp: value.min_xdr_permyriad_per_icp,
            ledger_canister_id: value.ledger_canister_id.map(Into::into),
            cmc_canister_id: value.cmc_canister_id.map(Into::into),
            allow_ic_system_canister_overrides: value.allow_ic_system_canister_overrides,
            automatic: value.automatic,
        }
    }
}

impl From<crate::dto::fleet_subnet_root::FleetSubnetRootCanisterSummary> for wire::RootInventory {
    fn from(value: crate::dto::fleet_subnet_root::FleetSubnetRootCanisterSummary) -> Self {
        Self {
            fleet_registry: value.fleet_registry.into(),
            placement_subnet: value.placement_subnet.into_principal().into(),
            fleet_subnet_root: value.fleet_subnet_root.into(),
            status: value.status,
            infrastructure_canisters: value.infrastructure_canisters,
            component_canisters: value.component_canisters,
            pooled_canisters: value.pooled_canisters,
            total_canisters: value.total_canisters,
        }
    }
}

impl From<wire::RootInventory> for crate::dto::fleet_subnet_root::FleetSubnetRootCanisterSummary {
    fn from(value: wire::RootInventory) -> Self {
        Self {
            fleet_registry: value.fleet_registry.into(),
            placement_subnet: ids::SubnetId::from_principal(value.placement_subnet.into()),
            fleet_subnet_root: value.fleet_subnet_root.into(),
            status: value.status,
            infrastructure_canisters: value.infrastructure_canisters,
            component_canisters: value.component_canisters,
            pooled_canisters: value.pooled_canisters,
            total_canisters: value.total_canisters,
        }
    }
}

impl From<ids::FleetSubnetRootBinding> for wire::FleetSubnetRootBinding {
    fn from(value: ids::FleetSubnetRootBinding) -> Self {
        Self {
            authority: value.authority.into(),
            placement_subnet: value.placement_subnet.into_principal().into(),
            fleet_subnet_root: value.fleet_subnet_root.into(),
            component_admissions: value.component_admissions,
            component_topology_digest: value.component_topology_digest,
            limits: value.limits,
            funding: value.funding.into(),
        }
    }
}

impl From<wire::FleetSubnetRootBinding> for ids::FleetSubnetRootBinding {
    fn from(value: wire::FleetSubnetRootBinding) -> Self {
        Self {
            authority: value.authority.into(),
            placement_subnet: ids::SubnetId::from_principal(value.placement_subnet.into()),
            fleet_subnet_root: value.fleet_subnet_root.into(),
            component_admissions: value.component_admissions,
            component_topology_digest: value.component_topology_digest,
            limits: value.limits,
            funding: value.funding.into(),
        }
    }
}
