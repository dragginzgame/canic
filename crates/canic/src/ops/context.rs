use crate::{
    Error,
    config::{
        Config,
        model::{CanisterConfig, SubnetConfig},
    },
    memory::Env,
};

///
/// Context Helpers
///
/// These functions resolve configuration records that are specific to the
/// *currently executing canister* or *subnet*.
///
/// They combine runtime environment data (from [`Env`]) with the
/// static configuration model (from [`Config`]) to provide an
/// "effective view" of configuration for the active execution context.
///

/// Fetch the effective config for the current canister.
pub fn cfg_current_canister() -> Result<CanisterConfig, Error> {
    let subnet_cfg = cfg_current_subnet()?;

    // canister cfg
    let canister_type = Env::try_get_canister_type()?;
    let canister_cfg = subnet_cfg.try_get_canister(&canister_type)?;

    Ok(canister_cfg)
}

/// Fetch the configuration record for the currently executing subnet.
pub fn cfg_current_subnet() -> Result<SubnetConfig, Error> {
    let subnet_type = Env::try_get_subnet_type()?;
    let subnet_cfg = Config::get().try_get_subnet(&subnet_type)?;

    Ok(subnet_cfg)
}
