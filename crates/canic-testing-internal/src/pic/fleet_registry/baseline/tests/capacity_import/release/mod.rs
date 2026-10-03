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
    let read = |caller| -> Result<StatusResponse, Error> {
        pic.query_candid_as(
            root,
            caller,
            canic::protocol::CANIC_ROOT_STATUS,
            (StatusRequest::PoolRelease,),
        )
        .unwrap()
    };
    let StatusResponse::PoolRelease(observed) = read(controller).unwrap() else {
        panic!("expected pool release census");
    };
    assert_eq!(observed.root, root);
    assert_eq!(observed.capacity_import.as_ref(), Some(expected));
    let StatusResponse::PoolRelease(replayed) = read(controller).unwrap() else {
        panic!("expected pool release census replay");
    };
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
