use canic::ids::CanisterRole;

///
/// CANISTER ROLES
///
/// Canonical role constants used by Canic's internal test harnesses.
///

pub const APP: CanisterRole = CanisterRole::new("app");
pub const MINIMAL: CanisterRole = CanisterRole::new("minimal");
pub const SCALE_HUB: CanisterRole = CanisterRole::new("scale_hub");
pub const SCALE_REPLICA: CanisterRole = CanisterRole::new("scale_replica");
pub const TEST: CanisterRole = CanisterRole::new("test");
pub const WASM_STORE: CanisterRole = CanisterRole::WASM_STORE;
pub const USER_HUB: CanisterRole = CanisterRole::new("user_hub");
pub const USER_SHARD: CanisterRole = CanisterRole::new("user_shard");
