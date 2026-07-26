// Category A - Internal runtime-configured tests (ConfigTestBuilder when needed).

use crate::{
    cdk::types::Cycles,
    config::schema::{
        CanisterAuthConfig, CanisterConfig, CanisterKind, ComponentChildConfig, ComponentChildKind,
        ComponentLimitsConfig, ComponentSpecConfig, CyclesFundingPolicyConfig,
        DiagnosticsCanisterConfig, MetricsCanisterConfig, RoleDeclaration, RoleDeclarationKind,
        StandardsCanisterConfig,
    },
    config::{Config, ConfigModel},
    ids::{CanisterRole, ComponentSpecId},
};
use std::sync::Arc;

///
/// ConfigTestBuilder
///

#[derive(Default)]
pub struct ConfigTestBuilder {
    model: ConfigModel,
}

impl ConfigTestBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            model: ConfigModel::test_default(),
        }
    }

    #[must_use]
    pub fn with_default_canister_kind(
        self,
        role: impl Into<CanisterRole>,
        kind: CanisterKind,
    ) -> Self {
        self.with_default_canister(role, Self::canister_config(kind))
    }

    /// Add one canister configuration to the canonical default Component Spec.
    ///
    /// # Panics
    ///
    /// Panics only if Canic's canonical `"default"` Component Spec identifier stops
    /// satisfying `ComponentSpecId` admission.
    #[must_use]
    pub fn with_default_canister(
        self,
        role: impl Into<CanisterRole>,
        config: CanisterConfig,
    ) -> Self {
        self.with_component_spec_canister(
            "default".parse().expect("default Component Spec ID"),
            role,
            config,
        )
    }

    #[must_use]
    pub fn with_component_spec_canister(
        mut self,
        component_spec: ComponentSpecId,
        role: impl Into<CanisterRole>,
        config: CanisterConfig,
    ) -> Self {
        let role = role.into();
        let declaration_kind = match config.kind {
            CanisterKind::Root => RoleDeclarationKind::Root,
            _ => RoleDeclarationKind::Canister,
        };
        self.model.roles.insert(
            role.clone(),
            RoleDeclaration {
                kind: declaration_kind,
                package: role.as_ref().to_string(),
            },
        );

        if config.kind == CanisterKind::Root {
            return self;
        }

        let entry = self
            .model
            .component_specs
            .entry(component_spec)
            .or_insert_with(|| Self::component_spec_config(role.clone()));
        if config.kind == CanisterKind::Service {
            entry.component_role = role;
            entry.initial_cycles = config.initial_cycles;
            entry.topup = config.topup;
            entry.cycles_funding = config.cycles_funding;
            entry.scaling = config.scaling;
            entry.sharding = config.sharding;
            entry.binding = config.binding;
            entry.auth = config.auth;
            entry.standards = config.standards;
            entry.diagnostics = config.diagnostics;
            entry.metrics = config.metrics;
            return self;
        }

        let kind = match config.kind {
            CanisterKind::Singleton => ComponentChildKind::Singleton,
            CanisterKind::Replica => ComponentChildKind::Replica,
            CanisterKind::Shard => ComponentChildKind::Shard,
            CanisterKind::Instance => ComponentChildKind::Instance,
            CanisterKind::Root | CanisterKind::Service => unreachable!("handled above"),
        };
        entry.children.insert(
            role,
            ComponentChildConfig {
                kind,
                maximum_instances: if kind == ComponentChildKind::Singleton {
                    1
                } else {
                    4_096
                },
                initial_cycles: config.initial_cycles,
                topup: config.topup,
                cycles_funding: config.cycles_funding,
                auth: config.auth,
                standards: config.standards,
                diagnostics: config.diagnostics,
                metrics: config.metrics,
            },
        );

        self
    }

    #[must_use]
    pub fn build(self) -> ConfigModel {
        self.model
    }

    /// Install this builder's model as the process-local test config.
    ///
    /// # Panics
    ///
    /// Panics if the constructed test configuration fails runtime
    /// initialization.
    #[must_use]
    pub fn install(self) -> Arc<ConfigModel> {
        Config::reset_for_tests();
        Config::init_from_model_for_tests(self.model).expect("init test config")
    }

    #[must_use]
    pub fn canister_config(kind: CanisterKind) -> CanisterConfig {
        CanisterConfig {
            kind,
            initial_cycles: Cycles::new(0),
            topup: None,
            icp_refill: None,
            cycles_funding: CyclesFundingPolicyConfig::default(),
            scaling: None,
            sharding: None,
            binding: None,
            auth: CanisterAuthConfig::default(),
            standards: StandardsCanisterConfig::default(),
            diagnostics: DiagnosticsCanisterConfig::default(),
            metrics: MetricsCanisterConfig::default(),
        }
    }

    fn component_spec_config(component_role: CanisterRole) -> ComponentSpecConfig {
        ComponentSpecConfig {
            component_role,
            maximum_instances: 1,
            limits: ComponentLimitsConfig::default(),
            initial_cycles: Cycles::new(0),
            topup: None,
            cycles_funding: CyclesFundingPolicyConfig::default(),
            scaling: None,
            sharding: None,
            binding: None,
            auth: CanisterAuthConfig::default(),
            standards: StandardsCanisterConfig::default(),
            diagnostics: DiagnosticsCanisterConfig::default(),
            metrics: MetricsCanisterConfig::default(),
            children: Default::default(),
        }
    }
}
