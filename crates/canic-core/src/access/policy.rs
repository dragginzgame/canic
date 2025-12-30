use crate::{
    Error, ThisError,
    access::AccessError,
    infra::ic::{Network, build_network},
    ops::runtime::env::EnvOps,
};

///
/// PolicyError
///

#[derive(Debug, ThisError)]
pub enum PolicyError {
    #[error("this endpoint is only available on the prime subnet")]
    NotPrimeSubnet,

    #[error("this endpoint requires a build-time network (DFX_NETWORK) of either 'ic' or 'local'")]
    BuildNetworkUnknown,

    #[error(
        "this endpoint is only available when built for '{expected}' (DFX_NETWORK), but was built for '{actual}'"
    )]
    BuildNetworkMismatch { expected: Network, actual: Network },
}

impl From<PolicyError> for Error {
    fn from(err: PolicyError) -> Self {
        AccessError::PolicyError(err).into()
    }
}

///
/// Policies
///

#[allow(clippy::unused_async)]
pub async fn is_prime_subnet() -> Result<(), Error> {
    if EnvOps::is_prime_subnet() {
        Ok(())
    } else {
        Err(PolicyError::NotPrimeSubnet.into())
    }
}

///
/// build_network_ic
/// Permits access only when `DFX_NETWORK=ic` was set at build time.
///

#[allow(clippy::unused_async)]
pub async fn build_network_ic() -> Result<(), Error> {
    check_build_network(Network::Ic).map_err(Into::into)
}

///
/// build_network_local
/// Permits access only when `DFX_NETWORK=local` was set at build time.
///

#[allow(clippy::unused_async)]
pub async fn build_network_local() -> Result<(), Error> {
    check_build_network(Network::Local).map_err(Into::into)
}

///
/// Helpers
///

pub(crate) fn check_build_network(expected: Network) -> Result<(), PolicyError> {
    let actual = build_network();

    match actual {
        Some(actual) if actual == expected => Ok(()),
        Some(actual) => Err(PolicyError::BuildNetworkMismatch { expected, actual }),
        None => Err(PolicyError::BuildNetworkUnknown),
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

    fn check(expected: Network, actual: Option<Network>) -> Result<(), PolicyError> {
        // Inline the same logic but with injected `actual`
        match actual {
            Some(actual) if actual == expected => Ok(()),
            Some(actual) => Err(PolicyError::BuildNetworkMismatch { expected, actual }),
            None => Err(PolicyError::BuildNetworkUnknown),
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
            PolicyError::BuildNetworkMismatch { expected, actual } => {
                assert_eq!(expected, Network::Ic);
                assert_eq!(actual, Network::Local);
            }
            _ => panic!("expected BuildNetworkMismatch"),
        }
    }

    #[test]
    fn build_network_unknown_errors() {
        let err = check(Network::Ic, None).unwrap_err();
        assert!(matches!(err, PolicyError::BuildNetworkUnknown));
    }
}
