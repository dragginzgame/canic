//! Reusable native backend scenarios for this source cohort. These assertions
//! exercise Auth's public engine with real fixture signatures, not a production
//! issuer. Passing them does not establish backing provenance, crash atomicity,
//! bounded cleanup, durable clock/authority metadata or production adoption.
use ic_auth::{
    admission_budget::SessionAdmissionBudget,
    portable::{
        LoginRevocationChange, MemoryLoginRevocations, PortableAuthority, PortableClaimsContext,
        ValidatedGrants, browser_principal, claims_hash, encode_claims,
    },
    portable_credential::PortableVerificationContext,
    portable_signature::PreparedIssuerKey,
    session::*,
    session_request::{SessionRequest, SessionTtlPolicy},
};
use ic_auth_protocol_types::*;
use ic_platform::ed25519::{Signer, SigningKey};

const SECOND: u64 = 1_000_000_000;
const NOW: u64 = 10 * SECOND;

/// Test-driver hooks, not another production storage interface. Each `fresh`
/// call must allocate isolated test state with the requested protected bounds.
/// Reopen must reconstruct from the adapter's persisted representation. Failure
/// injection applies to the next admission/logout commit, before any mutation.
pub trait Backend: SessionStore + Sized {
    fn fresh(generation: u64, limits: SessionLimits) -> Self;
    fn fail_next_commit(&mut self);
    fn reopen(self) -> Self;
    fn advance_generation(&mut self);
    /// Complete physical login records, from the same serialized checkpoint.
    fn retained_logins(&self) -> Vec<LoginRevocationChange>;
}

/// Run the same admission, authorization, retry, rollback, logout, reopen and capacity scenarios
/// against a host adapter. Panics identify a violated assertion. Use only with
/// disposable qualification backing; this deliberately injects failed commits.
pub fn assert_backend_contract<B: Backend>() {
    assert_admission_retry::<B>();
    assert_logout_reopen::<B>();
    assert_replacement_generation::<B>();
    assert_capacity::<B>();
    assert_authorization::<B>();
    assert_maximum_shape::<B>();
}

/// Maximum grant/scopes/label bounds survive admission, record restoration and
/// scoped authorization. This is a shape contract, not a host cost measurement.
pub fn assert_maximum_shape<B: Backend>() {
    let mut f = Fixture::new(1);
    f.claims.grants = (0..limits::MAX_GRANTS)
        .map(|role| DelegatedRoleGrant {
            target: format!("r{role:02}{}", "r".repeat(limits::MAX_LABEL_BYTES - 3))
                .parse()
                .unwrap(),
            scopes: (0..limits::MAX_SCOPES_PER_GRANT)
                .map(|scope| {
                    format!("s{scope:02}{}", "s".repeat(limits::MAX_LABEL_BYTES - 3))
                        .parse()
                        .unwrap()
                })
                .collect(),
        })
        .collect();
    let grant = f.claims.grants.last().unwrap();
    f.role = grant.target.clone();
    let requested = grant.scopes[grant.scopes.len() - limits::MAX_SESSION_SCOPES..].to_vec();
    f.grants = ValidatedGrants::new(f.claims.grants.clone()).unwrap();
    let required =
        RequiredScopes::new(requested.iter().map(ToString::to_string).collect()).unwrap();
    let mut store = B::fresh(0, limits(4, 65_536));
    let mut ledger = f.ledger();
    let mut budget = SessionAdmissionBudget::new(1, NOW, 0).unwrap();
    let admitted = f
        .admit_request(
            &mut store,
            &mut ledger,
            &mut budget,
            NOW,
            SessionRequest::new(requested.iter().map(ToString::to_string).collect(), None).unwrap(),
        )
        .unwrap();
    let before = f.authorize(&mut store, &mut ledger, NOW, 0).unwrap();
    assert_eq!(before.scopes(), requested);
    assert_eq!(before.expires_at_ns(), admitted.receipt().expires_at_ns());
    let before_records = snapshot(&mut store, &f, &Fixture::new(2));
    let mut store = store.reopen();
    let mut ledger = f
        .ledger_at(NOW)
        .restore_records(
            ledger.observed_at_ns(),
            ledger.occupancy(),
            store.retained_logins(),
        )
        .unwrap();
    let after = authorize_session(
        &mut store,
        &mut ledger,
        &SessionContext {
            authority: f.authority(NOW),
            max_ttl: SessionTtlPolicy::new(5, 30).unwrap().maximum(),
            role: &f.role,
            required_scopes: &required,
        },
        policy(0),
        Session::clone,
    )
    .unwrap();
    assert_eq!(after, before, "maximum-size session must survive reopen");
    assert_eq!(snapshot(&mut store, &f, &Fixture::new(2)), before_records);
}

/// Only the admitted caller with current issuer authority and an admitted scope
/// may invoke the operation. Exact expiry refuses without pruning or mutation,
/// including after reopening the host's persisted records.
pub fn assert_authorization<B: Backend>() {
    let f = Fixture::new(1);
    let other = Fixture::new(2);
    let read = RequiredScopes::new(vec!["read".into()]).unwrap();
    // The issuer grants write, but this session requested only read.
    let write = RequiredScopes::new(vec!["write".into()]).unwrap();
    for reopen in [false, true] {
        let mut store = B::fresh(0, limits(4, 65_536));
        let mut ledger = f.ledger();
        let mut budget = SessionAdmissionBudget::new(1, NOW, 0).unwrap();
        let expires = f
            .admit(&mut store, &mut ledger, &mut budget, NOW)
            .unwrap()
            .receipt()
            .expires_at_ns();
        let before = snapshot(&mut store, &f, &other);
        if reopen {
            store = store.reopen();
            ledger = f
                .ledger_at(NOW)
                .restore_records(
                    ledger.observed_at_ns(),
                    ledger.occupancy(),
                    store.retained_logins(),
                )
                .unwrap();
            assert_eq!(snapshot(&mut store, &f, &other), before);
        }
        let counts = ledger.occupancy();
        let mut wrong_issuer = f.authority(NOW);
        wrong_issuer.issuer = Principal::from_slice(&[99]);
        for (case, authority, required, expected) in [
            ("admitted read", f.authority(NOW), &read, Ok(f.caller())),
            (
                "unadmitted scope",
                f.authority(NOW),
                &write,
                Err(SessionError::ScopeRejected),
            ),
            (
                "unadmitted caller",
                other.authority(NOW),
                &read,
                Err(SessionError::MissingOrExpired),
            ),
            (
                "different issuer, same subject",
                wrong_issuer,
                &read,
                Err(SessionError::StaleAuthority),
            ),
            (
                "before expiry",
                f.authority(expires - 1),
                &read,
                Ok(f.caller()),
            ),
            (
                "exact expiry",
                f.authority(expires),
                &read,
                Err(SessionError::MissingOrExpired),
            ),
            (
                "expiry before scope refusal",
                f.authority(expires),
                &write,
                Err(SessionError::MissingOrExpired),
            ),
            (
                "after expiry",
                f.authority(expires + 1),
                &read,
                Err(SessionError::MissingOrExpired),
            ),
        ] {
            let mut invoked = false;
            let result = authorize_session(
                &mut store,
                &mut ledger,
                &SessionContext {
                    authority,
                    max_ttl: SessionTtlPolicy::new(5, 30).unwrap().maximum(),
                    role: &f.role,
                    required_scopes: required,
                },
                policy(0),
                |session| {
                    invoked = true;
                    assert_eq!(session.issuer(), f.claims.issuer);
                    assert_eq!(session.subject(), f.claims.account_subject);
                    session.caller()
                },
            );
            assert_eq!(invoked, expected.is_ok(), "{case}; reopened={reopen}");
            assert_eq!(result, expected, "{case}; reopened={reopen}");
            assert_eq!(snapshot(&mut store, &f, &other), before, "{case}");
            assert_eq!(ledger.occupancy(), counts, "{case}");
        }
    }
}

/// Failed admission is atomic, consumes proof credit, and exact retry preserves
/// the original lease after proof expiry. Uses fresh disposable backing.
pub fn assert_admission_retry<B: Backend>() {
    let f = Fixture::new(1);
    let other = Fixture::new(2);
    let mut store = B::fresh(0, limits(4, 65_536));
    let mut ledger = f.ledger();
    let mut budget = SessionAdmissionBudget::new(1, NOW, 0).unwrap();
    let before = snapshot(&mut store, &f, &other);
    store.fail_next_commit();
    assert_eq!(
        f.admit(&mut store, &mut ledger, &mut budget, NOW),
        Err(SessionError::StorageFailure)
    );
    assert_eq!(
        snapshot(&mut store, &f, &other),
        before,
        "failed admission must preserve every record"
    );
    assert_eq!(ledger.occupancy(), (0, 0));
    // Verification work remains charged even when the host commit fails.
    assert_eq!(
        f.admit(&mut store, &mut ledger, &mut budget, NOW),
        Err(SessionError::Credential(
            ic_auth::portable_credential::PortableCredentialError::Ed25519(
                ic_auth::portable_signature::PortableSignatureError::Budget(
                    ic_auth::admission_budget::AdmissionBudgetError::Exhausted
                )
            )
        ))
    );
    let mut budget = SessionAdmissionBudget::new(1, NOW, 0).unwrap();
    let created = f.admit(&mut store, &mut ledger, &mut budget, NOW).unwrap();
    assert!(matches!(created, SessionEstablished::Created(_)));
    let after = snapshot(&mut store, &f, &other);
    assert_eq!(
        f.admit(&mut store, &mut ledger, &mut budget, NOW + 3 * SECOND),
        Ok(SessionEstablished::ExactRetry(created.receipt()))
    );
    assert_eq!(
        snapshot(&mut store, &f, &other),
        after,
        "retry must preserve the original lease and records after proof expiry"
    );
}

/// Reopening retains authorization; failed logout preserves it; committed logout
/// survives another reopen while retaining consumed-proof evidence.
pub fn assert_logout_reopen<B: Backend>() {
    let f = Fixture::new(1);
    let other = Fixture::new(2);
    let mut store = B::fresh(0, limits(4, 65_536));
    let mut ledger = f.ledger();
    let mut budget = SessionAdmissionBudget::new(1, NOW, 0).unwrap();
    f.admit(&mut store, &mut ledger, &mut budget, NOW).unwrap();
    let after = snapshot(&mut store, &f, &other);
    let mut store = store.reopen();
    let observed = ledger.observed_at_ns();
    ledger = f
        .ledger_at(NOW + 3 * SECOND)
        .restore_records(observed, ledger.occupancy(), store.retained_logins())
        .unwrap();
    assert!(
        f.authorize(&mut store, &mut ledger, NOW + 3 * SECOND, 0)
            .is_ok()
    );
    assert_eq!(snapshot(&mut store, &f, &other), after);
    let mut logout_budget = SessionAdmissionBudget::new(2, SECOND, 0).unwrap();
    store.fail_next_commit();
    assert_eq!(
        logout_session(
            &mut store,
            &mut ledger,
            &mut logout_budget,
            f.caller(),
            NOW + 3 * SECOND
        ),
        LogoutResult::Unavailable(LogoutUnavailable::Storage)
    );
    assert_eq!(
        snapshot(&mut store, &f, &other),
        after,
        "failed logout must preserve session, replay and reservation"
    );
    assert!(
        f.authorize(&mut store, &mut ledger, NOW + 3 * SECOND, 0)
            .is_ok()
    );
    assert!(matches!(
        logout_session(
            &mut store,
            &mut ledger,
            &mut logout_budget,
            f.caller(),
            NOW + 3 * SECOND
        ),
        LogoutResult::Revoked { .. }
    ));
    let revoked = snapshot(&mut store, &f, &other);
    assert!(revoked.sessions[0].is_none());
    assert_eq!(
        revoked.occupancy[0].replays, 1,
        "logout retains replay evidence"
    );
    let mut store = store.reopen();
    ledger = f
        .ledger_at(NOW + 3 * SECOND)
        .restore_records(
            ledger.observed_at_ns(),
            ledger.occupancy(),
            store.retained_logins(),
        )
        .unwrap();
    assert_eq!(
        f.authorize(&mut store, &mut ledger, NOW + 3 * SECOND, 0),
        Err(SessionError::Revocation(
            ic_auth::portable::LoginRevocationError::Revoked
        ))
    );
    assert_eq!(snapshot(&mut store, &f, &other), revoked);
}

/// Replacement retains replay history; generation invalidation survives reopen.
pub fn assert_replacement_generation<B: Backend>() {
    let f = Fixture::new(1);
    let other = Fixture::new(2);
    // A fresh proof for the same login replaces the lease, retaining both replays.
    let mut store = B::fresh(0, limits(4, 65_536));
    let mut ledger = f.ledger();
    let mut budget = SessionAdmissionBudget::new(8, SECOND, 0).unwrap();
    f.admit(&mut store, &mut ledger, &mut budget, NOW).unwrap();
    let mut renewal = Fixture::new(1);
    renewal.claims.nonce = [2; 16];
    assert!(matches!(
        renewal.admit(&mut store, &mut ledger, &mut budget, NOW),
        Ok(SessionEstablished::Replaced(_))
    ));
    let before = snapshot(&mut store, &f, &other);
    assert_eq!(before.occupancy[0].replays, 2);
    store.advance_generation();
    assert_eq!(
        f.authorize(&mut store, &mut ledger, NOW, 0),
        Err(SessionError::StaleAuthority)
    );
    assert_eq!(
        f.authorize(&mut store, &mut ledger, NOW, 1),
        Err(SessionError::StaleAuthority)
    );
    let mut store = store.reopen();
    let changed = snapshot(&mut store, &f, &other);
    assert_eq!(changed.generation, 1);
    assert_eq!(changed.logins, before.logins);
    assert_eq!(changed.occupancy[0].replays, 2);
}

/// Session, global/per-caller replay and encoded-byte exhaustion refuse without
/// changing records or ledger. Replacement must still respect replay capacity.
pub fn assert_capacity<B: Backend>() {
    let f = Fixture::new(1);
    let other = Fixture::new(2);
    for (bounds, expected) in [
        (limits(1, 65_536), SessionError::Capacity),
        (limits(4, 1), SessionError::ByteCapacity),
    ] {
        let mut store = B::fresh(0, bounds);
        let mut ledger = f.ledger();
        let mut budget = SessionAdmissionBudget::new(8, SECOND, 0).unwrap();
        if bounds.max_encoded_bytes() > 1 {
            f.admit(&mut store, &mut ledger, &mut budget, NOW).unwrap();
        }
        let before = snapshot(&mut store, &f, &other);
        let counts = ledger.occupancy();
        assert_eq!(
            other.admit(&mut store, &mut ledger, &mut budget, NOW),
            Err(expected)
        );
        assert_eq!(snapshot(&mut store, &f, &other), before);
        assert_eq!(ledger.occupancy(), counts);
    }
    for (scope, bounds, device) in [
        (
            "global replay",
            SessionLimits::new(4, 1, 4, 65_536).unwrap(),
            2,
        ),
        (
            "caller replay",
            SessionLimits::new(4, 8, 1, 65_536).unwrap(),
            1,
        ),
    ] {
        let mut store = B::fresh(0, bounds);
        let mut ledger = f.ledger();
        let mut budget = SessionAdmissionBudget::new(8, SECOND, 0).unwrap();
        let created = f.admit(&mut store, &mut ledger, &mut budget, NOW).unwrap();
        let mut candidate = Fixture::new(device);
        candidate.claims.nonce = [2; 16];
        let before = snapshot(&mut store, &f, &candidate);
        let counts = ledger.occupancy();
        assert_eq!(
            candidate.admit(&mut store, &mut ledger, &mut budget, NOW),
            Err(SessionError::Capacity),
            "{scope} exhaustion must refuse before writing"
        );
        assert_eq!(snapshot(&mut store, &f, &candidate), before);
        assert_eq!(ledger.occupancy(), counts);
        let mut store = store.reopen();
        assert_eq!(snapshot(&mut store, &f, &candidate), before);
        assert_eq!(
            f.admit(&mut store, &mut ledger, &mut budget, NOW),
            Ok(SessionEstablished::ExactRetry(created.receipt())),
            "a full replay pool must still allow the original exact retry"
        );
        if device == 1 {
            // Per-caller saturation does not exhaust the receiver-wide pool.
            assert!(matches!(
                other.admit(&mut store, &mut ledger, &mut budget, NOW),
                Ok(SessionEstablished::Created(_))
            ));
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
struct Snapshot {
    generation: u64,
    sessions: [Option<Vec<u8>>; 2],
    occupancy: [SessionOccupancy; 2],
    replays: [bool; 2],
    logins: Vec<Vec<u8>>,
}
fn snapshot(store: &mut impl Backend, f: &Fixture, other: &Fixture) -> Snapshot {
    let mut logins: Vec<_> = store
        .retained_logins()
        .iter()
        .map(|r| r.encode().unwrap())
        .collect();
    logins.sort();
    store
        .read(|reader| {
            Ok(Snapshot {
                generation: reader.generation(),
                sessions: [f.caller(), other.caller()]
                    .map(|caller| reader.session(caller).unwrap().map(|s| s.encode().unwrap())),
                occupancy: [
                    reader.occupancy(f.caller())?,
                    reader.occupancy(other.caller())?,
                ],
                replays: [
                    reader.replay_exists(claims_hash(&f.claims).unwrap())?,
                    reader.replay_exists(claims_hash(&other.claims).unwrap())?,
                ],
                logins,
            })
        })
        .unwrap()
}
fn limits(sessions: usize, bytes: usize) -> SessionLimits {
    SessionLimits::new(sessions, 8, 4, bytes).unwrap()
}
fn policy(generation: u64) -> SessionPolicy {
    SessionPolicy {
        generation,
        enabled: true,
        subject_admissible: true,
    }
}
struct Fixture {
    claims: PortableClaims,
    key: SigningKey,
    prepared: PreparedIssuerKey,
    role: AuthRole,
    grants: ValidatedGrants,
}
impl Fixture {
    fn new(device: u8) -> Self {
        let key = SigningKey::from_bytes(&[42; 32]);
        let prepared = PreparedIssuerKey::new(key.verifying_key().to_bytes()).unwrap();
        let role: AuthRole = "user".parse().unwrap();
        let claims = PortableClaims {
            audience: AudienceId {
                canonical_network_id: CanonicalId::from_bytes([1; 32]),
                fleet_id: CanonicalId::from_bytes([2; 32]),
            },
            mechanism: PortableMechanism::Ed25519,
            issuer: Principal::from_slice(&[1]),
            account_subject: Principal::from_slice(&[2]),
            key_binding_id: prepared.binding(),
            browser_public_key: SigningKey::from_bytes(&[device; 32])
                .verifying_key()
                .to_bytes(),
            login_started_at_ns: NOW - SECOND,
            login_deadline_ns: NOW + 60 * SECOND,
            issued_at_ns: NOW,
            admission_expires_at_ns: NOW + 2 * SECOND,
            nonce: [1; 16],
            grants: vec![DelegatedRoleGrant {
                target: role.clone(),
                scopes: vec!["read".parse().unwrap(), "write".parse().unwrap()],
            }],
        };
        let grants = ValidatedGrants::new(claims.grants.clone()).unwrap();
        Self {
            claims,
            key,
            prepared,
            role,
            grants,
        }
    }
    fn caller(&self) -> Principal {
        browser_principal(&self.claims.browser_public_key)
    }
    fn ledger(&self) -> MemoryLoginRevocations {
        self.ledger_at(0)
    }
    fn ledger_at(&self, now: u64) -> MemoryLoginRevocations {
        MemoryLoginRevocations::new(self.claims.audience, 4, 4, SECOND, now).unwrap()
    }
    fn authority(&self, now: u64) -> PortableAuthority<'_> {
        PortableAuthority {
            audience: &self.claims.audience,
            issuer: self.claims.issuer,
            mechanism: self.claims.mechanism,
            key_binding_id: self.claims.key_binding_id,
            grants: &self.grants,
            caller: self.caller(),
            now_ns: now,
            authority_not_before_ns: 0,
            authority_expires_at_ns: NOW + 120 * SECOND,
            enrollment_expires_at_ns: NOW + 90 * SECOND,
            min_login_started_at_ns: 0,
        }
    }
    fn admit(
        &self,
        store: &mut impl SessionStore,
        ledger: &mut MemoryLoginRevocations,
        budget: &mut SessionAdmissionBudget,
        now: u64,
    ) -> Result<SessionEstablished, SessionError> {
        self.admit_request(
            store,
            ledger,
            budget,
            now,
            SessionRequest::new(vec!["read".into()], None).unwrap(),
        )
    }
    fn admit_request(
        &self,
        store: &mut impl SessionStore,
        ledger: &mut MemoryLoginRevocations,
        budget: &mut SessionAdmissionBudget,
        now: u64,
        request: SessionRequest,
    ) -> Result<SessionEstablished, SessionError> {
        let claims = PortableClaimsContext {
            authority: self.authority(now),
            min_proof_issued_at_ns: 0,
            max_login_ttl_ns: 86_400 * SECOND,
            max_proof_ttl_ns: 60 * SECOND,
        };
        let credential = PortableCredential {
            claims: encode_claims(&self.claims).unwrap(),
            proof: PortableProof::Ed25519 {
                signature: self
                    .key
                    .sign(&claims_hash(&self.claims).unwrap())
                    .to_bytes()
                    .to_vec(),
            },
        };
        establish_session(
            store,
            ledger,
            budget,
            &credential,
            request,
            &SessionAdmissionContext {
                verification: PortableVerificationContext::Ed25519 {
                    claims: &claims,
                    public_key: &self.prepared,
                },
                role: &self.role,
                ttl: SessionTtlPolicy::new(5, 30).unwrap(),
            },
            policy(0),
        )
    }
    fn authorize(
        &self,
        store: &mut impl SessionStore,
        ledger: &mut MemoryLoginRevocations,
        now: u64,
        generation: u64,
    ) -> Result<Session, SessionError> {
        authorize_session(
            store,
            ledger,
            &SessionContext {
                authority: self.authority(now),
                max_ttl: SessionTtlPolicy::new(5, 30).unwrap().maximum(),
                role: &self.role,
                required_scopes: RequiredScopes::NONE,
            },
            policy(generation),
            Session::clone,
        )
    }
}
