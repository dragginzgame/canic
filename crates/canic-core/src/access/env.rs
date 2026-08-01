//! Module: access::env
//!
//! Responsibility: gate endpoint access by canister environment and build network.
//! Does not own: caller identity, app mode, or endpoint response mapping.
//! Boundary: access expressions call this for self/environment predicates.

use crate::{
    access::AccessError,
    ids::BuildNetwork,
    ops::{ic::build_network::BuildNetworkOps, runtime::env::EnvOps},
};

// -----------------------------------------------------------------------------
// Env Checks
// -----------------------------------------------------------------------------

/// is_fleet_subnet_root
///
/// Permit access only from the configured Fleet Subnet Root canister.
pub fn is_fleet_subnet_root() -> Result<(), AccessError> {
    if EnvOps::is_fleet_subnet_root() {
        Ok(())
    } else {
        Err(AccessError::Denied(
            "this endpoint is only available on the Fleet Subnet Root".to_string(),
        ))
    }
}

/// build_network_ic
///
/// Permits access only when `ICP_ENVIRONMENT=ic` was set at build time.
pub fn build_network_ic() -> Result<(), AccessError> {
    check_build_network(BuildNetwork::Ic)
}

/// build_network_local
///
/// Permits access only when `ICP_ENVIRONMENT=local` was set at build time.
pub fn build_network_local() -> Result<(), AccessError> {
    check_build_network(BuildNetwork::Local)
}

// -----------------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------------

/// check_build_network
///
/// Permit access only when the build network matches the expected build network.
pub fn check_build_network(expected: BuildNetwork) -> Result<(), AccessError> {
    let actual = BuildNetworkOps::build_network();

    match actual {
        Some(actual) if actual == expected => Ok(()),
        Some(actual) => Err(AccessError::Denied(format!(
            "this endpoint is only available when built for '{expected}' (ICP_ENVIRONMENT), but was built for '{actual}'"
        ))),
        None => Err(AccessError::Denied(
            "this endpoint requires a build-time network (ICP_ENVIRONMENT) of either 'ic' or 'local'"
                .to_string(),
        )),
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn check(expected: BuildNetwork, actual: Option<BuildNetwork>) -> Result<(), AccessError> {
        // Inline the same logic but with injected `actual`.
        match actual {
            Some(actual) if actual == expected => Ok(()),
            Some(actual) => Err(AccessError::Denied(format!(
                "this endpoint is only available when built for '{expected}' (ICP_ENVIRONMENT), but was built for '{actual}'"
            ))),
            None => Err(AccessError::Denied(
                "this endpoint requires a build-time network (ICP_ENVIRONMENT) of either 'ic' or 'local'"
                    .to_string(),
            )),
        }
    }

    #[test]
    fn build_network_matches_expected() {
        assert!(check(BuildNetwork::Ic, Some(BuildNetwork::Ic)).is_ok());
        assert!(check(BuildNetwork::Local, Some(BuildNetwork::Local)).is_ok());
    }

    #[test]
    fn build_network_mismatch_errors() {
        let err = check(BuildNetwork::Ic, Some(BuildNetwork::Local)).unwrap_err();
        assert_eq!(err.kind(), crate::access::AccessErrorKind::Denied);
    }

    #[test]
    fn build_network_unknown_errors() {
        let err = check(BuildNetwork::Ic, None).unwrap_err();
        assert_eq!(err.kind(), crate::access::AccessErrorKind::Denied);
    }
}
