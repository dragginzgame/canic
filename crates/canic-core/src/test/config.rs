// Category A - Internal runtime-configured tests (ConfigTestBuilder when needed).

use crate::{
    cdk::types::Cycles,
    config::schema::{CanisterConfig, CanisterKind, RandomnessConfig},
    config::{Config, ConfigModel},
    ids::{CanisterRole, SubnetRole},
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
    pub fn with_app_directory(mut self, role: impl Into<CanisterRole>) -> Self {
        self.model.app_directory.insert(role.into());
        self
    }

    #[must_use]
    pub fn with_prime_canister_kind(
        self,
        role: impl Into<CanisterRole>,
        kind: CanisterKind,
    ) -> Self {
        self.with_prime_canister(role, Self::canister_config(kind))
    }

    #[must_use]
    pub fn with_prime_canister(
        self,
        role: impl Into<CanisterRole>,
        config: CanisterConfig,
    ) -> Self {
        self.with_subnet_canister(SubnetRole::PRIME, role, config)
    }

    #[must_use]
    pub fn with_subnet_canister(
        mut self,
        subnet: impl Into<SubnetRole>,
        role: impl Into<CanisterRole>,
        config: CanisterConfig,
    ) -> Self {
        let subnet = subnet.into();
        let role = role.into();
        let entry = self.model.subnets.entry(subnet).or_default();

        entry.canisters.insert(role, config);

        self
    }

    #[must_use]
    pub fn with_prime_auto_create(self, role: impl Into<CanisterRole>) -> Self {
        self.with_subnet_auto_create(SubnetRole::PRIME, role)
    }

    #[must_use]
    pub fn with_subnet_auto_create(
        mut self,
        subnet: impl Into<SubnetRole>,
        role: impl Into<CanisterRole>,
    ) -> Self {
        let subnet = subnet.into();
        let role = role.into();
        let entry = self.model.subnets.entry(subnet).or_default();

        entry.auto_create.insert(role);

        self
    }

    #[must_use]
    pub fn build(self) -> ConfigModel {
        self.model
    }

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
            randomness: RandomnessConfig::default(),
            scaling: None,
            sharding: None,
        }
    }
}
