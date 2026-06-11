use crate::{
    cdk::types::Principal,
    dto::auth::{DelegationProofIssueRequest, DelegationProofPrepareResponse},
    error::InternalError,
    ops::rpc::RpcOps,
    protocol,
};
use candid::CandidType;
use serde::de::DeserializeOwned;

///
/// RootAuthMaterialClient
///
pub(super) struct RootAuthMaterialClient {
    root_pid: Principal,
}

impl RootAuthMaterialClient {
    const DELEGATION_PREPARE: RootAuthMaterialEndpoint =
        RootAuthMaterialEndpoint::structural_bootstrap(protocol::CANIC_PREPARE_DELEGATION_PROOF);
    #[cfg(test)]
    const ENDPOINTS: &[RootAuthMaterialEndpoint] = &[Self::DELEGATION_PREPARE];

    pub(super) const fn new(root_pid: Principal) -> Self {
        Self { root_pid }
    }

    pub(super) async fn prepare_delegation_proof(
        &self,
        request: DelegationProofIssueRequest,
    ) -> Result<DelegationProofPrepareResponse, InternalError> {
        self.call_rpc_result(Self::DELEGATION_PREPARE, request)
            .await
    }

    async fn call_rpc_result<T, A>(
        &self,
        endpoint: RootAuthMaterialEndpoint,
        arg: A,
    ) -> Result<T, InternalError>
    where
        T: CandidType + DeserializeOwned,
        A: CandidType,
    {
        debug_assert!(endpoint.is_structural_bootstrap());
        RpcOps::call_rpc_result(self.root_pid, endpoint.method, arg).await
    }
}

///
/// RootAuthMaterialEndpoint
///
#[derive(Clone, Copy)]
struct RootAuthMaterialEndpoint {
    method: &'static str,
    class: RootAuthMaterialEndpointClass,
}

impl RootAuthMaterialEndpoint {
    const fn structural_bootstrap(method: &'static str) -> Self {
        Self {
            method,
            class: RootAuthMaterialEndpointClass::StructuralBootstrap,
        }
    }

    const fn is_structural_bootstrap(&self) -> bool {
        matches!(
            self.class,
            RootAuthMaterialEndpointClass::StructuralBootstrap
        )
    }
}

///
/// RootAuthMaterialEndpointClass
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RootAuthMaterialEndpointClass {
    StructuralBootstrap,
}

#[cfg(test)]
mod tests {
    use super::{RootAuthMaterialClient, RootAuthMaterialEndpointClass};
    use crate::protocol;
    use std::collections::BTreeSet;

    #[test]
    fn root_auth_material_client_endpoint_table_is_structural_bootstrap_only() {
        let expected = BTreeSet::from([protocol::CANIC_PREPARE_DELEGATION_PROOF]);
        let actual = RootAuthMaterialClient::ENDPOINTS
            .iter()
            .inspect(|endpoint| {
                assert_eq!(
                    endpoint.class,
                    RootAuthMaterialEndpointClass::StructuralBootstrap
                );
            })
            .map(|endpoint| endpoint.method)
            .collect::<BTreeSet<_>>();

        assert_eq!(actual, expected);
        assert_eq!(
            actual.len(),
            RootAuthMaterialClient::ENDPOINTS.len(),
            "root auth material client endpoint methods must be unique"
        );
    }
}
