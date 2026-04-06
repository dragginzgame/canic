use candid::Principal;
use canic::{Error, protocol};

use super::{Pic, startup};

impl Pic {
    /// Dump basic PocketIC status and log context for one canister.
    pub fn dump_canister_debug(&self, canister_id: Principal, context: &str) {
        eprintln!("{context}: debug for canister {canister_id}");

        match self.canister_status(canister_id, None) {
            Ok(status) => eprintln!("canister_status: {status:?}"),
            Err(err) => {
                let message = err.to_string();
                if startup::is_dead_instance_transport_error(&message) {
                    eprintln!("canister_status unavailable: PocketIC instance no longer reachable");
                    return;
                }
                eprintln!("canister_status failed: {err:?}");
            }
        }

        match self.fetch_canister_logs(canister_id, Principal::anonymous()) {
            Ok(records) => {
                if records.is_empty() {
                    eprintln!("canister logs: <empty>");
                } else {
                    for record in records {
                        eprintln!("canister log: {record:?}");
                    }
                }
            }
            Err(err) => {
                let message = err.to_string();
                if startup::is_dead_instance_transport_error(&message) {
                    eprintln!(
                        "fetch_canister_logs unavailable: PocketIC instance no longer reachable"
                    );
                    return;
                }
                eprintln!("fetch_canister_logs failed: {err:?}");
            }
        }
    }

    // Query `canic_ready` and panic with debug context on transport failures.
    pub(super) fn fetch_ready(&self, canister_id: Principal) -> bool {
        match self.query_call(canister_id, protocol::CANIC_READY, ()) {
            Ok(ready) => ready,
            Err(err) => {
                self.debug_or_panic_dead_instance(canister_id, "query canic_ready failed", &err)
            }
        }
    }

    // Fail fast once the PocketIC instance itself is gone.
    fn panic_dead_instance_transport(context: &str, err: &Error) -> ! {
        panic!("{context}: PocketIC instance no longer reachable: {err}");
    }

    // Emit local debug when possible, but avoid cascading debug failures on a dead instance.
    fn debug_or_panic_dead_instance(
        &self,
        canister_id: Principal,
        context: &str,
        err: &Error,
    ) -> ! {
        if startup::is_dead_instance_transport_error(&err.to_string()) {
            Self::panic_dead_instance_transport(context, err);
        }

        self.dump_canister_debug(canister_id, context);
        panic!("{context}: {err:?}");
    }
}
