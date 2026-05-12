//! Lifecycle adapters.
//!
//! This module is public solely so it can be referenced by
//! macro expansions in downstream crates. It is not intended
//! for direct use.
//!
//! It must remain synchronous and minimal.

pub mod init;
pub mod upgrade;

use crate::{config::schema::ConfigModel, ops::ic::IcOps};
use std::fmt;

///
/// LifecyclePhase
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifecyclePhase {
    Init,
    PostUpgrade,
}

impl fmt::Display for LifecyclePhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Init => f.write_str("init"),
            Self::PostUpgrade => f.write_str("post_upgrade"),
        }
    }
}

pub fn lifecycle_trap(phase: LifecyclePhase, err: impl fmt::Display) -> ! {
    crate::cdk::api::trap(format!("{phase}: {err}"))
}

fn config_with_current_root_controller(config: ConfigModel) -> ConfigModel {
    config_with_root_controller(config, IcOps::msg_caller())
}

fn config_with_root_controller(
    mut config: ConfigModel,
    controller: crate::cdk::candid::Principal,
) -> ConfigModel {
    if !config.controllers.contains(&controller) {
        config.controllers.push(controller);
    }
    config
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cdk::candid::Principal;

    fn p(byte: u8) -> Principal {
        Principal::from_slice(&[byte])
    }

    #[test]
    fn config_with_root_controller_appends_installing_controller() {
        let config = config_with_root_controller(ConfigModel::default(), p(7));

        assert_eq!(config.controllers, vec![p(7)]);
    }

    #[test]
    fn config_with_root_controller_deduplicates_existing_controller() {
        let mut config = ConfigModel {
            controllers: vec![p(7), p(8)],
            ..Default::default()
        };

        config = config_with_root_controller(config, p(7));

        assert_eq!(config.controllers, vec![p(7), p(8)]);
    }
}
