use crate::{
    cdk::types::Principal,
    dto::auth::{
        AttestationKeySet, DelegationProofIssueRequest, DelegationProofPrepareResponse,
        InternalInvocationProofRequest, RoleAttestationRequest, SignedInternalInvocationProofV1,
        SignedRoleAttestation,
    },
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
    const ATTESTATION_KEY_SET: RootAuthMaterialEndpoint =
        RootAuthMaterialEndpoint::structural_bootstrap(protocol::CANIC_ATTESTATION_KEY_SET);
    const DELEGATION_PREPARE: RootAuthMaterialEndpoint =
        RootAuthMaterialEndpoint::structural_bootstrap(protocol::CANIC_PREPARE_DELEGATION_PROOF);
    const INTERNAL_INVOCATION_PROOF: RootAuthMaterialEndpoint =
        RootAuthMaterialEndpoint::structural_bootstrap(
            protocol::CANIC_REQUEST_INTERNAL_INVOCATION_PROOF,
        );
    const ROLE_ATTESTATION: RootAuthMaterialEndpoint =
        RootAuthMaterialEndpoint::structural_bootstrap(protocol::CANIC_REQUEST_ROLE_ATTESTATION);
    #[cfg(test)]
    const ENDPOINTS: &[RootAuthMaterialEndpoint] = &[
        Self::ATTESTATION_KEY_SET,
        Self::DELEGATION_PREPARE,
        Self::INTERNAL_INVOCATION_PROOF,
        Self::ROLE_ATTESTATION,
    ];

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

    pub(super) async fn request_role_attestation(
        &self,
        request: RoleAttestationRequest,
    ) -> Result<SignedRoleAttestation, InternalError> {
        self.call_rpc_result(Self::ROLE_ATTESTATION, request).await
    }

    pub(super) async fn request_internal_invocation_proof(
        &self,
        request: InternalInvocationProofRequest,
    ) -> Result<SignedInternalInvocationProofV1, InternalError> {
        self.call_rpc_result(Self::INTERNAL_INVOCATION_PROOF, request)
            .await
    }

    pub(super) async fn attestation_key_set(&self) -> Result<AttestationKeySet, InternalError> {
        self.call_rpc_result(Self::ATTESTATION_KEY_SET, ()).await
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
        let expected = BTreeSet::from([
            protocol::CANIC_ATTESTATION_KEY_SET,
            protocol::CANIC_PREPARE_DELEGATION_PROOF,
            protocol::CANIC_REQUEST_INTERNAL_INVOCATION_PROOF,
            protocol::CANIC_REQUEST_ROLE_ATTESTATION,
        ]);
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
