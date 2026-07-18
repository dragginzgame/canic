//! Module: workflow::rpc::capability::replay
//!
//! Responsibility: project capability metadata into replay request metadata.
//! Does not own: replay storage, request dispatch, or capability proof validation.
//! Boundary: validates metadata freshness and preserves durable replay identifiers.

use crate::{
    dto::{capability::CapabilityRequestMetadata, error::Error, rpc::RootRequestMetadata},
    workflow::rpc::{
        capability::MAX_CAPABILITY_CLOCK_SKEW_NS, request::handler::capability::RootCapability,
    },
};

pub(super) const fn with_root_request_metadata(
    request: RootCapability,
    metadata: RootRequestMetadata,
) -> RootCapability {
    request.with_metadata(metadata)
}

pub(super) fn project_replay_metadata(
    metadata: CapabilityRequestMetadata,
    now_ns: u64,
) -> Result<RootRequestMetadata, Error> {
    if metadata.ttl_ns == 0 {
        return Err(Error::invalid(
            "capability metadata ttl_ns must be greater than zero",
        ));
    }

    let max_future_ns = now_ns
        .checked_add(MAX_CAPABILITY_CLOCK_SKEW_NS)
        .ok_or_else(|| Error::invalid("capability metadata clock skew overflow"))?;
    if metadata.issued_at_ns > max_future_ns {
        return Err(Error::invalid(
            "capability metadata issued_at_ns is too far in the future",
        ));
    }

    let expires_at_ns = metadata
        .issued_at_ns
        .checked_add(metadata.ttl_ns)
        .ok_or_else(|| Error::invalid("capability metadata expiry overflow"))?;
    if now_ns >= expires_at_ns {
        return Err(Error::conflict("capability metadata has expired"));
    }

    Ok(RootRequestMetadata {
        request_id: metadata.request_id,
        ttl_ns: metadata.ttl_ns,
    })
}
