//! Ingress payload limits registered by generated update endpoints.

use crate::{
    cdk,
    dto::{
        error::Error,
        security::{SecurityEvent, SecurityEventReason},
    },
    ops::runtime::security::{SecurityEventInput, SecurityOps},
};
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
    match current_update_rejection(0) {
        Ok(None) => cdk::api::accept_message(),
        Ok(Some(event)) => emit_inspect_security_event(&event),
        Err(DuplicateUpdatePayloadLimit) => emit_duplicate_limit_event(),
    }
}

/// Enforce the current update payload limit during replicated execution.
pub fn enforce_update_message() -> Result<(), Error> {
    match current_update_rejection(cdk::api::time() / 1_000_000_000) {
        Ok(None) => Ok(()),
        Ok(Some(event)) => {
            if let Err(err) = SecurityOps::record(SecurityEventInput {
                caller: event.caller,
                endpoint: event.endpoint.clone(),
                request_bytes: event.request_bytes,
                max_bytes: event.max_bytes,
                created_at: event.created_at,
                reason: event.reason,
            }) {
                cdk::println!("security event stable write failed: {err}");
            }

            Err(Error::exhausted(format!(
                "ingress payload for '{}' exceeded configured limit: {} > {} bytes",
                event.endpoint, event.request_bytes, event.max_bytes
            )))
        }
        Err(DuplicateUpdatePayloadLimit) => Err(Error::internal(
            "duplicate update payload limit metadata; rejecting request",
        )),
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

// Return a security event when the current update exceeds its configured limit.
fn current_update_rejection(
    created_at: u64,
) -> Result<Option<SecurityEvent>, DuplicateUpdatePayloadLimit> {
    let endpoint = cdk::api::msg_method_name();
    let request_bytes = cdk::api::msg_arg_data().len();
    let max_bytes = update_limit_for(&endpoint)?.unwrap_or(DEFAULT_UPDATE_INGRESS_MAX_BYTES);

    if request_bytes <= max_bytes {
        return Ok(None);
    }

    Ok(Some(SecurityEvent {
        id: 0,
        created_at,
        caller: cdk::api::msg_caller(),
        endpoint,
        request_bytes: usize_to_u64(request_bytes),
        max_bytes: usize_to_u64(max_bytes),
        reason: SecurityEventReason::IngressPayloadLimitExceeded,
    }))
}

// Emit an operator-visible line for pre-consensus rejects.
fn emit_inspect_security_event(event: &SecurityEvent) {
    cdk::println!(
        "security ingress payload reject caller={} endpoint={} request_bytes={} max_bytes={}",
        event.caller,
        event.endpoint,
        event.request_bytes,
        event.max_bytes
    );
}

// Emit an operator-visible line when generated endpoint metadata is invalid.
fn emit_duplicate_limit_event() {
    cdk::println!("security ingress payload reject duplicate update payload limit metadata");
}

// Convert platform sizes into stable DTO integers without truncating.
fn usize_to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
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
