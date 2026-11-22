//! Explicit endpoints for the opted-in payment participant.

use canic_management::funding;
use canic_management_protocol::stored;
pub(super) const KEY: &str = "hub.funding.v1";

// configure_child_funding
// Approve a finite budget for one directory child.
// guard  : controller
// limits : default Wire limits
#[canic::update(contract = stored::ConfigureChildFunding, guard = canic::guard::require_controller)]
async fn configure_child_funding(
    rule: stored::ChildFunding,
) -> Result<stored::ChildFundingStatus, String> {
    funding::configure(rule).await
}

// stop_child_funding
// Close the child's payment edge.
// guard  : controller
// limits : default Wire limits
#[canic::update(contract = stored::StopChildFunding, guard = canic::guard::require_controller)]
fn stop_child_funding(
    child: ic_platform_types::Principal,
) -> Result<stored::ChildFundingStatus, String> {
    funding::stop(child)
}

// watch_child_balance
// Schedule this child's own low-balance check.
// guard  : retained directory parent checked by watch
// limits : default Wire limits
#[canic::update(contract = funding::Watch)]
fn watch_child_balance(terms: (u64, u64)) -> Result<(), String> {
    funding::watch(terms)
}

// request_child_funding
// Pay or recover the current child's original request.
// guard  : exact configured direct child checked by request
// limits : default Wire limits
#[canic::update(contract = funding::Request)]
async fn request_child_funding(request: (u64, u128)) -> Result<u64, String> {
    funding::request(request).await
}
