//! Native receiver composition with genuine Ed25519 proofs and the retained store.

use super::*;
use crate::{
    config::schema::{PortableReceiverConfig, Validate},
    ops::storage::auth::application_sessions::{
        ApplicationSessionStateError, ApplicationSessionTestStateGuard, PersistentReceiverPolicy,
    },
    test::{seams, support::fleet_key},
};
use canic_contracts::dto::portable_enrollment::PortableEnrollmentGrant;
use ic_auth::portable::{browser_principal, claims_hash, encode_claims};
use ic_auth_protocol_types::{DelegatedRoleGrant, PortableClaims, PortableProof};
use ic_platform::ed25519::{Signer, SigningKey};

const NOW: u64 = 10_000_000_000;
const START: u64 = NOW - 1_000_000_000;

fn p(id: u8) -> Principal {
    Principal::from_slice(&[id])
}

fn profile() -> PortableReceiverConfig {
    PortableReceiverConfig {
        max_sessions: 2048,
        clock_skew_ns: 1,
        verification_burst: 4,
        verification_refill_interval_ns: 1,
        unknown_logout_burst: 2,
        unknown_logout_refill_interval_ns: 1,
        enrollment_refresh_interval_ns: 1_000_000_000,
        enrollment_receipt_timeout_ns: 1_000_000_000,
        max_login_ttl_ns: 60_000_000_000,
        max_proof_ttl_ns: 2_000_000_000,
        max_certificate_age_ns: 60_000_000_000,
    }
}

fn owner() -> PersistentReceiverState {
    LocalApplicationAuthorizationStateOps::initialize_receiver_floors(START, 1).unwrap();
    PersistentReceiverState::restore(
        PersistentReceiverPolicy {
            max_sessions: 2048,
            fleet: fleet_key(1),
            clock_skew_ns: 1,
            verification_burst: 4,
            verification_refill_interval_ns: 1,
            unknown_logout_burst: 2,
            unknown_logout_refill_interval_ns: 1,
        },
        START,
    )
    .unwrap()
}

struct Fixture {
    config: LocalApplicationAuthorizationConfig,
    identity: LocalApplicationAuthoritySnapshot,
    snapshot: PortableEnrollmentSnapshot,
}

impl Fixture {
    fn new() -> Self {
        Self {
            config: LocalApplicationAuthorizationConfig {
                receiver: Some(profile()),
                allowed_scopes: vec!["read".into()],
                default_session_ttl_secs: 5,
                maximum_session_ttl_secs: 30,
            },
            identity: LocalApplicationAuthoritySnapshot::new(fleet_key(1), "app_service".into()),
            snapshot: PortableEnrollmentSnapshot {
                authority: p(1),
                receiver: p(2),
                fleet: fleet_key(1),
                boot_challenge: [9; 32],
                revision: 1,
                generation: 1,
                issued_at_ns: START,
                expires_at_ns: NOW + 90_000_000_000,
                entries: vec![PortableIssuerEnrollment {
                    issuer: p(42),
                    key: PortableIssuerKey::Ed25519 {
                        public_key: SigningKey::from_bytes(&[42; 32]).verifying_key().to_bytes(),
                    },
                    not_before_ns: START,
                    expires_at_ns: NOW + 120_000_000_000,
                    grants: vec![PortableEnrollmentGrant {
                        role: "app_service".into(),
                        scopes: vec!["read".into()],
                    }],
                    deployment_approval_digest: [7; 32],
                    deployment_approval_expires_at_ns: NOW + 120_000_000_000,
                }],
                retired_keys: vec![],
            },
        }
    }

    fn context(&self, caller: Principal, now_ns: u64) -> PortableReceiverContext<'_> {
        PortableReceiverContext {
            config: &self.config,
            identity: &self.identity,
            snapshot: &self.snapshot,
            caller,
            now_ns,
        }
    }
}

fn caller(device: u8) -> Principal {
    browser_principal(
        &SigningKey::from_bytes(&[device; 32])
            .verifying_key()
            .to_bytes(),
    )
}

fn issue_at(now_ns: u64, device: u8) -> SessionInput {
    let signer = SigningKey::from_bytes(&[42; 32]);
    let fleet = fleet_key(1);
    let claims = PortableClaims {
        mechanism: PortableMechanism::Ed25519,
        audience: AudienceId {
            canonical_network_id: CanonicalId::from_bytes(*fleet.canonical_network_id.as_bytes()),
            fleet_id: CanonicalId::from_bytes(*fleet.fleet_id.as_bytes()),
        },
        issuer: p(42),
        account_subject: p(43),
        key_binding_id: Sha256::digest(signer.verifying_key().to_bytes()).into(),
        browser_public_key: SigningKey::from_bytes(&[device; 32])
            .verifying_key()
            .to_bytes(),
        login_started_at_ns: now_ns - 1,
        login_deadline_ns: now_ns + 60_000_000_000 - 1,
        issued_at_ns: now_ns,
        admission_expires_at_ns: now_ns + 2_000_000_000,
        nonce: [device; 16],
        grants: vec![DelegatedRoleGrant {
            target: "app_service".parse().unwrap(),
            scopes: vec!["read".parse().unwrap()],
        }],
    };
    SessionInput {
        credential: PortableCredential {
            claims: encode_claims(&claims).unwrap(),
            proof: PortableProof::Ed25519 {
                signature: signer
                    .sign(&claims_hash(&claims).unwrap())
                    .to_bytes()
                    .to_vec(),
            },
        }
        .try_into()
        .unwrap(),
        scopes: ic_wire::List::new(vec![ic_wire::Text::new("read".into()).unwrap()]).unwrap(),
        ttl_secs: None,
    }
}

fn browser() -> Principal {
    caller(3)
}
fn issue() -> SessionInput {
    issue_at(NOW, 3)
}

#[test]
fn portable_receiver_real_admission_read_exact_retry_and_durable_logout() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut receiver = owner();
    let fixture = Fixture::new();
    let input = issue();
    let view = admit(
        &mut receiver,
        fixture.context(browser(), NOW),
        input.clone(),
    )
    .unwrap();
    assert_eq!(
        (view.issuer, view.subject, view.caller),
        (p(42), p(43), browser())
    );
    assert_eq!(view.expires_at_ns, NOW + 5_000_000_000);
    let before = LocalApplicationAuthorizationState::export();
    let read_view = read(
        &mut receiver,
        fixture.context(browser(), NOW + 1),
        vec!["read".into()],
        false,
    )
    .unwrap();
    assert_eq!(read_view, view);
    assert_eq!(
        LocalApplicationAuthorizationState::export(),
        before,
        "query must not publish its clock observation"
    );
    let retry_time = NOW + 2_000_000_000;
    let retried = admit(
        &mut receiver,
        fixture.context(browser(), retry_time),
        input.clone(),
    )
    .unwrap();
    assert_eq!(
        retried, view,
        "same request retries retain the original expiry after proof expiry"
    );
    let revoked = logout(&mut receiver, browser(), retry_time + 1).unwrap();
    assert!(matches!(
        revoked,
        ic_auth_protocol_types::LogoutResult::Revoked { .. }
    ));
    let mut restored = PersistentReceiverState::restore(
        PersistentReceiverPolicy {
            max_sessions: 2048,
            fleet: fleet_key(1),
            clock_skew_ns: 1,
            verification_burst: 4,
            verification_refill_interval_ns: 1,
            unknown_logout_burst: 2,
            unknown_logout_refill_interval_ns: 1,
        },
        retry_time + 2,
    )
    .unwrap();
    assert!(
        read(
            &mut restored,
            fixture.context(browser(), retry_time + 2),
            vec!["read".into()],
            false
        )
        .is_err()
    );
    assert!(
        admit(
            &mut restored,
            fixture.context(browser(), retry_time + 3),
            input
        )
        .is_err()
    );
}

#[test]
fn portable_receiver_scope_caller_expiry_and_current_authority_are_enforced() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut receiver = owner();
    let mut fixture = Fixture::new();
    assert!(admit(&mut receiver, fixture.context(p(9), NOW), issue()).is_err());
    let mut broad = issue();
    broad.scopes = ic_wire::List::new(vec![ic_wire::Text::new("write".into()).unwrap()]).unwrap();
    assert!(matches!(
        admit(&mut receiver, fixture.context(browser(), NOW), broad),
        Err(PortableSessionError::Session(SessionError::ScopeRejected))
    ));
    admit(&mut receiver, fixture.context(browser(), NOW), issue()).unwrap();
    assert!(matches!(
        read(
            &mut receiver,
            fixture.context(browser(), NOW),
            vec!["write".into()],
            true
        ),
        Err(PortableSessionError::Receiver(
            PersistentReceiverError::Session(SessionError::ScopeRejected)
        ))
    ));
    fixture.snapshot.entries.clear();
    assert!(matches!(
        read(&mut receiver, fixture.context(browser(), NOW), vec![], true),
        Err(PortableSessionError::Receiver(
            PersistentReceiverError::Session(SessionError::StaleAuthority)
        ))
    ));
    fixture = Fixture::new();
    assert!(matches!(
        read(
            &mut receiver,
            fixture.context(browser(), NOW + 5_000_000_000),
            vec![],
            true
        ),
        Err(PortableSessionError::Expired)
    ));
}

#[test]
fn portable_receiver_logout_needs_no_enrollment_and_backward_clock_never_acknowledges() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut receiver = owner();
    let mut fixture = Fixture::new();
    fixture.snapshot.expires_at_ns = NOW + 1_000_000_000;
    admit(&mut receiver, fixture.context(browser(), NOW), issue()).unwrap();
    // Logout takes only the retained owner, actual caller and trusted time. No gate is consulted.
    assert!(matches!(
        logout(&mut receiver, browser(), fixture.snapshot.expires_at_ns),
        Ok(ic_auth_protocol_types::LogoutResult::Revoked { .. })
    ));
    let before = LocalApplicationAuthorizationState::export();
    assert!(matches!(
        logout(&mut receiver, browser(), NOW),
        Err(PortableSessionError::Receiver(
            PersistentReceiverError::Clock
        ))
    ));
    assert_eq!(LocalApplicationAuthorizationState::export(), before);
}

#[test]
fn portable_receiver_reset_floors_are_retained_and_require_proven_empty_state() {
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut receiver = owner();
    let first = LocalApplicationAuthorizationState::receiver_floors().unwrap();
    assert_eq!(first.min_proof_issued_at_ns, START + 1);
    assert_eq!(first.min_login_started_at_ns, START + 2);
    let fixture = Fixture::new();
    admit(&mut receiver, fixture.context(browser(), NOW), issue()).unwrap();
    assert_eq!(
        LocalApplicationAuthorizationStateOps::initialize_receiver_floors(NOW, 99).unwrap(),
        first
    );
    let mut state = LocalApplicationAuthorizationState::export();
    state.receiver_floors = None;
    LocalApplicationAuthorizationState::import(state);
    assert_eq!(
        LocalApplicationAuthorizationStateOps::initialize_receiver_floors(NOW, 1),
        Err(ApplicationSessionStateError::ReceiverResetMetadata)
    );
}

#[test]
fn portable_receiver_profile_requires_explicit_bounded_work_and_proof_policy() {
    assert!(profile().validate().is_ok());
    let mut invalid = profile();
    invalid.verification_burst = 0;
    assert!(invalid.validate().is_err());
    invalid = profile();
    invalid.enrollment_refresh_interval_ns = 1;
    assert!(invalid.validate().is_err());
    invalid = profile();
    invalid.max_proof_ttl_ns = 60_000_000_001;
    assert!(invalid.validate().is_err());
}

#[test]
fn portable_receiver_controlled_clock_never_enables_unqualified_production() {
    use canic_contracts::ids::BuildNetwork;
    assert!(runtime::require_qualified_clock(BuildNetwork::Local).is_ok());
    assert!(matches!(
        runtime::require_qualified_clock(BuildNetwork::Ic),
        Err(PortableSessionError::UnqualifiedClock)
    ));
    assert!(toml::from_str::<PortableReceiverConfig>("clock_skew_ns = 1").is_err());
}

#[test]
fn retained_receiver_operates_across_intent_cleanup_success_failure_and_reclamation() {
    use crate::{
        ids::{IntentId, IntentResourceKey},
        ops::storage::intent::IntentStoreOps,
        workflow::runtime::intent::IntentCleanupWorkflow,
    };
    let _lock = seams::lock();
    let _state = ApplicationSessionTestStateGuard::empty();
    let mut receiver = owner();
    let fixture = Fixture::new();
    let admitted = admit(&mut receiver, fixture.context(browser(), NOW), issue()).unwrap();
    IntentStoreOps::reset_for_tests();
    let resource = IntentResourceKey::new("application:receiver-cleanup");
    IntentStoreOps::try_reserve(IntentId(41), resource.clone(), 1, 10, Some(0), 10).unwrap();
    let cleaned = IntentCleanupWorkflow::run_due_batch();
    assert_eq!(
        cleaned.completion().outcome(),
        ic_timers::TimerCompletionOutcome::Success
    );
    assert_eq!(IntentStoreOps::pending_total().unwrap(), 0);
    assert_eq!(
        read(
            &mut receiver,
            fixture.context(browser(), NOW + 1),
            vec!["read".into()],
            false
        )
        .unwrap(),
        admitted
    );

    IntentStoreOps::try_reserve(IntentId(42), resource, 1, 10, Some(0), 10).unwrap();
    IntentStoreOps::clear_totals_for_tests();
    let failed = IntentCleanupWorkflow::run_due_batch();
    assert_eq!(
        failed.completion().outcome(),
        ic_timers::TimerCompletionOutcome::RetryableFailure
    );
    assert_eq!(
        read(
            &mut receiver,
            fixture.context(browser(), NOW + 1),
            vec!["read".into()],
            false
        )
        .unwrap(),
        admitted
    );

    let next = NOW + 6_000_000_000;
    assert!(receiver.cleanup_sessions(next).unwrap().total_removed() > 0);
    let next_login = admit(
        &mut receiver,
        fixture.context(caller(4), next),
        issue_at(next, 4),
    )
    .unwrap();
    assert_eq!(next_login.caller, caller(4));
    IntentStoreOps::reset_for_tests();
}

#[test]
fn receiver_capacity_configuration_defaults_and_round_trips() {
    let mut configured = profile();
    configured.max_sessions = 512;
    configured.validate().unwrap();
    let text = toml::to_string(&configured).unwrap();
    assert_eq!(
        toml::from_str::<PortableReceiverConfig>(&text).unwrap(),
        configured
    );
    let omitted = text
        .lines()
        .filter(|line| !line.starts_with("max_sessions ="))
        .collect::<Vec<_>>()
        .join("\n");
    let defaulted: PortableReceiverConfig = toml::from_str(&omitted).unwrap();
    assert_eq!(
        defaulted.max_sessions,
        crate::model::auth::application_authorization::capacity::DEFAULT_MAX_SESSIONS
    );
    defaulted.validate().unwrap();
    configured.max_sessions = 0;
    assert!(configured.validate().is_err());
    configured.max_sessions = u32::MAX;
    assert!(configured.validate().is_err());
}
