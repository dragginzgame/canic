pub mod format {
    pub use crate::format::{byte_size, cycles_tc, truncate};
}

pub mod icp_refill {
    pub use crate::domain::icp_refill::icp_refill_outcome_is_resumable;
}

/// Canonical immutable root-funding policy identities shared with host planning.
pub mod fleet_funding_policy {
    pub use crate::model::fleet_funding_policy::{
        FleetFundingPolicyValidationError, validate_coordinator_root_funding_policy,
        validate_fleet_root_funding_capacity, validate_fleet_subnet_root_funding_authority,
    };
    pub use crate::ops::fleet_funding_policy::{
        coordinator_root_funding_policy_hash, fleet_root_funding_operation_id,
        fleet_subnet_root_funding_policy_hash,
    };
}

/// Return whether a name uses canonical lowercase ASCII snake_case.
#[must_use]
pub const fn is_ascii_snake_case(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty() || !bytes[0].is_ascii_lowercase() {
        return false;
    }

    let mut index = 1;
    let mut previous_was_underscore = false;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte.is_ascii_lowercase() || byte.is_ascii_digit() {
            previous_was_underscore = false;
        } else if byte == b'_' && !previous_was_underscore {
            previous_was_underscore = true;
        } else {
            return false;
        }
        index += 1;
    }

    !previous_was_underscore
}
