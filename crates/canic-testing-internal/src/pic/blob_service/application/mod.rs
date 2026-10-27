//! Black-box contract checks for the embedded application's Wire counter.

use ic_platform::{pocket_ic::PocketIc, principal::ExternalPrincipal as Principal};
use ic_testkit::pic::{WireCallSpec, query_wire, update_wire};

// Pin the fixture's public ABI independently of its endpoint implementation.
const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 64,
    allocation: 256,
    nodes: 8,
    depth: 4,
};
const COUNT: WireCallSpec = WireCallSpec {
    method: "application_count",
    method_id: 0x454d_4244_0000_0001,
    request_limits: LIMITS,
    response_limits: LIMITS,
};
const INCREMENT: WireCallSpec = WireCallSpec {
    method: "application_increment",
    method_id: 0x454d_4244_0000_0002,
    request_limits: LIMITS,
    response_limits: LIMITS,
};

pub(super) fn count(pic: &PocketIc, service: Principal) -> u64 {
    query_wire(pic, service, Principal::anonymous(), COUNT, &()).unwrap()
}

pub(super) fn increment(pic: &PocketIc, service: Principal) -> u64 {
    let before = count(pic, service);
    let wrong = ic_wire::native::encode_request(COUNT.method_id, &(), LIMITS).unwrap();
    let mut trailing = ic_wire::native::encode_request(INCREMENT.method_id, &(), LIMITS).unwrap();
    trailing.push(0);
    for request in [wrong, trailing] {
        assert!(
            pic.update_call(service, Principal::anonymous(), INCREMENT.method, request)
                .is_err()
        );
        assert_eq!(count(pic, service), before);
    }
    update_wire(pic, service, Principal::anonymous(), INCREMENT, &()).unwrap()
}

#[test]
fn counter_limits_admit_unit_and_full_width_response() {
    for spec in [COUNT, INCREMENT] {
        let request = ic_wire::native::encode_request(spec.method_id, &(), LIMITS).unwrap();
        ic_wire::native::decode_request::<()>(spec.method_id, &request, LIMITS).unwrap();
        let reply = ic_wire::native::encode_response(spec.method_id, &u64::MAX, LIMITS).unwrap();
        assert_eq!(
            ic_wire::native::decode_response::<u64>(spec.method_id, &reply, LIMITS).unwrap(),
            u64::MAX,
        );
    }
}
