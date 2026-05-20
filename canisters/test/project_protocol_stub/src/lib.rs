//! Shared protocol descriptors for the project hub/instance test canisters.

use canic::api::canister::CanisterRole;

pub const PROJECT_HUB: CanisterRole = CanisterRole::new("project_hub");
pub const PROJECT_INSTANCE: CanisterRole = CanisterRole::new("project_instance");

canic::canic_protected_endpoint! {
    pub fn project_instance_record_visit_endpoint =
        "project_instance_record_visit",
        role = CanisterRole::new("project_hub");
}
