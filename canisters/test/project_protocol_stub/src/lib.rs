//! Shared protocol descriptors for the project hub/instance test canisters.

use canic::api::canister::CanisterRole;

pub const PROJECT_HUB: CanisterRole = CanisterRole::new("project_hub");
pub const PROJECT_INSTANCE: CanisterRole = CanisterRole::new("project_instance");
