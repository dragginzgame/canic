//! Installed acceptance of all six outer Root observation frames and admission boundaries.
use super::*;
use canic::dto::{self, root_observation::Request};
use ic_testkit::pic::{WireCallSpec, update_wire};
use serde::{Serialize, de::DeserializeOwned};

pub(super) fn qualify(fixture: &ActiveComponentRegistryFixture) {
    let target = fixture.issuer.canister_id;
    macro_rules! read {
        ($method:ident, $input:expr, $reply:ty, $optional:expr) => {{
            use canic::protocol::root_observation::$method as protocol;
            qualify_method::<_, $reply>(
                fixture,
                target,
                $input,
                WireCallSpec {
                    method: protocol::METHOD,
                    method_id: protocol::METHOD_ID,
                    request_limits: protocol::REQUEST_LIMITS,
                    response_limits: protocol::RESPONSE_LIMITS,
                },
                $optional,
            )
        }};
    }
    let child = fixture.verifier.canister_id;
    let usage = read!(
        child_funding,
        ic_platform::Principal::from(child),
        dto::child_funding::ChildFundingUsage,
        false
    )
    .unwrap();
    assert_eq!(Principal::from(usage.parent), target);
    assert_eq!(Principal::from(usage.child), child);
    let balance = read!(
        cycle_balance,
        (),
        dto::role::CycleBalanceStatusResponse,
        false
    )
    .unwrap();
    assert!(balance.cycles > 0);
    let page = dto::page::PageRequest {
        offset: 0,
        limit: 1,
    };
    read!(
        cycle_history,
        page,
        dto::page::Page<dto::cycles::CycleTrackerEntry>,
        false
    );
    read!(
        cycle_topups,
        page,
        dto::page::Page<dto::cycle_topups::CycleTopupEvent>,
        true
    );
    read!(
        memory_allocations,
        (),
        dto::memory::MemoryAllocationsResponse,
        false
    );
    read!(
        metrics,
        dto::role::MetricsStatusRequest {
            kind: dto::metrics::MetricsKind::Security,
            page
        },
        dto::page::Page<dto::metric_page::MetricEntry>,
        true
    );
}

fn qualify_method<I: Serialize, O: DeserializeOwned>(
    fixture: &ActiveComponentRegistryFixture,
    target: Principal,
    input: I,
    spec: WireCallSpec,
    optional: bool,
) -> Option<O> {
    let read = |caller, target: Principal| -> Result<O, Error> {
        update_wire(
            fixture.pic(),
            fixture.root,
            caller,
            spec,
            &Request {
                canister_id: target.into(),
                input: &input,
            },
        )
        .expect("native Root relay transport")
    };
    let outsider = Principal::from_slice(&[0x7e; 29]);
    assert!(controller_authority_unavailable(&read(outsider, target)));
    for invalid in [
        fixture.root,
        Principal::anonymous(),
        Principal::management_canister(),
    ] {
        assert!(
            matches!(read(Principal::anonymous(), invalid), Err(error) if error.code() == canic::diagnostics::codes::REQUEST_INVALID.raw_code())
        );
    }
    match read(Principal::anonymous(), target) {
        Ok(value) => Some(value),
        Err(error)
            if optional
                && error.code() == canic::diagnostics::codes::REQUEST_INVALID.raw_code() =>
        {
            None
        }
        Err(error) => panic!("{} rejected its controller: {error:?}", spec.method),
    }
}
