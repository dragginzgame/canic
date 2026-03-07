use crate::{
    InternalError, dto::auth::DelegationCert, ops::runtime::env::EnvOps,
    workflow::rpc::RpcWorkflowError,
};

pub(super) fn validate_delegation_cert_policy(cert: &DelegationCert) -> Result<(), InternalError> {
    if cert.expires_at <= cert.issued_at {
        return Err(RpcWorkflowError::DelegationInvalidWindow {
            issued_at: cert.issued_at,
            expires_at: cert.expires_at,
        }
        .into());
    }

    if cert.aud.is_empty() {
        return Err(RpcWorkflowError::DelegationAudienceEmpty.into());
    }

    if cert.scopes.is_empty() {
        return Err(RpcWorkflowError::DelegationScopesEmpty.into());
    }

    if cert.scopes.iter().any(String::is_empty) {
        return Err(RpcWorkflowError::DelegationScopeEmpty.into());
    }

    let root_pid = EnvOps::root_pid()?;
    if cert.root_pid != root_pid {
        return Err(RpcWorkflowError::DelegationRootPidMismatch(cert.root_pid, root_pid).into());
    }

    if cert.shard_pid == root_pid {
        return Err(RpcWorkflowError::DelegationShardCannotBeRoot.into());
    }

    Ok(())
}
