use canic_core::ids::CanisterRole;

///
/// CANISTER ROLES
///
/// Canonical `CanisterRole` constants. Downstream Canic implementations can use
/// this pattern as a reference when wiring their own type catalogs.
///

pub const APP: CanisterRole = CanisterRole::new("app");
pub const AUTH: CanisterRole = CanisterRole::new("auth");
pub const AUTH_HUB: CanisterRole = CanisterRole::new("auth_hub");
pub const AUTH_SHARD: CanisterRole = CanisterRole::new("auth_shard");
pub const BLANK: CanisterRole = CanisterRole::new("blank");
pub const SCALE_HUB: CanisterRole = CanisterRole::new("scale_hub");
pub const SCALE: CanisterRole = CanisterRole::new("scale");
pub const SHARD_HUB: CanisterRole = CanisterRole::new("shard_hub");
pub const SHARD: CanisterRole = CanisterRole::new("shard");
pub const TEST: CanisterRole = CanisterRole::new("test");
