use candid::Principal;
use canic::{Error, dto::topology::SubnetRegistryResponse, ids::CanisterRole, protocol};

use super::Pic;

/// Wait until a PocketIC canister reports `canic_ready`.
pub fn wait_until_ready(pic: &Pic, canister_id: Principal, tick_limit: usize) {
    for _ in 0..tick_limit {
        if let Ok(ready) = pic.query_call_as::<bool, _>(
            canister_id,
            Principal::anonymous(),
            protocol::CANIC_READY,
            (),
        ) && ready
        {
            return;
        }
        pic.tick();
    }

    panic!("canister did not report ready in time: {canister_id}");
}

/// Resolve one role principal from root's subnet registry, polling until present.
#[must_use]
pub fn role_pid(pic: &Pic, root_id: Principal, role: &'static str, tick_limit: usize) -> Principal {
    for _ in 0..tick_limit {
        let registry: Result<Result<SubnetRegistryResponse, Error>, Error> = pic.query_call_as(
            root_id,
            Principal::anonymous(),
            protocol::CANIC_SUBNET_REGISTRY,
            (),
        );

        if let Ok(Ok(registry)) = registry
            && let Some(pid) = registry
                .0
                .into_iter()
                .find(|entry| entry.role == CanisterRole::new(role))
                .map(|entry| entry.pid)
        {
            return pid;
        }

        pic.tick();
    }

    panic!("{role} canister must be registered");
}
