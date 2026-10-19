//! Issue current portable sessions through IC Auth's real Ed25519 verifier.

use super::*;
use ed25519_dalek::{Signer, SigningKey};
use ic_auth::{
    admission_budget::SessionAdmissionBudget,
    portable::{
        MemoryLoginRevocations, PortableClaimsContext, browser_principal, claims_hash,
        encode_claims,
    },
    portable_credential::PortableVerificationContext,
    session::{MemorySessionStore, SessionContext, SessionEstablished, SessionPolicy},
    session_request::SessionRequest,
};
use ic_auth_protocol_types::{
    AudienceId, AuthRole, CanonicalId, DelegatedRoleGrant, PortableClaims, PortableMechanism,
    PortableProof,
};
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
    browser_principal(&SigningKey::from_bytes(&seed).verifying_key().to_bytes())
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
    let signer = SigningKey::from_bytes(&[42; 32]);
    let role: AuthRole = "app_service".parse().unwrap();
    let mut browser_seed = [0; 32];
    browser_seed[..2].copy_from_slice(&device.to_be_bytes());
    let claims = PortableClaims {
        mechanism: PortableMechanism::Ed25519,
        audience: audience(),
        issuer: Principal::from_slice(&[42]),
        account_subject,
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
            target: role.clone(),
            scopes: vec!["read".into()],
        }],
    };
    let context = SessionContext {
        authority: PortableClaimsContext {
            audience: &claims.audience,
            issuer: claims.issuer,
            mechanism: claims.mechanism,
            key_binding_id: claims.key_binding_id,
            grants: &claims.grants,
            caller: browser(device),
            now_ns: NOW,
            authority_not_before_ns: 0,
            authority_expires_at_ns: NOW + 120_000_000_000,
            enrollment_expires_at_ns: NOW + 90_000_000_000,
            min_login_started_at_ns: 0,
            min_proof_issued_at_ns: 0,
            max_login_ttl_ns: 86_400_000_000_000,
            max_proof_ttl_ns: 60_000_000_000,
        },
        role: &role,
        required_scopes: &[],
    };
    let mut memory = MemorySessionStore::new(
        0,
        SessionLimits {
            max_sessions: 4,
            max_replays: 8,
            max_replays_per_caller: 4,
            max_encoded_bytes: 16_384,
        },
    )
    .unwrap();
    let mut logins = ledger();
    let mut budget = SessionAdmissionBudget::new(32, 1, 0).unwrap();
    let bytes = encode_claims(&claims).unwrap();
    let signature = signer.sign(&claims_hash(&claims).unwrap()).to_bytes();
    let established = ic_auth::session::establish_session(
        &mut memory,
        &mut logins,
        &mut budget,
        &bytes,
        &SessionRequest::new(vec!["read".into()], None).unwrap(),
        &context,
        SessionPolicy {
            generation: 0,
            enabled: true,
            subject_admissible: true,
            default_ttl_secs: 5,
            max_ttl_secs: 30,
        },
        |budget, decoded| {
            Ok(decoded.verify(
                budget,
                &PortableProof::Ed25519 {
                    signature: signature.to_vec(),
                },
                &PortableVerificationContext::Ed25519 {
                    claims: &context.authority,
                    public_key: &signer.verifying_key().to_bytes(),
                },
            )?)
        },
    )
    .unwrap();
    let SessionEstablished::Created(session) = established else {
        panic!("fresh session")
    };
    let replay =
        Replay::decode(&memory.replay_records().next().unwrap().encode().unwrap()).unwrap();
    let login = logins.records().next().unwrap();
    (session, replay, login)
}

/// Run the actual shared session decision with protected fixture enrollment.
pub(super) fn authorize(
    store: &mut impl SessionStore,
    ledger: &mut MemoryLoginRevocations,
    now_ns: u64,
    generation: u64,
    required_scopes: &[String],
) -> Result<Session, SessionError> {
    let signer = SigningKey::from_bytes(&[42; 32]);
    let audience = audience();
    let role: AuthRole = "app_service".parse().unwrap();
    let grants = vec![DelegatedRoleGrant {
        target: role.clone(),
        scopes: vec!["read".into()],
    }];
    let context = SessionContext {
        authority: PortableClaimsContext {
            audience: &audience,
            issuer: Principal::from_slice(&[42]),
            mechanism: PortableMechanism::Ed25519,
            key_binding_id: Sha256::digest(signer.verifying_key().to_bytes()).into(),
            grants: &grants,
            caller: browser(1),
            now_ns,
            authority_not_before_ns: 0,
            authority_expires_at_ns: NOW + 120_000_000_000,
            enrollment_expires_at_ns: NOW + 90_000_000_000,
            min_login_started_at_ns: 0,
            min_proof_issued_at_ns: 0,
            max_login_ttl_ns: 86_400_000_000_000,
            max_proof_ttl_ns: 60_000_000_000,
        },
        role: &role,
        required_scopes,
    };
    ic_auth::session::authorize_session(
        store,
        ledger,
        &context,
        SessionPolicy {
            generation,
            enabled: true,
            subject_admissible: true,
            default_ttl_secs: 5,
            max_ttl_secs: 30,
        },
    )
}
