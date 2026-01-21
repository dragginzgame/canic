//! Build/network access predicates.
//!
//! This bucket is limited to build-time environment gating.

use crate::{access::AccessError, ids::BuildNetwork, ops::ic::network::NetworkOps};

/// build_network_ic
/// Permits access only when `DFX_NETWORK=ic` was set at build time.
#[expect(clippy::unused_async)]
pub async fn build_network_ic() -> Result<(), AccessError> {
    check_build_network(BuildNetwork::Ic)
}

/// build_network_local
/// Permits access only when `DFX_NETWORK=local` was set at build time.
#[expect(clippy::unused_async)]
pub async fn build_network_local() -> Result<(), AccessError> {
    check_build_network(BuildNetwork::Local)
}

///
/// Helpers
///

pub fn check_build_network(expected: BuildNetwork) -> Result<(), AccessError> {
    let actual = NetworkOps::build_network();

    match actual {
        Some(actual) if actual == expected => Ok(()),
        Some(actual) => Err(AccessError::Denied(format!(
            "this endpoint is only available when built for '{expected}' (DFX_NETWORK), but was built for '{actual}'"
        ))),
        None => Err(AccessError::Denied(
            "this endpoint requires a build-time network (DFX_NETWORK) of either 'ic' or 'local'"
                .to_string(),
        )),
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

    fn check(expected: BuildNetwork, actual: Option<BuildNetwork>) -> Result<(), AccessError> {
        // Inline the same logic but with injected `actual`
        match actual {
            Some(actual) if actual == expected => Ok(()),
            Some(actual) => Err(AccessError::Denied(format!(
                "this endpoint is only available when built for '{expected}' (DFX_NETWORK), but was built for '{actual}'"
            ))),
            None => Err(AccessError::Denied(
                "this endpoint requires a build-time network (DFX_NETWORK) of either 'ic' or 'local'"
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
        let err = check(BuildNetwork::Ic, Some(BuildNetwork::Local))
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("this endpoint is only available when built for 'ic' (DFX_NETWORK)"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn build_network_unknown_errors() {
        let err = check(BuildNetwork::Ic, None).unwrap_err().to_string();
        assert!(
            err.contains("this endpoint requires a build-time network (DFX_NETWORK)"),
            "unexpected error: {err}"
        );
    }
}
