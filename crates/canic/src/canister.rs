//! Canonical `CanisterType` constants. Downstream ICU implementations can use
//! this pattern as a reference when wiring their own type catalogs.

use crate::types::CanisterType;

pub const BLANK: CanisterType = CanisterType::new("blank");
pub const DELEGATION: CanisterType = CanisterType::new("delegation");
pub const SCALE_HUB: CanisterType = CanisterType::new("scale_hub");
pub const SCALE: CanisterType = CanisterType::new("scale");
pub const SHARD_HUB: CanisterType = CanisterType::new("shard_hub");
pub const SHARD: CanisterType = CanisterType::new("shard");
