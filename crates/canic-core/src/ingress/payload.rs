//! Module: ingress::payload
//!
//! Responsibility: update ingress payload limits registered by endpoint macros.
//! Does not own: endpoint dispatch, authorization, or payload decoding.
//! Boundary: stores method limit metadata consumed during ingress inspection.

use std::sync::Mutex;

pub const DEFAULT_UPDATE_INGRESS_MAX_BYTES: usize = 16 * 1024;

static UPDATE_LIMITS: Mutex<Vec<UpdatePayloadLimit>> = Mutex::new(Vec::new());

// Payload byte limit registered for one update method.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct UpdatePayloadLimit {
    method: &'static str,
    max_bytes: usize,
}

/// Register one update endpoint payload limit.
///
/// # Panics
///
/// Panics if the process-local update payload limit registry mutex is poisoned.
pub fn register_update_limit(method: &'static str, max_bytes: usize) {
    UPDATE_LIMITS
        .lock()
        .expect("update payload limit registry poisoned")
        .push(UpdatePayloadLimit { method, max_bytes });
}

/// Return the configured payload limit for one update method.
///
/// # Panics
///
/// Panics if the process-local update payload limit registry mutex is poisoned.
fn update_limit_for(method: &str) -> Result<Option<usize>, DuplicateUpdatePayloadLimit> {
    let limits = UPDATE_LIMITS
        .lock()
        .expect("update payload limit registry poisoned");
    unique_limit_for(&limits, method)
}

/// Inspect the current ingress update and accept it only when within limit.
///
/// # Panics
///
/// Panics if reading the configured payload limit finds a poisoned registry
/// mutex.
pub fn inspect_update_message() {
    let method = current_method_name();
    let payload_len = current_payload_bytes().len();
    let Ok(max_bytes) = update_limit_for(&method) else {
        return;
    };
    let max_bytes = max_bytes.unwrap_or(DEFAULT_UPDATE_INGRESS_MAX_BYTES);

    if payload_len <= max_bytes {
        accept_current_message();
    }
}

/// Return the method selected by the current ingress message.
#[must_use]
pub fn current_method_name() -> String {
    ic_cdk::api::msg_method_name()
}

/// Return the current ingress payload for bounded role-specific decoding.
#[must_use]
pub fn current_payload_bytes() -> Vec<u8> {
    ic_cdk::api::msg_arg_data()
}

/// Accept the current ingress after its role-specific bound has passed.
pub fn accept_current_message() {
    ic_cdk::api::accept_message();
}

/// Return whether one encoded payload fits its selected variant's exact bound.
#[must_use]
pub const fn payload_within_limit(payload_len: usize, max_bytes: usize) -> bool {
    payload_len <= max_bytes
}

// Error returned when more than one limit is registered for the same method.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DuplicateUpdatePayloadLimit;

// Return one unique limit for a method, treating duplicate metadata as invalid.
fn unique_limit_for(
    limits: &[UpdatePayloadLimit],
    method: &str,
) -> Result<Option<usize>, DuplicateUpdatePayloadLimit> {
    let mut found = None;

    for limit in limits.iter().filter(|limit| limit.method == method) {
        if found.replace(limit.max_bytes).is_some() {
            return Err(DuplicateUpdatePayloadLimit);
        }
    }

    Ok(found)
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::{UpdatePayloadLimit, payload_within_limit, unique_limit_for};

    #[test]
    fn unique_limit_returns_registered_limit() {
        let limits = [UpdatePayloadLimit {
            method: "save",
            max_bytes: 1024,
        }];

        assert_eq!(unique_limit_for(&limits, "save"), Ok(Some(1024)));
    }

    #[test]
    fn unique_limit_rejects_duplicate_method_metadata() {
        let limits = [
            UpdatePayloadLimit {
                method: "save",
                max_bytes: 1024,
            },
            UpdatePayloadLimit {
                method: "save",
                max_bytes: 2048,
            },
        ];

        assert_eq!(
            unique_limit_for(&limits, "save"),
            Err(super::DuplicateUpdatePayloadLimit)
        );
    }

    #[test]
    fn variant_payload_limit_accepts_boundary_and_rejects_first_excess() {
        assert!(payload_within_limit(16_384, 16_384));
        assert!(!payload_within_limit(16_385, 16_384));
    }
}
