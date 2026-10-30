//! Issue current portable sessions through IC Auth's real Ed25519 verifier.

use super::*;
use ic_auth::{
    admission_budget::SessionAdmissionBudget,
    portable::{
        MemoryLoginRevocations, PortableAuthority, PortableClaimsContext, PortableClaimsError,
        ValidatedGrants, browser_principal, claims_hash, encode_claims,
    },
    portable_credential::PortableVerificationContext,
    session::{
        MemorySessionStore, RequiredScopes, SessionAdmissionContext, SessionContext,
        SessionEstablished, SessionPolicy,
    },
    session_request::{SessionRequest, SessionTtlLimit, SessionTtlPolicy},
};
use ic_auth_protocol_types::{
    AudienceId, AuthRole, CanonicalId, DelegatedRoleGrant, PortableClaims, PortableMechanism,
    PortableProof,
};
use ic_platform::ed25519::{Signer, SigningKey};
use sha2::{Digest, Sha256};

pub(super) const NOW: u64 = 10_000_000_000;

pub(super) fn audience() -> AudienceId {
    let fleet = fleet_key(1);
    AudienceId {
        canonical_network_id: CanonicalId::from_bytes(*fleet.canonical_network_id.as_bytes()),
        fleet_id: CanonicalId::from_bytes(*fleet.fleet_id.as_bytes()),
    }
}

pub(super) fn ledger() -> MemoryLoginRevocations {
    MemoryLoginRevocations::new(audience(), 4096, 256, 1_000_000_000, 0).unwrap()
}

pub(super) fn browser(id: u16) -> Principal {
    let mut seed = [0; 32];
    seed[..2].copy_from_slice(&id.to_be_bytes());
    browser_principal(&SigningKey::from_bytes(&seed).verifying_key().to_bytes()).into()
}

pub(super) fn pair(second: bool) -> (Session, Replay, LoginRevocationChange) {
    pair_for_browser(second, 1)
}

pub(super) fn pair_for_browser(
    second: bool,
    device: u16,
) -> (Session, Replay, LoginRevocationChange) {
    pair_for_account(second, device, Principal::from_slice(&[43]))
}

pub(super) fn pair_for_account(
    second: bool,
    device: u16,
    account_subject: Principal,
) -> (Session, Replay, LoginRevocationChange) {
    let mut memory = MemorySessionStore::new(0, SessionLimits::new(4, 8, 4, 16_384).unwrap());
    let mut logins = ledger();
    let mut budget = SessionAdmissionBudget::new(32, 1, 0).unwrap();
    let established = establish(
        &mut memory,
        &mut logins,
        &mut budget,
        second,
        device,
        account_subject,
    )
    .unwrap();
    let SessionEstablished::Created(receipt) = established else {
        panic!("fresh session")
    };
    let session = memory.session_records().next().unwrap().clone();
    assert_eq!(session.caller(), receipt.caller());
    let replay =
        Replay::decode(&memory.replay_records().next().unwrap().encode().unwrap()).unwrap();
    let login = logins.records().next().unwrap();
    (session, replay, login)
}

/// Submit a signed credential directly to the selected transaction owner.
pub(super) fn establish(
    store: &mut impl SessionStore,
    logins: &mut MemoryLoginRevocations,
    budget: &mut SessionAdmissionBudget,
    second: bool,
    device: u16,
    account_subject: Principal,
) -> Result<SessionEstablished, SessionError> {
    establish_for_issuer(store, logins, budget, second, device, account_subject, 42)
}

/// One fixture issuance reused verbatim across admission attempts.
pub(super) struct IssuedCredential {
    claims: PortableClaims,
    bytes: Vec<u8>,
    proof: PortableProof,
}

/// Issue independently signed credentials for issuer-scoped account regressions.
pub(super) fn establish_for_issuer(
    store: &mut impl SessionStore,
    logins: &mut MemoryLoginRevocations,
    budget: &mut SessionAdmissionBudget,
    second: bool,
    device: u16,
    account_subject: Principal,
    issuer: u8,
) -> Result<SessionEstablished, SessionError> {
    let credential = issue(second, device, account_subject, issuer);
    establish_issued(store, logins, budget, &credential, browser(device), NOW)
}

pub(super) fn issue(
    second: bool,
    device: u16,
    account_subject: Principal,
    issuer: u8,
) -> IssuedCredential {
    let signer = SigningKey::from_bytes(&[issuer; 32]);
    let role: AuthRole = "app_service".parse().unwrap();
    let mut browser_seed = [0; 32];
    browser_seed[..2].copy_from_slice(&device.to_be_bytes());
    let claims = PortableClaims {
        mechanism: PortableMechanism::Ed25519,
        audience: audience(),
        issuer: Principal::from_slice(&[issuer]).into(),
        account_subject: account_subject.into(),
        key_binding_id: Sha256::digest(signer.verifying_key().to_bytes()).into(),
        browser_public_key: SigningKey::from_bytes(&browser_seed)
            .verifying_key()
            .to_bytes(),
        login_started_at_ns: NOW - 1_000_000_000,
        login_deadline_ns: NOW + 60_000_000_000,
        issued_at_ns: NOW,
        admission_expires_at_ns: NOW + 2_000_000_000,
        nonce: [if second { 2 } else { 1 }; 16],
        grants: vec![DelegatedRoleGrant {
            target: role,
            scopes: vec!["read".into()],
        }],
    };
    let bytes = encode_claims(&claims).unwrap();
    let signature = signer.sign(&claims_hash(&claims).unwrap()).to_bytes();
    IssuedCredential {
        claims,
        bytes,
        proof: PortableProof::Ed25519 {
            signature: signature.to_vec(),
        },
    }
}

pub(super) fn establish_issued(
    store: &mut impl SessionStore,
    logins: &mut MemoryLoginRevocations,
    budget: &mut SessionAdmissionBudget,
    issued: &IssuedCredential,
    observed_caller: Principal,
    now_ns: u64,
) -> Result<SessionEstablished, SessionError> {
    let approved_issuer = [42, 44]
        .into_iter()
        .find(|id| issued.claims.issuer == Principal::from_slice(&[*id]).into())
        .ok_or(SessionError::StaleAuthority)?;
    let public_key = SigningKey::from_bytes(&[approved_issuer; 32])
        .verifying_key()
        .to_bytes();
    let audience = audience();
    let role: AuthRole = "app_service".parse().unwrap();
    let grants = ValidatedGrants::new(vec![DelegatedRoleGrant {
        target: role.clone(),
        scopes: vec!["read".into()],
    }])
    .unwrap();
    let claims = PortableClaimsContext {
        authority: PortableAuthority {
            audience: &audience,
            issuer: Principal::from_slice(&[approved_issuer]).into(),
            mechanism: PortableMechanism::Ed25519,
            key_binding_id: Sha256::digest(public_key).into(),
            grants: &grants,
            caller: observed_caller.into(),
            now_ns,
            authority_not_before_ns: 0,
            authority_expires_at_ns: NOW + 120_000_000_000,
            enrollment_expires_at_ns: NOW + 90_000_000_000,
            min_login_started_at_ns: 0,
        },
        min_proof_issued_at_ns: 0,
        max_login_ttl_ns: 86_400_000_000_000,
        max_proof_ttl_ns: 60_000_000_000,
    };
    ic_auth::session::establish_session(
        store,
        logins,
        budget,
        &ic_auth_protocol_types::PortableCredential {
            claims: issued.bytes.clone(),
            proof: issued.proof.clone(),
        },
        &SessionRequest::new(vec!["read".into()], None).unwrap(),
        &SessionAdmissionContext {
            verification: PortableVerificationContext::Ed25519 {
                claims: &claims,
                public_key: &public_key,
            },
            role: &role,
            ttl: SessionTtlPolicy::new(5, 30).unwrap(),
        },
        SessionPolicy {
            generation: 0,
            enabled: true,
            subject_admissible: true,
        },
    )
}

/// Run the actual shared session decision with protected fixture enrollment.
pub(super) fn authorize(
    store: &mut impl SessionStore,
    ledger: &mut MemoryLoginRevocations,
    now_ns: u64,
    generation: u64,
    required_scopes: &[String],
    refresh: impl FnOnce(&mut PortableAuthority<'_>),
) -> Result<Session, SessionError> {
    let signer = SigningKey::from_bytes(&[42; 32]);
    let audience = audience();
    let role: AuthRole = "app_service".parse().unwrap();
    let grants = ValidatedGrants::new(vec![DelegatedRoleGrant {
        target: role.clone(),
        scopes: vec!["read".into()],
    }])
    .unwrap();
    let required_scopes = RequiredScopes::new(required_scopes.to_vec())?;
    let mut context = SessionContext {
        authority: PortableAuthority {
            audience: &audience,
            issuer: Principal::from_slice(&[42]).into(),
            mechanism: PortableMechanism::Ed25519,
            key_binding_id: Sha256::digest(signer.verifying_key().to_bytes()).into(),
            grants: &grants,
            caller: browser(1).into(),
            now_ns,
            authority_not_before_ns: 0,
            authority_expires_at_ns: NOW + 120_000_000_000,
            enrollment_expires_at_ns: NOW + 90_000_000_000,
            min_login_started_at_ns: 0,
        },
        role: &role,
        required_scopes: &required_scopes,
        max_ttl: SessionTtlLimit::new(30).unwrap(),
    };
    refresh(&mut context.authority);
    ic_auth::session::authorize_session(
        store,
        ledger,
        &context,
        SessionPolicy {
            generation,
            enabled: true,
            subject_admissible: true,
        },
        Session::clone,
    )
}

#[test]
fn submitted_claims_cannot_supply_receiver_authority() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut owner = store();
    let mut logins = ledger();
    let mut budget = SessionAdmissionBudget::new(32, 1, 0).unwrap();
    let original = LocalApplicationAuthorizationState::export();
    for variant in 0..4 {
        let issuer = if variant == 0 { 45 } else { 42 };
        let mut credential = issue(false, 1, Principal::from_slice(&[43]), issuer);
        let expected = match variant {
            0 => SessionError::StaleAuthority,
            1 => {
                credential.claims.grants[0].scopes.push("write".into());
                SessionError::Claims(PortableClaimsError::GrantsExceeded)
            }
            2 => {
                credential.claims.audience.fleet_id = CanonicalId::from_bytes([99; 32]);
                SessionError::Claims(PortableClaimsError::AudienceMismatch)
            }
            _ => {
                credential.claims.key_binding_id = [99; 32];
                SessionError::Claims(PortableClaimsError::KeyMismatch)
            }
        };
        credential.bytes = encode_claims(&credential.claims).unwrap();
        credential.proof = PortableProof::Ed25519 {
            signature: SigningKey::from_bytes(&[issuer; 32])
                .sign(&claims_hash(&credential.claims).unwrap())
                .to_bytes()
                .to_vec(),
        };
        assert_eq!(
            establish_issued(
                &mut owner,
                &mut logins,
                &mut budget,
                &credential,
                browser(1),
                NOW
            ),
            Err(expected)
        );
        assert_eq!(LocalApplicationAuthorizationState::export(), original);
        assert_eq!(logins.occupancy(), (0, 0));
    }
    let allowed = issue(false, 1, Principal::from_slice(&[43]), 42);
    assert!(matches!(
        establish_issued(
            &mut owner,
            &mut logins,
            &mut budget,
            &allowed,
            browser(1),
            NOW
        ),
        Ok(SessionEstablished::Created(_))
    ));
}
