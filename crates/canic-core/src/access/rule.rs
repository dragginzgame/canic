use crate::{
    InternalError, ThisError, access::AccessError, ids::BuildNetwork, ops::ic::network::NetworkOps,
};

///
/// RuleAccessError
///

#[derive(Debug, ThisError)]
pub enum RuleAccessError {
    #[error("this endpoint requires a build-time network (DFX_NETWORK) of either 'ic' or 'local'")]
    BuildNetworkUnknown,

    #[error(
        "this endpoint is only available when built for '{expected}' (DFX_NETWORK), but was built for '{actual}'"
    )]
    BuildNetworkMismatch {
        expected: BuildNetwork,
        actual: BuildNetwork,
    },
}

impl From<RuleAccessError> for InternalError {
    fn from(err: RuleAccessError) -> Self {
        AccessError::Rule(err).into()
    }
}

///
/// Rules
///

/// build_network_ic
/// Permits access only when `DFX_NETWORK=ic` was set at build time.
#[allow(clippy::unused_async)]
pub async fn build_network_ic() -> Result<(), AccessError> {
    check_build_network(BuildNetwork::Ic).map_err(AccessError::from)
}

/// build_network_local
/// Permits access only when `DFX_NETWORK=local` was set at build time.
#[allow(clippy::unused_async)]
pub async fn build_network_local() -> Result<(), AccessError> {
    check_build_network(BuildNetwork::Local).map_err(AccessError::from)
}

///
/// Helpers
///

pub fn check_build_network(expected: BuildNetwork) -> Result<(), RuleAccessError> {
    let actual = NetworkOps::build_network();

    match actual {
        Some(actual) if actual == expected => Ok(()),
        Some(actual) => Err(RuleAccessError::BuildNetworkMismatch { expected, actual }),
        None => Err(RuleAccessError::BuildNetworkUnknown),
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

    fn check(expected: BuildNetwork, actual: Option<BuildNetwork>) -> Result<(), RuleAccessError> {
        // Inline the same logic but with injected `actual`
        match actual {
            Some(actual) if actual == expected => Ok(()),
            Some(actual) => Err(RuleAccessError::BuildNetworkMismatch { expected, actual }),
            None => Err(RuleAccessError::BuildNetworkUnknown),
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

        match err {
            RuleAccessError::BuildNetworkMismatch { expected, actual } => {
                assert_eq!(expected, BuildNetwork::Ic);
                assert_eq!(actual, BuildNetwork::Local);
            }
            RuleAccessError::BuildNetworkUnknown => panic!("expected BuildNetworkMismatch"),
        }
    }

    #[test]
    fn build_network_unknown_errors() {
        let err = check(BuildNetwork::Ic, None).unwrap_err();
        assert!(matches!(err, RuleAccessError::BuildNetworkUnknown));
    }
}
