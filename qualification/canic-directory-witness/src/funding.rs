//! Explicit endpoints for the opted-in payment participant.

use canic_management::funding;
use canic_management_protocol::stored;
pub(super) const KEY: &str = "hub.funding.v1";

// configure_child_funding
// Approve a finite budget for one directory child.
// guard  : controller
// limits : default Wire limits
#[canic::update(contract = stored::ConfigureChildFunding)]
async fn configure_child_funding(
    rule: stored::ChildFunding,
) -> Result<stored::ChildFundingStatus, stored::FundingError> {
    funding::configure(rule).await
}

// stop_child_funding
// Close the child's payment edge.
// guard  : controller
// limits : default Wire limits
#[canic::update(contract = stored::StopChildFunding)]
fn stop_child_funding(
    child: ic_platform_types::Principal,
) -> Result<stored::ChildFundingStatus, stored::FundingError> {
    funding::stop(child)
}

// watch_child_balance
// Schedule this child's own low-balance check.
// guard  : retained directory parent checked by watch
// limits : default Wire limits
#[canic::update(contract = funding::Watch)]
fn watch_child_balance(terms: (u64, u64)) -> Result<(), stored::FundingError> {
    funding::watch(terms)
}

// request_child_funding
// Pay or recover the current child's original request.
// guard  : exact configured direct child checked by request
// limits : default Wire limits
#[canic::update(contract = funding::Request)]
async fn request_child_funding(request: (u64, u128)) -> Result<u64, stored::FundingError> {
    funding::request(request).await
}

// fund_ledger
// Have this installed project pay its own Root for its singleton.
// guard  : installing Root (sole controller)
// limits : sharding::LIMITS
#[canic::update(guard = canic::guard::require_controller, limits = canic::sharding::LIMITS)]
async fn fund_ledger(
    terms: (u128, u128),
) -> Result<canic::sharding::payment::Payment, canic::sharding::payment::Refusal> {
    canic::sharding::payment::acquire_placement(
        ic_platform_runtime::system::caller(),
        canic::sharding::payment::Demand::Singleton("project_ledger".into()),
        terms.0,
        terms.1,
    )
    .await
    .map_err(|error| match error {
        canic::sharding::payment::AcquireError::Refused(reason) => reason,
        other => panic!("funded placement call failed: {other:?}"),
    })
}

// ledger
// Read the project's sole acknowledged ledger.
// guard  : public
// limits : default Wire limits
#[canic::query]
fn ledger() -> Result<ic_platform_types::Principal, canic::topology::SingletonLookupError> {
    canic::topology::acknowledged_singleton("project_ledger")
}
