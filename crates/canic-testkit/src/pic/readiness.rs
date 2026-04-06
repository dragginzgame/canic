use candid::{Principal, decode_one, encode_args};
use canic::{Error, dto::topology::SubnetRegistryResponse, ids::CanisterRole, protocol};
use pocket_ic::PocketIc;

/// Wait until a PocketIC canister reports `canic_ready`.
pub fn wait_until_ready(pic: &PocketIc, canister_id: Principal, tick_limit: usize) {
    let payload = encode_args(()).expect("encode empty args");

    for _ in 0..tick_limit {
        if let Ok(bytes) = pic.query_call(
            canister_id,
            Principal::anonymous(),
            protocol::CANIC_READY,
            payload.clone(),
        ) && let Ok(ready) = decode_one::<bool>(&bytes)
            && ready
        {
            return;
        }
        pic.tick();
    }

    panic!("canister did not report ready in time: {canister_id}");
}

/// Resolve one role principal from root's subnet registry, polling until present.
#[must_use]
pub fn role_pid(
    pic: &PocketIc,
    root_id: Principal,
    role: &'static str,
    tick_limit: usize,
) -> Principal {
    for _ in 0..tick_limit {
        let registry: Result<Result<SubnetRegistryResponse, Error>, Error> = {
            let payload = encode_args(()).expect("encode empty args");
            pic.query_call(
                root_id,
                Principal::anonymous(),
                protocol::CANIC_SUBNET_REGISTRY,
                payload,
            )
            .map_err(|err| {
                Error::internal(format!(
                    "pocket_ic query_call failed (canister={root_id}, method={}): {err}",
                    protocol::CANIC_SUBNET_REGISTRY
                ))
            })
            .and_then(|bytes| {
                decode_one(&bytes).map_err(|err| {
                    Error::internal(format!("decode_one failed for subnet registry: {err}"))
                })
            })
        };

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
