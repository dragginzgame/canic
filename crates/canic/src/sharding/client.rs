use super::{Key, Placement};
use crate::method::{self, CallError, Contract};
use ic_platform_types::Principal;

/// Reserve or recover a shared shard for one placement key.
pub struct AssignShard;

impl Contract for AssignShard {
    const NAME: &'static str = "assign_shard";
    const ID: u64 = super::ASSIGN;
    const REQUEST: ic_wire::Limits = super::LIMITS;
    type Input = (String, Key);
    type Output = Placement;
}

/// Observe an existing shared-shard assignment.
pub struct LookupShard;

impl Contract for LookupShard {
    const NAME: &'static str = "lookup_shard";
    const ID: u64 = super::LOOKUP;
    const REQUEST: ic_wire::Limits = super::LIMITS;
    type Input = (String, Key);
    type Output = Placement;
}

/// Reserve or recover a dedicated instance for one placement key.
pub struct AssignInstance;

impl Contract for AssignInstance {
    const NAME: &'static str = "assign_instance";
    const ID: u64 = super::ASSIGN_INSTANCE;
    const REQUEST: ic_wire::Limits = super::LIMITS;
    type Input = Key;
    type Output = Placement;
}

/// Observe an existing dedicated-instance assignment.
pub struct LookupInstance;

impl Contract for LookupInstance {
    const NAME: &'static str = "lookup_instance";
    const ID: u64 = super::LOOKUP_INSTANCE;
    const REQUEST: ic_wire::Limits = super::LIMITS;
    type Input = Key;
    type Output = Placement;
}

/// Request the original assignment. After transport uncertainty, look up the same key.
/// # Errors
/// An unverified or missing reply cannot establish an assignment.
pub async fn assign(root: Principal, pool: &str, key: Key) -> Result<Placement, &'static str> {
    call::<AssignShard>(root, (pool.to_owned(), key)).await
}

/// Observe the retained claim without starting any creation.
/// # Errors
/// Reports transport or framing failures separately from an absent key.
pub async fn lookup(root: Principal, pool: &str, key: Key) -> Result<Placement, &'static str> {
    call::<LookupShard>(root, (pool.to_owned(), key)).await
}

/// Request the original dedicated child for a stable application key.
/// # Errors
/// [`CallError::Request`] and [`CallError::Dispatch`] mean this attempt was never
/// submitted. For a rejection, unknown outcome or unreadable reply, observe the
/// original key with [`lookup_instance`]. Retain that key on every retry.
pub async fn assign_instance(root: Principal, key: Key) -> Result<Placement, CallError> {
    method::call::<AssignInstance>(root, 10, &key).await
}

/// Look up the original project claim without allocating.
/// # Errors
/// Returns an error for a missing or invalid reply.
pub async fn lookup_instance(root: Principal, key: Key) -> Result<Placement, CallError> {
    method::call::<LookupInstance>(root, 10, &key).await
}

async fn call<M: Contract<Output = Placement>>(
    root: Principal,
    key: M::Input,
) -> Result<Placement, &'static str> {
    method::call::<M>(root, 10, &key)
        .await
        .map_err(|error| match error {
            CallError::Request(_) => "placement request",
            CallError::Dispatch(_) => "placement transport",
            CallError::Rejected(_) | CallError::Unknown(_) => "placement reply unavailable",
            CallError::Reply(_) => "placement reply",
        })
}

/// Request or recover one whole shard by its ordinal. The hub owns its account routes.
pub struct GrowShard;
impl Contract for GrowShard {
    const NAME: &'static str = "grow_shard";
    const ID: u64 = ic_wire::method_id(Self::NAME);
    const REQUEST: ic_wire::Limits = super::LIMITS;
    type Input = (String, u8);
    type Output = (Placement, u32);
}

/// Recover the same ordinal after reply loss. The second value is the approved keys per shard.
/// # Errors
/// A transport or framing error leaves the original request unresolved.
pub async fn grow_shard(
    root: Principal,
    pool: &str,
    ordinal: u8,
) -> Result<(Placement, u32), CallError> {
    method::call::<GrowShard>(root, 10, &(pool.to_owned(), ordinal)).await
}

/// Recover or request one singleton belonging to the calling registry instance.
pub struct RequestSingleton;
impl Contract for RequestSingleton {
    const NAME: &'static str = "request_singleton";
    const ID: u64 = ic_wire::method_id(Self::NAME);
    const REQUEST: ic_wire::Limits = super::LIMITS;
    type Input = String;
    type Output = Placement;
}
