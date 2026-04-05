use super::dependency_unavailable;
use crate::{
    access::AccessError,
    cdk::{api::msg_arg_data, candid::de::IDLDeserialize, types::Principal},
    dto::auth::DelegatedToken,
    ops::{
        auth::{DelegatedTokenOps, VerifiedDelegatedToken},
        ic::IcOps,
        runtime::env::EnvOps,
    },
};

const MAX_INGRESS_BYTES: usize = 64 * 1024; // 64 KiB

///
/// CallerBoundToken
///
/// Verified delegated token that has passed caller-subject binding.
struct CallerBoundToken {
    verified: VerifiedDelegatedToken,
}

impl CallerBoundToken {
    /// bind_to_caller
    ///
    /// Enforce subject binding and return a caller-bound token wrapper.
    fn bind_to_caller(
        verified: VerifiedDelegatedToken,
        caller: Principal,
    ) -> Result<Self, AccessError> {
        enforce_subject_binding(verified.claims.subject(), caller)?;
        Ok(Self { verified })
    }

    /// scopes
    ///
    /// Borrow token scopes after caller binding has been enforced.
    fn scopes(&self) -> &[String] {
        self.verified.claims.scopes()
    }

    /// into_verified
    ///
    /// Unwrap the verified delegated token for downstream consumers.
    fn into_verified(self) -> VerifiedDelegatedToken {
        self.verified
    }
}

pub(super) async fn delegated_token_verified(
    authenticated_subject: Principal,
    required_scope: Option<&str>,
) -> Result<VerifiedDelegatedToken, AccessError> {
    let token = delegated_token_from_args()?;

    let authority_pid =
        EnvOps::root_pid().map_err(|_| dependency_unavailable("root pid unavailable"))?;

    let now_secs = IcOps::now_secs();
    let self_pid = IcOps::canister_self();

    verify_token(
        token,
        authenticated_subject,
        authority_pid,
        now_secs,
        self_pid,
        required_scope,
    )
    .await
}

/// Verify a delegated token against the configured authority.
#[expect(clippy::unused_async)]
async fn verify_token(
    token: DelegatedToken,
    caller: Principal,
    authority_pid: Principal,
    now_secs: u64,
    self_pid: Principal,
    required_scope: Option<&str>,
) -> Result<VerifiedDelegatedToken, AccessError> {
    let verified = DelegatedTokenOps::verify_token(&token, authority_pid, now_secs, self_pid)
        .map_err(|err| AccessError::Denied(err.to_string()))?;

    let caller_bound = CallerBoundToken::bind_to_caller(verified, caller)?;
    enforce_required_scope(required_scope, caller_bound.scopes())?;

    Ok(caller_bound.into_verified())
}

pub(super) fn enforce_subject_binding(
    sub: Principal,
    caller: Principal,
) -> Result<(), AccessError> {
    if sub == caller {
        Ok(())
    } else {
        Err(AccessError::Denied(format!(
            "delegated token subject '{sub}' does not match caller '{caller}'"
        )))
    }
}

pub(super) fn enforce_required_scope(
    required_scope: Option<&str>,
    token_scopes: &[String],
) -> Result<(), AccessError> {
    let Some(required_scope) = required_scope else {
        return Ok(());
    };

    if token_scopes.iter().any(|scope| scope == required_scope) {
        Ok(())
    } else {
        Err(AccessError::Denied(format!(
            "delegated token missing required scope '{required_scope}'"
        )))
    }
}

fn delegated_token_from_args() -> Result<DelegatedToken, AccessError> {
    let bytes = msg_arg_data();

    if bytes.len() > MAX_INGRESS_BYTES {
        return Err(AccessError::Denied(
            "delegated token payload exceeds size limit".to_string(),
        ));
    }

    let mut decoder = IDLDeserialize::new(&bytes)
        .map_err(|err| AccessError::Denied(format!("failed to decode ingress arguments: {err}")))?;

    decoder.get_value::<DelegatedToken>().map_err(|err| {
        AccessError::Denied(format!(
            "failed to decode delegated token as first argument: {err}"
        ))
    })
}
