use crate::{
    cdk::types::Principal,
    dto::auth::{DelegatedTokenClaims, DelegationCert},
    ops::prelude::*,
};

//
// TokenAudience
//

pub struct TokenAudience<'a> {
    pub aud: &'a [Principal],
}

//
// TokenGrant
//

pub struct TokenGrant<'a> {
    pub shard_pid: Principal,
    pub aud: &'a [Principal],
    pub scopes: &'a [String],
}

//
// TokenLifetime
//

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TokenLifetime {
    pub iat: u64,
    pub exp: u64,
}

//
// VerifiedTokenClaims
//

#[derive(CandidType, Clone, Debug, Eq, PartialEq)]
pub struct VerifiedTokenClaims {
    sub: Principal,
    shard_pid: Principal,
    scopes: Vec<String>,
    aud: Vec<Principal>,
    iat: u64,
    exp: u64,
    ext: Option<Vec<u8>>,
}

impl VerifiedTokenClaims {
    // Build internal verified claims from the boundary DTO shape.
    #[must_use]
    pub fn from_dto(claims: DelegatedTokenClaims) -> Self {
        Self {
            sub: claims.sub,
            shard_pid: claims.shard_pid,
            scopes: claims.scopes,
            aud: claims.aud,
            iat: claims.iat,
            exp: claims.exp,
            ext: claims.ext,
        }
    }

    // Build internal verified claims from a borrowed DTO payload.
    #[must_use]
    pub fn from_dto_ref(claims: &DelegatedTokenClaims) -> Self {
        Self::from_dto(claims.clone())
    }

    // Convert internal verified claims back into the boundary DTO shape.
    #[must_use]
    pub fn to_dto(&self) -> DelegatedTokenClaims {
        DelegatedTokenClaims {
            sub: self.sub,
            shard_pid: self.shard_pid,
            scopes: self.scopes.clone(),
            aud: self.aud.clone(),
            iat: self.iat,
            exp: self.exp,
            ext: self.ext.clone(),
        }
    }

    // Borrow the audience-only subset used by verifier-local checks.
    #[must_use]
    pub fn audience(&self) -> TokenAudience<'_> {
        TokenAudience { aud: &self.aud }
    }

    // Borrow the grant-bound subset used against delegation certs.
    #[must_use]
    pub fn grant(&self) -> TokenGrant<'_> {
        TokenGrant {
            shard_pid: self.shard_pid,
            aud: &self.aud,
            scopes: &self.scopes,
        }
    }

    // Return the token lifetime bounds.
    #[must_use]
    pub const fn lifetime(&self) -> TokenLifetime {
        TokenLifetime {
            iat: self.iat,
            exp: self.exp,
        }
    }

    // Return the authenticated token subject.
    #[must_use]
    pub const fn subject(&self) -> Principal {
        self.sub
    }

    // Return the shard that signed or will sign the token.
    #[must_use]
    pub const fn shard_pid(&self) -> Principal {
        self.shard_pid
    }

    // Return the token expiry timestamp.
    #[must_use]
    pub const fn expires_at(&self) -> u64 {
        self.exp
    }

    // Borrow the granted scopes.
    #[must_use]
    pub fn scopes(&self) -> &[String] {
        &self.scopes
    }
}

//
// VerifiedDelegatedToken
//

pub struct VerifiedDelegatedToken {
    pub claims: VerifiedTokenClaims,
    pub cert: DelegationCert,
}

impl VerifiedDelegatedToken {
    // Convert verified token contents back into DTO parts for boundary consumers.
    #[must_use]
    pub fn into_parts(self) -> (DelegatedTokenClaims, DelegationCert) {
        (self.claims.to_dto(), self.cert)
    }
}
