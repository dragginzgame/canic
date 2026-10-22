//! Mounted operator inspection reports local facts without authorizing provider payments.

mod funding;
mod provider;
pub(super) use provider::{cashier, qualify};

use super::tests::decode_blob;
use canic_blob_service::__private::ic_blob_storage_contracts as contracts;
use contracts::dto::{
    configuration::ServiceConfigurationInput,
    operator::{LocalServiceStatus, LocalStatusFailure, OperatorScope},
};
use ic_platform::{pocket_ic::PocketIc, principal::ExternalPrincipal as Principal};

fn query<T: serde::Serialize + serde::de::DeserializeOwned>(
    pic: &PocketIc,
    scope: OperatorScope,
    caller: Principal,
    method: &str,
    request: impl serde::Serialize,
) -> T {
    decode_blob(
        method,
        &pic.query_call(
            scope.service.into(),
            caller,
            method,
            contracts::wire::encode_request(method, request).unwrap(),
        )
        .expect("mounted operator query"),
    )
    .unwrap()
}

pub(super) fn check(pic: &PocketIc, config: &ServiceConfigurationInput, fenced: bool) {
    let scope = OperatorScope {
        service: config.service,
        namespace: config.namespace,
        cashier: config.billing.cashier,
        payment_account: config.payment_account,
    };
    let stable = pic.get_stable_memory(scope.service.into());
    let inspect = |actor, request| -> Result<LocalServiceStatus, LocalStatusFailure> {
        query(pic, scope, actor, "blob_local_status", request)
    };
    let status = inspect(config.operator.into(), scope).unwrap();
    assert_eq!(status.scope, scope);
    assert_eq!(status.uploads.fenced, fenced);
    assert_eq!(status.funding.fenced, fenced);
    assert_eq!(status.gateways.fenced, fenced);
    assert_eq!(status.reads.fenced, fenced);
    assert_eq!(
        status.funding.cumulative_allocation,
        config.funding.allocated
    );
    assert_eq!(
        status.funding.available_allocation,
        config.funding.allocated
    );
    assert_eq!(status.funding.retained_intents, 0);
    assert_eq!(status.funding.last_operation, None);
    assert_eq!(status.funding.transport_accepted, 0);
    assert_eq!(status.funding.uncredited_accepted, 0);
    assert_eq!(status.funding.reserved_or_uncertain, 0);
    assert_eq!(
        inspect(config.payment_account.into(), scope),
        Err(LocalStatusFailure::Denied)
    );
    for changed in wrong_scopes(scope) {
        assert_eq!(
            inspect(config.operator.into(), changed),
            Err(LocalStatusFailure::Binding)
        );
    }
    funding::check(pic, config.operator.into(), scope, status.funding);
    assert_eq!(inspect(config.operator.into(), scope).unwrap(), status);
    assert_eq!(pic.get_stable_memory(scope.service.into()), stable);
}

fn wrong_scopes(scope: OperatorScope) -> [OperatorScope; 4] {
    let other = Principal::from_slice(&[99]).into();
    [
        OperatorScope {
            service: other,
            ..scope
        },
        OperatorScope {
            namespace: scope.namespace + 1,
            ..scope
        },
        OperatorScope {
            cashier: other,
            ..scope
        },
        OperatorScope {
            payment_account: other,
            ..scope
        },
    ]
}
