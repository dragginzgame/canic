//! Explicit fixture application policy; Root grants alone never authenticate an account.

use canic::{
    api::auth::portable_issuer::PortableIssuancePolicy,
    dto::portable_issuance::{IssuePortableCredential, PortableCredentialIssuanceRefusal as E},
};
use ic_platform::Principal;
use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};

const APPLICATION_SCOPE: &str = "app00:xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx";
// Disposable browser fixture account: Ed25519 seed [70; 32], distinct from browser keys.
const BROWSER_ACCOUNT: &[u8] = &[
    130, 106, 230, 31, 253, 242, 63, 134, 202, 211, 62, 116, 151, 22, 150, 252, 210, 29, 31, 136,
    33, 95, 143, 118, 72, 198, 179, 37, 2,
];

thread_local! {
    static RETRY_DENIED: Cell<bool> = const { Cell::new(false) };
}

/// Authenticate each fixture account independently of the Root's grant ceiling.
pub fn authorize(
    caller: Principal,
    request: &IssuePortableCredential,
) -> Result<PortableIssuancePolicy, E> {
    let approved_account = caller == Principal::from_slice(BROWSER_ACCOUNT)
        || [43, 45, 46, 47]
            .iter()
            .any(|id| caller == Principal::from_slice(&[*id]));
    let ordinary = approved_account
        && request.grants.len() == 1
        && request.grants[0].role == "issuer"
        && request.grants[0].scopes == [APPLICATION_SCOPE];
    let capacity = caller == Principal::from_slice(&[44])
        && request.grants.iter().all(|grant| {
            (grant.role == "issuer" || grant.role.starts_with("wide"))
                && grant
                    .scopes
                    .iter()
                    .all(|scope| scope == APPLICATION_SCOPE || scope.starts_with("wide"))
        });
    if !ordinary && !capacity {
        return Err(E::Denied);
    }
    Ok(PortableIssuancePolicy {
        login_ttl_ns: 3_600_000_000_000,
        proof_ttl_ns: 6_000_000_000,
        receipt_timeout_ns: 10_000_000_000,
        reconciliation_retention_ns: 60_000_000_000,
    })
}

/// Exercise permission changes between the real API's preflight and callback checks.
pub async fn issue(
    request: IssuePortableCredential,
) -> Result<canic::dto::portable_issuance::IssuedPortableCredential, E> {
    let checked = AtomicBool::new(false);
    canic::api::auth::portable_issuer::issue(request, |caller, request| {
        let mut policy = authorize(caller, request)?;
        if checked.swap(true, Ordering::Relaxed) {
            if caller == Principal::from_slice(&[45]) {
                return Err(E::Denied);
            }
            if caller == Principal::from_slice(&[46]) {
                policy.login_ttl_ns /= 2;
            }
            if caller == Principal::from_slice(&[47])
                && !RETRY_DENIED.with(|denied| denied.replace(true))
            {
                return Err(E::Denied);
            }
        }
        Ok(policy)
    })
    .await
}
