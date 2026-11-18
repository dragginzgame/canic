use super::{ASSIGN, Key, LIMITS, LOOKUP, Placement};
use ic_platform_runtime::call::{Call, Outcome, Wait};
use ic_platform_types::Principal;

/// Request the original assignment. After transport uncertainty, look up the same key.
/// # Errors
/// An unverified or missing reply cannot establish an assignment.
pub async fn assign(root: Principal, key: Key) -> Result<Placement, &'static str> {
    call(root, "assign_shard", ASSIGN, key).await
}

/// Observe the retained claim without starting any creation.
/// # Errors
/// Reports transport or framing failures separately from an absent key.
pub async fn lookup(root: Principal, key: Key) -> Result<Placement, &'static str> {
    call(root, "lookup_shard", LOOKUP, key).await
}

async fn call(
    root: Principal,
    name: &str,
    method: u64,
    key: Key,
) -> Result<Placement, &'static str> {
    let frame = |direction| ic_wire::Encoding::Frame { method, direction };
    let bytes = ic_wire::encode(&key, frame(ic_wire::Direction::Request), LIMITS)
        .map_err(|_| "placement request")?;
    let result = Call::new(root, name, Wait::Bounded { seconds: 10 })
        .owned_args(bytes)
        .execute()
        .await
        .map_err(|_| "placement transport")?;
    let Outcome::Reply(reply) = result.into_parts().0 else {
        return Err("placement reply unavailable");
    };
    ic_wire::decode(&reply, frame(ic_wire::Direction::Response), LIMITS)
        .map_err(|_| "placement reply")
}
