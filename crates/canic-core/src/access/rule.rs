use crate::{
    Error, PublicError, ThisError,
    access::AccessError,
    infra::ic::{Network, build_network},
    ops::runtime::env::EnvOps,
};

///
/// RuleError
///

#[derive(Debug, ThisError)]
pub enum RuleError {
    #[error("this endpoint is only available on the prime subnet")]
    NotPrimeSubnet,

    #[error("this endpoint requires a build-time network (DFX_NETWORK) of either 'ic' or 'local'")]
    BuildNetworkUnknown,

    #[error(
        "this endpoint is only available when built for '{expected}' (DFX_NETWORK), but was built for '{actual}'"
    )]
    BuildNetworkMismatch { expected: Network, actual: Network },
}

impl From<RuleError> for Error {
    fn from(err: RuleError) -> Self {
        AccessError::Rule(err).into()
    }
}

impl RuleError {
    pub fn public(&self) -> PublicError {
        PublicError::unauthorized(self.to_string())
    }
}

///
/// Rules
///

#[allow(clippy::unused_async)]
pub async fn is_prime_subnet() -> Result<(), PublicError> {
    if EnvOps::is_prime_subnet() {
        Ok(())
    } else {
        Err(RuleError::NotPrimeSubnet.public())
    }
}

///
/// build_network_ic
/// Permits access only when `DFX_NETWORK=ic` was set at build time.
///

#[allow(clippy::unused_async)]
pub async fn build_network_ic() -> Result<(), PublicError> {
    check_build_network(Network::Ic).map_err(|err| err.public())
}

///
/// build_network_local
/// Permits access only when `DFX_NETWORK=local` was set at build time.
///

#[allow(clippy::unused_async)]
pub async fn build_network_local() -> Result<(), PublicError> {
    check_build_network(Network::Local).map_err(|err| err.public())
}

///
/// Helpers
///

pub(crate) fn check_build_network(expected: Network) -> Result<(), RuleError> {
    let actual = build_network();

    match actual {
        Some(actual) if actual == expected => Ok(()),
        Some(actual) => Err(RuleError::BuildNetworkMismatch { expected, actual }),
        None => Err(RuleError::BuildNetworkUnknown),
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

    fn check(expected: Network, actual: Option<Network>) -> Result<(), RuleError> {
        // Inline the same logic but with injected `actual`
        match actual {
            Some(actual) if actual == expected => Ok(()),
            Some(actual) => Err(RuleError::BuildNetworkMismatch { expected, actual }),
            None => Err(RuleError::BuildNetworkUnknown),
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
            RuleError::BuildNetworkMismatch { expected, actual } => {
                assert_eq!(expected, Network::Ic);
                assert_eq!(actual, Network::Local);
            }
            _ => panic!("expected BuildNetworkMismatch"),
        }
    }

    #[test]
    fn build_network_unknown_errors() {
        let err = check(Network::Ic, None).unwrap_err();
        assert!(matches!(err, RuleError::BuildNetworkUnknown));
    }
}
