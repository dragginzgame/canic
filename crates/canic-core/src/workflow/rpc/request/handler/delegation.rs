use crate::{
    InternalError, cdk::types::Principal, dto::auth::DelegationCert, ops::auth::audience,
    workflow::rpc::RpcWorkflowError,
};

pub(super) fn validate_delegation_cert_policy(
    cert: &DelegationCert,
    expected_root_pid: Principal,
) -> Result<(), InternalError> {
    if cert.expires_at <= cert.issued_at {
        return Err(RpcWorkflowError::DelegationInvalidWindow {
            issued_at: cert.issued_at,
            expires_at: cert.expires_at,
        }
        .into());
    }

    if audience::has_empty_roles(&cert.aud) {
        return Err(RpcWorkflowError::DelegationAudienceEmpty.into());
    }

    if cert.scopes.is_empty() {
        return Err(RpcWorkflowError::DelegationScopesEmpty.into());
    }

    if cert.scopes.iter().any(String::is_empty) {
        return Err(RpcWorkflowError::DelegationScopeEmpty.into());
    }

    if cert.root_pid != expected_root_pid {
        return Err(
            RpcWorkflowError::DelegationRootPidMismatch(cert.root_pid, expected_root_pid).into(),
        );
    }

    if cert.shard_pid == expected_root_pid {
        return Err(RpcWorkflowError::DelegationShardCannotBeRoot.into());
    }

    Ok(())
}
