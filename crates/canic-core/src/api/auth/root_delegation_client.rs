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
/// RootDelegationProofClient
///
pub(super) struct RootDelegationProofClient {
    root_pid: Principal,
}

impl RootDelegationProofClient {
    #[cfg(test)]
    const ENDPOINTS: &[&'static str] = &[protocol::CANIC_PREPARE_DELEGATION_PROOF];

    pub(super) const fn new(root_pid: Principal) -> Self {
        Self { root_pid }
    }

    pub(super) async fn prepare_delegation_proof(
        &self,
        request: DelegationProofIssueRequest,
    ) -> Result<DelegationProofPrepareResponse, InternalError> {
        self.call_rpc_result(protocol::CANIC_PREPARE_DELEGATION_PROOF, request)
            .await
    }

    async fn call_rpc_result<T, A>(&self, method: &'static str, arg: A) -> Result<T, InternalError>
    where
        T: CandidType + DeserializeOwned,
        A: CandidType,
    {
        RpcOps::call_rpc_result(self.root_pid, method, arg).await
    }
}

#[cfg(test)]
mod tests {
    use super::RootDelegationProofClient;
    use crate::protocol;
    use std::collections::BTreeSet;

    #[test]
    fn root_delegation_proof_client_endpoint_table_is_prepare_only() {
        let expected = BTreeSet::from([protocol::CANIC_PREPARE_DELEGATION_PROOF]);
        let actual = RootDelegationProofClient::ENDPOINTS
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();

        assert_eq!(actual, expected);
        assert_eq!(
            actual.len(),
            RootDelegationProofClient::ENDPOINTS.len(),
            "root delegation proof client endpoint methods must be unique"
        );
    }
}
