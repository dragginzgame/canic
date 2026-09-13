//! Pure frontend origin, admission and selection checks.
//!
//! No transport, serialization or state access belongs here.

use crate::frontend::{FrontendError, model::FrontendEnvironmentInput};
use candid::Principal;
use canic_core::ids::CanonicalNetworkId;
use std::collections::BTreeSet;
use url::Url;

/// II's externally specified maximum alternative-origin list length.
pub const MAX_ALTERNATIVE_ORIGINS: usize = 100;
/// Host collection budget, separate from any runtime role or database capacity.
pub const MAX_EXPORTED_ROLES: usize = 128;

/// Accept only canonical bare origins, with explicitly local HTTP exceptions.
pub fn validate_origin(origin: &str, local: bool) -> Result<(), FrontendError> {
    let parsed = Url::parse(origin).map_err(|_| FrontendError::Origin(origin.to_string()))?;
    let canonical = parsed.origin().ascii_serialization() == origin;
    let bare = parsed.username().is_empty()
        && parsed.password().is_none()
        && parsed.query().is_none()
        && parsed.fragment().is_none()
        && parsed.path() == "/";
    let loopback = parsed.host_str().is_some_and(|host| {
        host == "localhost"
            || host.ends_with(".localhost")
            || host == "[::1]"
            || host
                .parse::<std::net::Ipv4Addr>()
                .is_ok_and(|ip| ip.is_loopback())
    });
    let scheme = parsed.scheme() == "https" || (local && parsed.scheme() == "http" && loopback);
    if !(canonical && bare && scheme) {
        return Err(FrontendError::Origin(origin.to_string()));
    }
    Ok(())
}

fn concrete_principal(principal: Principal) -> Result<(), FrontendError> {
    if principal == Principal::anonymous() || principal == Principal::management_canister() {
        return Err(FrontendError::Principal);
    }
    Ok(())
}

/// Validate browser input against selected network and the reviewed admission namespace.
pub fn validate_input(
    input: &FrontendEnvironmentInput,
    environment: &str,
    network: CanonicalNetworkId,
    admission_nonempty: bool,
    admission_origin: Option<&str>,
) -> Result<(), FrontendError> {
    if input.schema_version != 1 {
        return Err(FrontendError::Schema);
    }
    if input.environment != environment || input.canonical_network_id != network {
        return Err(FrontendError::Environment);
    }
    let local = network != CanonicalNetworkId::ic_mainnet();
    validate_origin(&input.api_origin, local)?;
    validate_origin(&input.identity.provider_origin, local)?;
    validate_origin(&input.identity.derivation_origin, local)?;
    concrete_principal(input.identity.canister_id)?;
    if admission_nonempty && admission_origin != Some(input.identity.derivation_origin.as_str()) {
        return Err(FrontendError::AdmissionOrigin);
    }
    if input.identity.alternative_origins.len() > MAX_ALTERNATIVE_ORIGINS {
        return Err(FrontendError::Bound("II alternative-origin count"));
    }
    let mut origins = BTreeSet::new();
    for origin in &input.identity.alternative_origins {
        validate_origin(origin, local)?;
        if !origins.insert(origin) {
            return Err(FrontendError::OriginSet);
        }
    }
    if let Some(asset) = &input.asset {
        concrete_principal(asset.canister_id)?;
        validate_origin(&asset.origin, local)?;
        if asset.origin != input.identity.derivation_origin && !origins.contains(&asset.origin) {
            return Err(FrontendError::OriginSet);
        }
    }
    if input.roles.is_empty() || input.roles.len() > MAX_EXPORTED_ROLES {
        return Err(FrontendError::Bound("selected role count"));
    }
    let mut identities = BTreeSet::new();
    for role in &input.roles {
        concrete_principal(role.canister_id)?;
        let label = role.role.as_str();
        let safe_label = !label.is_empty()
            && label.len() <= 128
            && label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'));
        if !safe_label || !identities.insert(role.canister_id) {
            return Err(FrontendError::Role(label.to_string()));
        }
    }
    Ok(())
}
