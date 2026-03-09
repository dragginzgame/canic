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

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cdk::types::Principal,
        storage::stable::env::{Env, EnvRecord},
    };

    struct EnvRestore(EnvRecord);

    impl Drop for EnvRestore {
        fn drop(&mut self) {
            Env::import(self.0.clone());
        }
    }

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn sample_cert(root_pid: Principal) -> DelegationCert {
        DelegationCert {
            root_pid,
            shard_pid: p(2),
            issued_at: 100,
            expires_at: 200,
            scopes: vec!["rpc:verify".to_string()],
            aud: vec![p(3)],
        }
    }

    #[test]
    fn validate_delegation_cert_policy_rejects_invalid_window() {
        let original = Env::export();
        let _restore = EnvRestore(original);

        let root_pid = p(1);
        Env::import(EnvRecord {
            root_pid: Some(root_pid),
            ..EnvRecord::default()
        });

        let mut cert = sample_cert(root_pid);
        cert.expires_at = cert.issued_at;

        let err = validate_delegation_cert_policy(&cert).expect_err("invalid window must fail");
        assert!(
            err.to_string()
                .contains("expires_at must be greater than issued_at")
        );
    }

    #[test]
    fn validate_delegation_cert_policy_rejects_empty_audience() {
        let original = Env::export();
        let _restore = EnvRestore(original);

        let root_pid = p(1);
        Env::import(EnvRecord {
            root_pid: Some(root_pid),
            ..EnvRecord::default()
        });

        let mut cert = sample_cert(root_pid);
        cert.aud.clear();

        let err = validate_delegation_cert_policy(&cert).expect_err("empty audience must fail");
        assert!(
            err.to_string()
                .contains("delegation audience must not be empty")
        );
    }

    #[test]
    fn validate_delegation_cert_policy_rejects_empty_scopes() {
        let original = Env::export();
        let _restore = EnvRestore(original);

        let root_pid = p(1);
        Env::import(EnvRecord {
            root_pid: Some(root_pid),
            ..EnvRecord::default()
        });

        let mut cert = sample_cert(root_pid);
        cert.scopes.clear();

        let err = validate_delegation_cert_policy(&cert).expect_err("empty scopes must fail");
        assert!(
            err.to_string()
                .contains("delegation scopes must not be empty")
        );
    }

    #[test]
    fn validate_delegation_cert_policy_rejects_empty_scope_values() {
        let original = Env::export();
        let _restore = EnvRestore(original);

        let root_pid = p(1);
        Env::import(EnvRecord {
            root_pid: Some(root_pid),
            ..EnvRecord::default()
        });

        let mut cert = sample_cert(root_pid);
        cert.scopes = vec![String::new()];

        let err = validate_delegation_cert_policy(&cert).expect_err("empty scope value must fail");
        assert!(err.to_string().contains("must not contain empty strings"));
    }

    #[test]
    fn validate_delegation_cert_policy_rejects_root_pid_mismatch() {
        let original = Env::export();
        let _restore = EnvRestore(original);

        let root_pid = p(1);
        Env::import(EnvRecord {
            root_pid: Some(root_pid),
            ..EnvRecord::default()
        });

        let cert = sample_cert(p(9));
        let err = validate_delegation_cert_policy(&cert).expect_err("root pid mismatch must fail");
        assert!(err.to_string().contains("delegation root pid mismatch"));
    }

    #[test]
    fn validate_delegation_cert_policy_rejects_shard_equal_to_root() {
        let original = Env::export();
        let _restore = EnvRestore(original);

        let root_pid = p(1);
        Env::import(EnvRecord {
            root_pid: Some(root_pid),
            ..EnvRecord::default()
        });

        let mut cert = sample_cert(root_pid);
        cert.shard_pid = root_pid;

        let err = validate_delegation_cert_policy(&cert).expect_err("root shard must fail");
        assert!(
            err.to_string()
                .contains("delegation shard_pid must not equal root pid")
        );
    }
}
