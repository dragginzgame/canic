use crate::{
    Error, ThisError, access::AccessError, infra::ic::network::Network,
    ops::runtime::network::NetworkOps,
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
    BuildNetworkMismatch { expected: Network, actual: Network },
}

impl From<RuleAccessError> for Error {
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
    check_build_network(Network::Ic).map_err(AccessError::from)
}

/// build_network_local
/// Permits access only when `DFX_NETWORK=local` was set at build time.
#[allow(clippy::unused_async)]
pub async fn build_network_local() -> Result<(), AccessError> {
    check_build_network(Network::Local).map_err(AccessError::from)
}

///
/// Helpers
///

pub fn check_build_network(expected: Network) -> Result<(), RuleAccessError> {
    let actual = NetworkOps::current_network();

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

    fn check(expected: Network, actual: Option<Network>) -> Result<(), RuleAccessError> {
        // Inline the same logic but with injected `actual`
        match actual {
            Some(actual) if actual == expected => Ok(()),
            Some(actual) => Err(RuleAccessError::BuildNetworkMismatch { expected, actual }),
            None => Err(RuleAccessError::BuildNetworkUnknown),
        }
    }

    #[test]
    fn build_network_matches_expected() {
        assert!(check(Network::Ic, Some(Network::Ic)).is_ok());
        assert!(check(Network::Local, Some(Network::Local)).is_ok());
    }

    #[test]
    fn build_network_mismatch_errors() {
        let err = check(Network::Ic, Some(Network::Local)).unwrap_err();

        match err {
            RuleAccessError::BuildNetworkMismatch { expected, actual } => {
                assert_eq!(expected, Network::Ic);
                assert_eq!(actual, Network::Local);
            }
            RuleAccessError::BuildNetworkUnknown => panic!("expected BuildNetworkMismatch"),
        }
    }

    #[test]
    fn build_network_unknown_errors() {
        let err = check(Network::Ic, None).unwrap_err();
        assert!(matches!(err, RuleAccessError::BuildNetworkUnknown));
    }
}
