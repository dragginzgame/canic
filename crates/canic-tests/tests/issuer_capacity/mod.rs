//! Installed issuer capacity and local instruction cost through existing protected metrics.

use super::*;
use canic::dto::{
    metrics::{MetricValue, MetricsKind},
    portable_enrollment::{
        PortableIssuerCustody, PortableIssuerPreparedKey, PortableIssuerRegistrationRefusal,
        PreparePortableIssuerKey,
    },
    portable_issuance::{
        IssuePortableCredential, IssuedPortableCredential,
        PortableCredentialIssuanceRefusal as Refusal, PortableCredentialIssuanceStatus as Status,
    },
    role::MetricsStatusRequest,
};

// Maintained journal capacities; refusal at each boundary is observed through the public API.
const KEY_CAPACITY: u8 = 64;
const ISSUANCE_CAPACITY: u8 = 32;
const LOGIN_CAPACITY: u8 = 64;
// A regression budget for this fixed grant/profile, not production full-state qualification.
const MAX_PROFILE_INSTRUCTIONS: u64 = 100_000_000;
// Fresh admission also signs and performs several journal commits around its Root call.
const MAX_LAST_ADMISSION_INSTRUCTIONS: u64 = 300_000_000;

struct CompletedIssuance {
    request: IssuePortableCredential,
    issued: IssuedPortableCredential,
}

#[derive(Default)]
struct EndpointTotal {
    calls: u64,
    instructions: u128,
}

#[test]
fn full_issuer_history_refuses_capacity_without_losing_login_or_original_results() {
    let fixture = setup_fresh_active_component_registry();
    let prepared = fill_key_history(&fixture);
    register_volatile_key(&fixture, &prepared);
    let mut first_batch = fill_issuances(&fixture, 1, "last-admission-32-logins");
    let first = first_batch.remove(0);
    let removed = reclaim_after_retention(&fixture, "first-batch-pruning");
    assert_eq!(removed.removed_issuances, u32::from(ISSUANCE_CAPACITY));
    assert_eq!(removed.removed_logins, 0);

    let mut second_batch =
        fill_issuances(&fixture, ISSUANCE_CAPACITY + 1, "last-admission-64-logins");
    let latest = second_batch.pop().unwrap();
    verify_full_history(&fixture, &latest);
    let removed = reclaim_after_retention(&fixture, "full-login-ledger-pruning");
    assert_eq!(removed.removed_issuances, u32::from(ISSUANCE_CAPACITY));
    assert_eq!(removed.removed_logins, 0);
    verify_live_login_capacity(&fixture, &first);
    drop(fixture);
}

fn fill_key_history(fixture: &ActiveComponentRegistryFixture) -> PortableIssuerPreparedKey {
    let mut selected = None;
    for id in 1..=KEY_CAPACITY {
        selected = Some(prepare_volatile_key(fixture, &key_request(id)).unwrap());
    }
    let refused = measure(
        fixture,
        "key-capacity",
        "test_prepare_volatile_portable_issuer",
        || prepare_volatile_key(fixture, &key_request(KEY_CAPACITY + 1)),
    );
    assert_eq!(refused, Err(PortableIssuerRegistrationRefusal::Capacity));
    let selected = selected.unwrap();
    assert_eq!(
        prepare_volatile_key(fixture, &key_request(KEY_CAPACITY)),
        Ok(selected.clone())
    );
    selected
}

const fn key_request(id: u8) -> PreparePortableIssuerKey {
    PreparePortableIssuerKey {
        operation_id: [id; 32],
        custody: PortableIssuerCustody::TrustedVolatileLocal,
    }
}

fn fill_issuances(
    fixture: &ActiveComponentRegistryFixture,
    first: u8,
    label: &str,
) -> Vec<CompletedIssuance> {
    let base = portable_issue_request(fixture, 5);
    let mut completed = Vec::new();
    for id in first..first + ISSUANCE_CAPACITY {
        let mut request = base.clone();
        request.operation_id = [id; 32];
        request.login_id = [id; 32];
        let issue = || issue_fixture(fixture, request.clone(), Principal::from_slice(&[43]));
        let issued = if id == first + ISSUANCE_CAPACITY - 1 {
            measure_with_ceiling(
                fixture,
                label,
                "test_issue_portable_credential",
                MAX_LAST_ADMISSION_INSTRUCTIONS,
                issue,
            )
        } else {
            issue()
        }
        .unwrap();
        completed.push(CompletedIssuance { request, issued });
    }
    assert_eq!(issuer_context(fixture).epoch, base.epoch);
    completed
}

fn verify_full_history(fixture: &ActiveComponentRegistryFixture, latest: &CompletedIssuance) {
    let account = Principal::from_slice(&[43]);
    let retry = measure(
        fixture,
        "full-history-exact-retry",
        "test_issue_portable_credential",
        || issue_fixture(fixture, latest.request.clone(), account),
    );
    assert_eq!(retry, Ok(latest.issued.clone()));
    let lookup = measure(
        fixture,
        "full-history-lookup",
        "test_reconcile_portable_credential",
        || {
            reconcile_fixture(
                fixture,
                latest.request.epoch,
                latest.request.operation_id,
                account,
            )
        },
    );
    assert_eq!(lookup, Ok(Status::Issued(latest.issued.clone())));
    let mut overflow = latest.request.clone();
    overflow.operation_id = [LOGIN_CAPACITY + 1; 32];
    let refused = measure(
        fixture,
        "issuance-capacity",
        "test_issue_portable_credential",
        || issue_fixture(fixture, overflow, account),
    );
    assert_eq!(refused, Err(Refusal::Capacity));
    let unchanged = measure(
        fixture,
        "full-history-no-op-cleanup",
        "test_reclaim_portable_issuance",
        || issuer_retention::reclaim_as_root(fixture),
    );
    assert_eq!(unchanged.removed_issuances, 0);
    assert_eq!(unchanged.removed_logins, 0);
    assert_eq!(unchanged.epoch, latest.request.epoch);
    assert_eq!(
        reconcile_fixture(
            fixture,
            latest.request.epoch,
            latest.request.operation_id,
            account
        ),
        Ok(Status::Issued(latest.issued.clone()))
    );
}

fn reclaim_after_retention(
    fixture: &ActiveComponentRegistryFixture,
    label: &str,
) -> canic::dto::portable_issuance::PortableIssuanceMaintenance {
    fixture
        .pic()
        .advance_time(Duration::from_secs(71))
        .expect("PocketIC advance_time");
    fixture.pic().tick().expect("PocketIC tick");
    measure(fixture, label, "test_reclaim_portable_issuance", || {
        issuer_retention::reclaim_as_root(fixture)
    })
}

fn verify_live_login_capacity(fixture: &ActiveComponentRegistryFixture, first: &CompletedIssuance) {
    let account = Principal::from_slice(&[43]);
    let mut overflow = portable_issue_request(fixture, 5);
    overflow.operation_id = [99; 32];
    overflow.login_id = [99; 32];
    let refused = measure(
        fixture,
        "live-login-capacity",
        "test_issue_portable_credential",
        || issue_fixture(fixture, overflow, account),
    );
    assert_eq!(refused, Err(Refusal::Capacity));
    assert_eq!(
        issue_fixture(fixture, first.request.clone(), account),
        Err(Refusal::EpochClosed)
    );
    assert_eq!(
        reconcile_fixture(
            fixture,
            first.request.epoch,
            first.request.operation_id,
            account
        ),
        Ok(Status::Unresolved)
    );
    let mut renewal = first.request.clone();
    renewal.epoch = issuer_context(fixture).epoch;
    renewal.operation_id = [100; 32];
    let renewed = measure(
        fixture,
        "full-login-ledger-renewal",
        "test_issue_portable_credential",
        || issue_fixture(fixture, renewal, account),
    )
    .unwrap();
    let original = issuer_retention::claims(&first.issued);
    let claims = issuer_retention::claims(&renewed);
    assert_eq!(claims.login_started_at_ns, original.login_started_at_ns);
    assert_eq!(claims.login_deadline_ns, original.login_deadline_ns);
    issuer_retention::admit_credential(fixture, renewed, account);
}

fn measure<T>(
    fixture: &ActiveComponentRegistryFixture,
    label: &str,
    method: &str,
    action: impl FnOnce() -> T,
) -> T {
    measure_with_ceiling(fixture, label, method, MAX_PROFILE_INSTRUCTIONS, action)
}

/// Measure one endpoint call through protected metrics under its profile budget.
pub fn measure_with_ceiling<T>(
    fixture: &ActiveComponentRegistryFixture,
    label: &str,
    method: &str,
    ceiling: u64,
    action: impl FnOnce() -> T,
) -> T {
    let before = endpoint_total(fixture, method);
    let result = action();
    let after = endpoint_total(fixture, method);
    assert_eq!(after.calls.checked_sub(before.calls), Some(1));
    let instructions =
        u64::try_from(after.instructions.checked_sub(before.instructions).unwrap()).unwrap();
    eprintln!(
        "{}",
        serde_json::json!({
            "auth_issuer_cost": label, "method": method, "instructions": instructions,
            "ceiling": ceiling, "counter": 1,
        })
    );
    assert!((1..ceiling).contains(&instructions));
    result
}

fn endpoint_total(fixture: &ActiveComponentRegistryFixture, method: &str) -> EndpointTotal {
    use canic::protocol::protected_metrics as wire;
    let result: Result<canic::dto::page::Page<canic::dto::metric_page::MetricEntry>, Error> =
        ic_testkit::pic::query_wire(
            fixture.pic(),
            fixture.issuer.canister_id,
            fixture.root,
            portable_spec(
                wire::METHOD,
                wire::METHOD_ID,
                wire::REQUEST_LIMITS,
                wire::RESPONSE_LIMITS,
            ),
            &MetricsStatusRequest {
                kind: MetricsKind::Runtime,
                page: PageRequest {
                    limit: 512,
                    offset: 0,
                },
            },
        )
        .unwrap();
    let page = result.unwrap();
    assert_eq!(page.total, u64::try_from(page.entries.len()).unwrap());
    let rows = page
        .entries
        .iter()
        .filter(|row| row.labels == ["perf", "endpoint", "update", method])
        .collect::<Vec<_>>();
    assert!(rows.len() <= 1, "endpoint metrics identity must be unique");
    let Some(row) = rows.first() else {
        return EndpointTotal::default();
    };
    let MetricValue::Measurement {
        count,
        total,
        count_overflowed,
        total_overflowed,
    } = row.value
    else {
        panic!("endpoint performance must be a measurement");
    };
    assert!(!count_overflowed && !total_overflowed);
    EndpointTotal {
        calls: count,
        instructions: total,
    }
}
