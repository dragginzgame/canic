use crate::{
    access::{AccessError, AccessRuleError, AccessRuleResult, RuleAccessError},
    cdk::types::Principal,
    ids::{BuildNetwork, CanisterRole},
    ops::{
        ic::network::NetworkOps,
        storage::{
            directory::{app::AppDirectoryOps, subnet::SubnetDirectoryOps},
            registry::subnet::SubnetRegistryOps,
        },
    },
};

///
/// Rules
///

/// Ensure the caller matches the subnet directory entry recorded for `role`.
/// Use for admin endpoints that expect specific app directory canisters.
#[must_use]
pub fn is_app_directory_role(caller: Principal, role: CanisterRole) -> AccessRuleResult {
    Box::pin(async move {
        if AppDirectoryOps::matches(&role, caller) {
            Ok(())
        } else {
            Err(AccessRuleError::NotAppDirectoryType(caller, role).into())
        }
    })
}

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

/// Require that the caller equals the provided `expected` principal.
/// Handy for single-tenant or pre-registered callers.
#[must_use]
pub fn is_principal(caller: Principal, expected: Principal) -> AccessRuleResult {
    Box::pin(async move {
        if caller == expected {
            Ok(())
        } else {
            Err(AccessRuleError::NotPrincipal(caller, expected).into())
        }
    })
}

/// Require that the caller is registered as a canister on this subnet.
///
/// NOTE: Currently enforced only on the root canister.
#[must_use]
pub fn is_registered_to_subnet(caller: Principal) -> AccessRuleResult {
    Box::pin(async move {
        if SubnetRegistryOps::is_registered(caller) {
            Ok(())
        } else {
            Err(AccessRuleError::NotRegisteredToSubnet(caller).into())
        }
    })
}

/// Ensure the caller matches the subnet directory entry recorded for `role`.
/// Use for admin endpoints that expect specific app directory canisters.
#[must_use]
pub fn is_subnet_directory_role(caller: Principal, role: CanisterRole) -> AccessRuleResult {
    Box::pin(async move {
        match SubnetDirectoryOps::get(&role) {
            Some(pid) if pid == caller => Ok(()),
            _ => Err(AccessRuleError::NotSubnetDirectoryType(caller, role).into()),
        }
    })
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
