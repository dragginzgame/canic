// Category A - Internal runtime-configured tests (ConfigTestBuilder when needed).

use crate::{
    cdk::types::Cycles,
    config::schema::{
        CanisterAuthConfig, CanisterConfig, CanisterKind, CyclesFundingPolicyConfig,
        DiagnosticsCanisterConfig, MetricsCanisterConfig, RoleDeclaration, RoleDeclarationKind,
        StandardsCanisterConfig,
    },
    config::{Config, ConfigModel},
    ids::{CanisterRole, TreeSpecId},
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

    /// Add one canister configuration to the canonical default Tree Spec.
    ///
    /// # Panics
    ///
    /// Panics only if Canic's canonical `"default"` Tree Spec identifier stops
    /// satisfying `TreeSpecId` admission.
    #[must_use]
    pub fn with_default_canister(
        self,
        role: impl Into<CanisterRole>,
        config: CanisterConfig,
    ) -> Self {
        self.with_tree_spec_canister(
            "default".parse().expect("default Tree Spec ID"),
            role,
            config,
        )
    }

    #[must_use]
    pub fn with_tree_spec_canister(
        mut self,
        tree_spec: TreeSpecId,
        role: impl Into<CanisterRole>,
        config: CanisterConfig,
    ) -> Self {
        let role = role.into();
        let declaration_kind = match config.kind {
            CanisterKind::Root => RoleDeclarationKind::Root,
            _ => RoleDeclarationKind::Canister,
        };
        let entry = self.model.tree_specs.entry(tree_spec).or_default();

        self.model.roles.insert(
            role.clone(),
            RoleDeclaration {
                kind: declaration_kind,
                package: role.as_ref().to_string(),
            },
        );
        entry.canisters.insert(role, config);

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
}
