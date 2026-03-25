use crate::{cdk::types::Principal, dto::auth::DelegationCert};

use super::{DelegationScopeError, TokenAudience, TokenGrant};

// Return true when the principal is present in the allowed set.
pub fn principal_allowed(target: Principal, allowed: &[Principal]) -> bool {
    allowed.contains(&target)
}

// Return true when every principal in `subset` is present in `superset`.
pub fn principals_subset(subset: &[Principal], superset: &[Principal]) -> bool {
    subset.iter().all(|item| principal_allowed(*item, superset))
}

// Return true when every string in `subset` is present in `superset`.
pub fn strings_subset(subset: &[String], superset: &[String]) -> bool {
    subset.iter().all(|item| superset.contains(item))
}

// Verify that this canister is explicitly included in the delegated audience.
pub fn verify_self_audience(
    audience: TokenAudience<'_>,
    self_pid: Principal,
) -> Result<(), DelegationScopeError> {
    if principal_allowed(self_pid, audience.aud) {
        Ok(())
    } else {
        Err(DelegationScopeError::SelfAudienceMissing { self_pid })
    }
}

// Validate token claims against the bounds encoded in the delegation cert.
pub fn validate_claims_against_cert(
    grant: TokenGrant<'_>,
    cert: &DelegationCert,
) -> Result<(), DelegationScopeError> {
    if grant.shard_pid != cert.shard_pid {
        return Err(DelegationScopeError::ShardPidMismatch {
            expected: cert.shard_pid,
            found: grant.shard_pid,
        });
    }

    if !principals_subset(grant.aud, &cert.aud) {
        for aud in grant.aud {
            if !principal_allowed(*aud, &cert.aud) {
                return Err(DelegationScopeError::AudienceNotAllowed { aud: *aud });
            }
        }
    }

    if !strings_subset(grant.scopes, &cert.scopes) {
        for scope in grant.scopes {
            if !cert.scopes.iter().any(|allowed| allowed == scope) {
                return Err(DelegationScopeError::ScopeNotAllowed {
                    scope: scope.clone(),
                });
            }
        }
    }

    Ok(())
}
