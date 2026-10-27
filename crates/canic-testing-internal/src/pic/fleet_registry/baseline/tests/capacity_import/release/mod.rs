//! Real Root pool census retains import progress without consuming calls, cycles or source state.

use super::*;

pub(super) fn assert_census(
    pic: &PocketIc,
    root: Principal,
    controller: Principal,
    expected: &PoolImportStatus,
) {
    let root_balance = pic.cycle_balance(root);
    let source_balances = expected
        .reservation
        .sources
        .iter()
        .map(|source| pic.cycle_balance(source.canister_id))
        .collect::<Vec<_>>();
    let read = |caller| -> Result<canic_contracts::dto::root::RootPoolReleaseResponse, Error> {
        let reply: Result<canic_contracts::dto::pool_census::Census, Error> =
            ic_testkit::pic::query_wire(
                pic,
                root,
                caller,
                ic_testkit::pic::WireCallSpec {
                    method: canic::protocol::pool_census::METHOD,
                    method_id: canic::protocol::pool_census::METHOD_ID,
                    request_limits: canic::protocol::pool_census::REQUEST_LIMITS,
                    response_limits: canic::protocol::pool_census::RESPONSE_LIMITS,
                },
                &(),
            )
            .expect("native pool release transport");
        reply.map(Into::into)
    };
    let observed = read(controller).unwrap();
    assert_eq!(observed.root, root);
    assert_eq!(observed.capacity_import.as_ref(), Some(expected));
    let replayed = read(controller).unwrap();
    assert_eq!(observed, replayed);
    let denied = read(Principal::self_authenticating(b"pool release outsider"))
        .err()
        .unwrap();
    assert_eq!(
        denied.code(),
        canic::diagnostics::codes::AUTHORITY_UNAVAILABLE.raw_code()
    );
    assert_eq!(pic.cycle_balance(root), root_balance);
    assert_eq!(
        expected
            .reservation
            .sources
            .iter()
            .map(|source| pic.cycle_balance(source.canister_id))
            .collect::<Vec<_>>(),
        source_balances
    );
}
