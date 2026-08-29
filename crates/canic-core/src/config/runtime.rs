//! Module: config::runtime
//!
//! Responsibility: own the build-compiled immutable configuration authority used by one exact
//! role runtime.
//! Does not own: source TOML parsing, host planning, Root control-plane configuration, or storage.
//! Boundary: build tooling projects one validated App model into this runtime-only authority.

#[cfg(any(not(target_arch = "wasm32"), test))]
use super::schema::CanisterConfig;
#[cfg(any(not(target_arch = "wasm32"), test))]
use super::{ComponentDeploymentConfigurationDigestError, ConfigModel};
use super::{
    ComponentDeploymentLimits, ComponentDeploymentPurpose, ComponentTopology,
    FlattenedComponentGroupDeploymentMember,
    schema::{
        AuthConfig, CanisterAuthConfig, CanisterKind, CyclesFundingPolicyConfig, FleetInitMode,
        IndexConfig, LocalApplicationAuthorizationConfig, LogConfig, ScalingConfig, ShardingConfig,
        StandardsCanisterConfig, TopupPolicy,
    },
};
use crate::{
    InternalError,
    dto::component_deployment::ProtectedComponentDeployment,
    ids::{
        CanisterRole, ComponentBinding, ComponentDeploymentConfigurationDigest,
        ComponentGroupDeploymentId, ComponentGroupSpecId, ComponentSpecId,
    },
};
#[cfg(any(not(target_arch = "wasm32"), test))]
use std::collections::BTreeSet;
use std::{cell::RefCell, sync::Arc};
#[cfg(any(not(target_arch = "wasm32"), test))]
use thiserror::Error as ThisError;

/// One exact role configuration within a compiled Component Spec.
#[derive(Clone, Debug)]
pub struct RuntimeCanisterAuthority {
    pub component_spec: Option<ComponentSpecId>,
    pub role: CanisterRole,
    pub config: RuntimeCanisterConfig,
}

/// Runtime-only fields consumed by the exact compiled role and its admitted children.
#[derive(Clone, Debug)]
pub struct RuntimeCanisterConfig {
    pub kind: CanisterKind,
    pub topup: Option<TopupPolicy>,
    pub cycles_funding: CyclesFundingPolicyConfig,
    pub scaling: Option<ScalingConfig>,
    pub sharding: Option<ShardingConfig>,
    pub index: Option<IndexConfig>,
    pub auth: CanisterAuthConfig,
    pub standards: StandardsCanisterConfig,
}

#[cfg(any(not(target_arch = "wasm32"), test))]
impl From<CanisterConfig> for RuntimeCanisterConfig {
    fn from(config: CanisterConfig) -> Self {
        Self {
            kind: config.kind,
            topup: config.topup,
            cycles_funding: config.cycles_funding,
            scaling: config.scaling,
            sharding: config.sharding,
            index: config.index,
            auth: config.auth,
            standards: config.standards,
        }
    }
}

/// One exact grouped-deployment member admitted by the compiled App authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeDeploymentMemberAuthority {
    pub deployment: ComponentGroupDeploymentId,
    pub component_group: ComponentGroupSpecId,
    pub member: FlattenedComponentGroupDeploymentMember,
}

/// One exact unique application-authorization declaration addressable by role.
#[derive(Clone, Debug)]
pub struct RuntimeApplicationAuthorization {
    pub role: CanisterRole,
    pub config: LocalApplicationAuthorizationConfig,
}

/// Complete immutable runtime projection for one exact role artifact.
#[derive(Clone, Debug)]
pub struct RoleRuntimeAuthority {
    pub role: CanisterRole,
    pub app_init_mode: FleetInitMode,
    pub log: LogConfig,
    pub auth: AuthConfig,
    pub fleet_admission: bool,
    pub global_icrc21: bool,
    pub component_topology: ComponentTopology,
    pub canisters: Vec<RuntimeCanisterAuthority>,
    pub configuration_digest: ComponentDeploymentConfigurationDigest,
    pub deployment_members: Vec<RuntimeDeploymentMemberAuthority>,
    pub application_authorizations: Vec<RuntimeApplicationAuthorization>,
}

/// Build-time rejection while compiling a role runtime authority.
#[cfg(any(not(target_arch = "wasm32"), test))]
#[derive(Debug, ThisError)]
pub enum RoleRuntimeAuthorityError {
    #[error(transparent)]
    Configuration(Box<ComponentDeploymentConfigurationDigestError>),

    #[error("runtime role {0} is not declared by the validated App configuration")]
    UnknownRole(CanisterRole),
}

#[cfg(any(not(target_arch = "wasm32"), test))]
impl From<ComponentDeploymentConfigurationDigestError> for RoleRuntimeAuthorityError {
    fn from(error: ComponentDeploymentConfigurationDigestError) -> Self {
        Self::Configuration(Box::new(error))
    }
}

impl RoleRuntimeAuthority {
    /// Compile the one runtime projection used by an exact declared role.
    #[cfg(any(not(target_arch = "wasm32"), test))]
    pub fn compile(
        config: &ConfigModel,
        role: &CanisterRole,
    ) -> Result<Self, RoleRuntimeAuthorityError> {
        let declaration = config
            .roles
            .get(role)
            .ok_or_else(|| RoleRuntimeAuthorityError::UnknownRole(role.clone()))?;
        let configuration = config.compile_component_deployment_configuration()?;
        let configuration_digest = configuration.digest()?;
        let relevant_component_specs = config
            .component_specs_for_role(role)
            .map(|(component_spec, _config)| component_spec.clone())
            .collect::<BTreeSet<_>>();
        let canisters = if role.is_root() {
            root_runtime_canister_authorities(config)
        } else {
            runtime_canister_authorities(config, role, &relevant_component_specs)
        };
        let supports_delegated_token_issuance = canisters
            .iter()
            .filter(|authority| &authority.role == role)
            .any(|authority| authority.config.auth.delegated_token_issuer);
        let application_authorizations = if supports_delegated_token_issuance {
            runtime_application_authorizations(config)
        } else {
            Vec::new()
        };
        // A Component validates the complete Root admission projection before accepting its own
        // binding. Keep that protected topology authority intact even though mutable/runtime
        // configuration is pruned to the exact role and its admitted descendants.
        let component_topology = if role.is_root() {
            ComponentTopology {
                component_specs: Vec::new(),
                provisioning_grants: Vec::new(),
            }
        } else {
            configuration.component_topology.clone()
        };
        let deployment_members = configuration
            .deployment_topology
            .component_group_deployments
            .iter()
            .flat_map(|deployment| {
                deployment
                    .members
                    .iter()
                    .filter(|member| relevant_component_specs.contains(&member.component_spec))
                    .cloned()
                    .map(|member| RuntimeDeploymentMemberAuthority {
                        deployment: deployment.deployment.clone(),
                        component_group: deployment.component_group.clone(),
                        member,
                    })
            })
            .collect();

        Ok(Self {
            role: role.clone(),
            app_init_mode: config.app.init_mode,
            log: config.log.clone(),
            auth: config.auth.clone(),
            fleet_admission: declaration.fleet_admission,
            global_icrc21: config
                .standards
                .as_ref()
                .is_some_and(|standards| standards.icrc21),
            component_topology,
            canisters,
            configuration_digest,
            deployment_members,
            application_authorizations,
        })
    }

    /// Compile the built-in Store projection without inventing an App role declaration.
    #[cfg(any(not(target_arch = "wasm32"), test))]
    pub fn compile_wasm_store(config: &ConfigModel) -> Result<Self, RoleRuntimeAuthorityError> {
        let configuration = config.compile_component_deployment_configuration()?;
        let configuration_digest = configuration.digest()?;
        Ok(Self {
            role: CanisterRole::WASM_STORE,
            app_init_mode: config.app.init_mode,
            log: config.log.clone(),
            auth: config.auth.clone(),
            fleet_admission: false,
            global_icrc21: config
                .standards
                .as_ref()
                .is_some_and(|standards| standards.icrc21),
            component_topology: ComponentTopology {
                component_specs: Vec::new(),
                provisioning_grants: Vec::new(),
            },
            canisters: vec![RuntimeCanisterAuthority {
                component_spec: None,
                role: CanisterRole::WASM_STORE,
                config: super::schema::implicit_wasm_store_canister_config().into(),
            }],
            configuration_digest,
            deployment_members: Vec::new(),
            application_authorizations: Vec::new(),
        })
    }

    #[must_use]
    pub fn canister(
        &self,
        component_spec: Option<&ComponentSpecId>,
        role: &CanisterRole,
    ) -> Option<RuntimeCanisterConfig> {
        self.canisters
            .iter()
            .find(|authority| {
                &authority.role == role && authority.component_spec.as_ref() == component_spec
            })
            .map(|authority| authority.config.clone())
    }

    #[must_use]
    pub fn canister_by_role(&self, role: &CanisterRole) -> Option<RuntimeCanisterConfig> {
        let mut matches = self
            .canisters
            .iter()
            .filter(|authority| &authority.role == role);
        let authority = matches.next()?;
        matches.next().is_none().then(|| authority.config.clone())
    }

    #[must_use]
    pub fn component_spec_for_role(&self, role: &CanisterRole) -> Option<ComponentSpecId> {
        let mut matches = self
            .canisters
            .iter()
            .filter(|authority| &authority.role == role)
            .filter_map(|authority| authority.component_spec.as_ref());
        let component_spec = matches.next()?;
        matches
            .all(|candidate| candidate == component_spec)
            .then(|| component_spec.clone())
    }

    #[must_use]
    pub fn local_application_authorization(
        &self,
        role: &CanisterRole,
    ) -> Option<LocalApplicationAuthorizationConfig> {
        self.application_authorizations
            .iter()
            .find(|authority| &authority.role == role)
            .map(|authority| authority.config.clone())
    }

    pub fn validate_protected_component_deployment(
        &self,
        context: &ProtectedComponentDeployment,
        owning_component: &ComponentBinding,
    ) -> Result<(), InternalError> {
        match context {
            ProtectedComponentDeployment::UngroupedOrdinary { binding } => (binding
                == owning_component)
                .then_some(())
                .ok_or_else(InternalError::invalid_input),
            ProtectedComponentDeployment::GroupMember {
                binding,
                configuration_digest,
                group_placement,
                component_group,
                member_path,
                purpose,
                labels,
                limits,
            } => {
                if binding != owning_component || configuration_digest != &self.configuration_digest
                {
                    return Err(InternalError::invalid_input());
                }
                let Some(authority) = self.deployment_members.iter().find(|authority| {
                    authority.deployment == group_placement.deployment
                        && authority.member.member_path == *member_path
                }) else {
                    return Err(InternalError::invalid_input());
                };
                validate_deployment_member(
                    authority,
                    owning_component,
                    component_group,
                    purpose,
                    labels,
                    limits,
                )
            }
        }
    }
}

#[cfg(any(not(target_arch = "wasm32"), test))]
fn runtime_canister_authorities(
    config: &ConfigModel,
    role: &CanisterRole,
    relevant_component_specs: &BTreeSet<ComponentSpecId>,
) -> Vec<RuntimeCanisterAuthority> {
    config
        .component_specs
        .iter()
        .filter(|(component_spec, _config)| relevant_component_specs.contains(*component_spec))
        .flat_map(|(component_spec, config)| {
            let admitted_roles = config
                .spawn_grants
                .get(role)
                .map(|grants| grants.keys().cloned().collect::<BTreeSet<_>>())
                .unwrap_or_default();
            config
                .canister_configs()
                .filter(move |(candidate, _config)| {
                    candidate == &role || admitted_roles.contains(*candidate)
                })
                .map(|(role, config)| RuntimeCanisterAuthority {
                    component_spec: Some(component_spec.clone()),
                    role: role.clone(),
                    config: config.into(),
                })
        })
        .collect()
}

#[cfg(any(not(target_arch = "wasm32"), test))]
fn root_runtime_canister_authorities(config: &ConfigModel) -> Vec<RuntimeCanisterAuthority> {
    std::iter::once(RuntimeCanisterAuthority {
        component_spec: None,
        role: CanisterRole::ROOT,
        config: super::schema::implicit_root_canister_config().into(),
    })
    .chain(std::iter::once(RuntimeCanisterAuthority {
        component_spec: None,
        role: CanisterRole::WASM_STORE,
        config: super::schema::implicit_wasm_store_canister_config().into(),
    }))
    .chain(
        config
            .component_specs
            .iter()
            .flat_map(|(component_spec, spec)| {
                spec.canister_configs()
                    .map(|(role, canister)| RuntimeCanisterAuthority {
                        component_spec: Some(component_spec.clone()),
                        role: role.clone(),
                        config: canister.into(),
                    })
            }),
    )
    .collect()
}

#[cfg(any(not(target_arch = "wasm32"), test))]
fn runtime_application_authorizations(
    config: &ConfigModel,
) -> Vec<RuntimeApplicationAuthorization> {
    config
        .roles
        .keys()
        .filter_map(|role| {
            let (_component_spec, spec) = config.component_spec_for_role(role)?;
            let authorization = spec
                .get_canister(role)?
                .auth
                .local_application_authorization?;
            Some(RuntimeApplicationAuthorization {
                role: role.clone(),
                config: authorization,
            })
        })
        .collect()
}

fn validate_deployment_member(
    authority: &RuntimeDeploymentMemberAuthority,
    owning_component: &ComponentBinding,
    component_group: &ComponentGroupSpecId,
    purpose: &ComponentDeploymentPurpose,
    labels: &[super::ComponentDeploymentLabel],
    limits: &ComponentDeploymentLimits,
) -> Result<(), InternalError> {
    if !deployment_member_identity_matches(authority, owning_component, component_group)
        || !deployment_member_policy_matches(authority, purpose, labels, limits)
    {
        return Err(InternalError::invalid_input());
    }
    Ok(())
}

fn deployment_member_identity_matches(
    authority: &RuntimeDeploymentMemberAuthority,
    owning_component: &ComponentBinding,
    component_group: &ComponentGroupSpecId,
) -> bool {
    component_group == &authority.component_group
        && owning_component.component_spec == authority.member.component_spec
        && owning_component.spec_hash == authority.member.component_spec_hash
}

fn deployment_member_policy_matches(
    authority: &RuntimeDeploymentMemberAuthority,
    purpose: &ComponentDeploymentPurpose,
    labels: &[super::ComponentDeploymentLabel],
    limits: &ComponentDeploymentLimits,
) -> bool {
    purpose == &authority.member.purpose
        && labels == authority.member.labels
        && limits == &authority.member.limits
}

struct InstalledRoleRuntimeAuthority {
    authority: Arc<RoleRuntimeAuthority>,
}

thread_local! {
    static ROLE_RUNTIME_AUTHORITY: RefCell<Option<InstalledRoleRuntimeAuthority>> =
        const { RefCell::new(None) };
}

/// Runtime installation and lookup owner for the one compiled role authority.
pub struct RoleRuntimeConfig;

impl RoleRuntimeConfig {
    pub fn init(
        authority: RoleRuntimeAuthority,
    ) -> Result<Arc<RoleRuntimeAuthority>, InternalError> {
        authority
            .component_topology
            .canonical_bytes()
            .map_err(|_error| InternalError::invariant())?;
        ROLE_RUNTIME_AUTHORITY.with(|installed| {
            let mut installed = installed.borrow_mut();
            if installed.is_some() {
                return Err(InternalError::invariant());
            }
            let authority = Arc::new(authority);
            *installed = Some(InstalledRoleRuntimeAuthority {
                authority: authority.clone(),
            });
            Ok(authority)
        })
    }

    #[must_use]
    pub fn try_get() -> Option<Arc<RoleRuntimeAuthority>> {
        ROLE_RUNTIME_AUTHORITY.with(|installed| {
            installed
                .borrow()
                .as_ref()
                .map(|installed| installed.authority.clone())
        })
    }

    #[cfg(test)]
    pub fn reset_for_tests() {
        ROLE_RUNTIME_AUTHORITY.with(|installed| *installed.borrow_mut() = None);
    }
}
