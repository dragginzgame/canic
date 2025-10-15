use canic::types::CanisterType;

///
/// CANISTER TYPES
///
/// Canonical `CanisterType` constants. Downstream Canic implementations can use
/// this pattern as a reference when wiring their own type catalogs.
///

pub const APP: CanisterType = CanisterType::new("app");
pub const AUTH: CanisterType = CanisterType::new("auth");
pub const BLANK: CanisterType = CanisterType::new("blank");
pub const SCALE_HUB: CanisterType = CanisterType::new("scale_hub");
pub const SCALE: CanisterType = CanisterType::new("scale");
pub const SHARD_HUB: CanisterType = CanisterType::new("shard_hub");
pub const SHARD: CanisterType = CanisterType::new("shard");
