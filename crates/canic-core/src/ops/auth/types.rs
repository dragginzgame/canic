use crate::dto::auth::{DelegatedTokenClaims, DelegationCert};

pub struct VerifiedDelegatedToken {
    pub claims: DelegatedTokenClaims,
    pub cert: DelegationCert,
}
