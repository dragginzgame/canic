//! Convert disposable callback controls and observations at the application boundary.

use crate::protocol::{CallbackFault, CallbackProbe};
use canic::__internal::core::api::runtime::topup_fixture;

/// Convert authenticated application input to the internal fixture control.
pub fn configure(fault: CallbackFault) {
    let fault = match fault {
        CallbackFault::None => topup_fixture::CallbackFault::None,
        CallbackFault::BeforeAwait => topup_fixture::CallbackFault::BeforeAwait,
        CallbackFault::AfterAwait => topup_fixture::CallbackFault::AfterAwait,
        CallbackFault::HoldReply => topup_fixture::CallbackFault::HoldReply,
    };
    topup_fixture::configure_callback(fault);
}

/// Project the real business attempt without changing its timer or lease.
#[must_use]
pub fn probe() -> CallbackProbe {
    let observed = topup_fixture::callback_probe();
    CallbackProbe {
        entered: observed.entered,
        operation_id: observed.operation_id,
        lease_deadline_ns: observed.lease_deadline_ns,
        held: observed.held,
    }
}
