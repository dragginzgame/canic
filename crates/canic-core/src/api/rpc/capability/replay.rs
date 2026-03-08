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
    now_secs: u64,
) -> Result<RootRequestMetadata, Error> {
    if metadata.ttl_seconds == 0 {
        return Err(Error::invalid(
            "capability metadata ttl_seconds must be greater than zero",
        ));
    }

    if metadata.issued_at > now_secs.saturating_add(super::MAX_CAPABILITY_CLOCK_SKEW_SECONDS) {
        return Err(Error::invalid(
            "capability metadata issued_at is too far in the future",
        ));
    }

    let expires_at = metadata
        .issued_at
        .checked_add(u64::from(metadata.ttl_seconds))
        .ok_or_else(|| Error::invalid("capability metadata expiry overflow"))?;
    if now_secs > expires_at {
        return Err(Error::conflict("capability metadata has expired"));
    }

    Ok(RootRequestMetadata {
        request_id: replay_request_id(metadata.request_id, metadata.nonce),
        ttl_seconds: u64::from(metadata.ttl_seconds),
    })
}

pub(super) fn replay_request_id(request_id: [u8; 16], nonce: [u8; 16]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(super::REPLAY_REQUEST_ID_DOMAIN_V1);
    hasher.update(request_id);
    hasher.update(nonce);
    hasher.finalize().into()
}
