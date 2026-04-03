use canic_core::ids::CanisterRole;

///
/// CANISTER ROLES
///
/// Canonical `CanisterRole` constants. Downstream Canic implementations can use
/// this pattern as a reference when wiring their own type catalogs.
///

pub const ROOT: CanisterRole = CanisterRole::ROOT;
pub const WASM_STORE: CanisterRole = CanisterRole::WASM_STORE;

pub const APP: CanisterRole = CanisterRole::new("app");
pub const AUTH: CanisterRole = CanisterRole::new("auth");
pub const MINIMAL: CanisterRole = CanisterRole::new("minimal");
pub const SCALE_HUB: CanisterRole = CanisterRole::new("scale_hub");
pub const SCALE: CanisterRole = CanisterRole::new("scale");
pub const TEST: CanisterRole = CanisterRole::new("test");
pub const USER_HUB: CanisterRole = CanisterRole::new("user_hub");
pub const USER_SHARD: CanisterRole = CanisterRole::new("user_shard");
