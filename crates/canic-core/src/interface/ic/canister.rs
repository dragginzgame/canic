use crate::{
    Error,
    cdk::mgmt::{self, CanisterSettings, CreateCanisterArgs},
    interface::prelude::*,
};

///
/// create_canister
/// Provision a new canister with controllers and an optional cycle balance.
///
pub async fn create_canister(
    controllers: Vec<Principal>,
    cycles: Cycles,
) -> Result<Principal, Error> {
    let settings = Some(CanisterSettings {
        controllers: Some(controllers),
        ..Default::default()
    });
    let cc_args = CreateCanisterArgs { settings };

    // create
    let canister_pid = mgmt::create_canister_with_extra_cycles(&cc_args, cycles.to_u128())
        .await?
        .canister_id;

    Ok(canister_pid)
}
