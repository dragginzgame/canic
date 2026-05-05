//! Ingress payload limits registered by generated update endpoints.

use crate::cdk;
use std::sync::Mutex;

pub const DEFAULT_UPDATE_INGRESS_MAX_BYTES: usize = 16 * 1024;

static UPDATE_LIMITS: Mutex<Vec<UpdatePayloadLimit>> = Mutex::new(Vec::new());

///
/// UpdatePayloadLimit
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UpdatePayloadLimit {
    pub method: &'static str,
    pub max_bytes: usize,
}

/// Register one update endpoint payload limit.
pub fn register_update_limit(method: &'static str, max_bytes: usize) {
    UPDATE_LIMITS
        .lock()
        .expect("update payload limit registry poisoned")
        .push(UpdatePayloadLimit { method, max_bytes });
}

/// Return the configured payload limit for one update method.
pub fn update_limit_for(method: &str) -> Result<Option<usize>, DuplicateUpdatePayloadLimit> {
    let limits = UPDATE_LIMITS
        .lock()
        .expect("update payload limit registry poisoned");
    unique_limit_for(&limits, method)
}

/// Inspect the current ingress update and accept it only when within limit.
pub fn inspect_update_message() {
    let method = cdk::api::msg_method_name();
    let payload_len = cdk::api::msg_arg_data().len();
    let Ok(max_bytes) = update_limit_for(&method) else {
        return;
    };
    let max_bytes = max_bytes.unwrap_or(DEFAULT_UPDATE_INGRESS_MAX_BYTES);

    if payload_len <= max_bytes {
        cdk::api::accept_message();
    }
}

///
/// DuplicateUpdatePayloadLimit
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DuplicateUpdatePayloadLimit;

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

#[cfg(test)]
mod tests {
    use super::{UpdatePayloadLimit, unique_limit_for};

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
}
