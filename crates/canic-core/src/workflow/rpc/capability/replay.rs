use crate::dto::{
    capability::CapabilityRequestMetadata,
    error::Error,
    rpc::{Request, RootRequestMetadata},
};
use sha2::{Digest, Sha256};

pub(super) const fn with_root_request_metadata(
    request: Request,
    metadata: RootRequestMetadata,
) -> Request {
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
        .checked_add(super::MAX_CAPABILITY_CLOCK_SKEW_NS)
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
        request_id: replay_request_id(metadata.request_id, metadata.nonce),
        ttl_ns: metadata.ttl_ns,
    })
}

pub(super) fn replay_request_id(request_id: [u8; 16], nonce: [u8; 16]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(super::REPLAY_REQUEST_ID_DOMAIN_V1);
    hasher.update(request_id);
    hasher.update(nonce);
    hasher.finalize().into()
}
