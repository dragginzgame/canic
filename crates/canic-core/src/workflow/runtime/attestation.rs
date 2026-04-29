use crate::{
    dto::{
        auth::AttestationKeySet,
        error::{Error, ErrorCode},
    },
    ops::{
        auth::AuthOps,
        rpc::RpcOps,
        runtime::{env::EnvOps, timer::TimerId},
    },
    protocol,
    workflow::{
        config::WORKFLOW_ATTESTATION_KEY_REFRESH_INTERVAL, prelude::*,
        runtime::timer::TimerWorkflow,
    },
};
use std::{cell::RefCell, time::Duration};

thread_local! {
    static TIMER: RefCell<Option<TimerId>> = const { RefCell::new(None) };
}

const REFRESH_INTERVAL: Duration = WORKFLOW_ATTESTATION_KEY_REFRESH_INTERVAL;

///
/// RoleAttestationKeyRefreshWorkflow
///

pub struct RoleAttestationKeyRefreshWorkflow;

impl RoleAttestationKeyRefreshWorkflow {
    // Start the periodic root attestation-key refresh loop for non-root canisters.
    pub fn start() {
        let _ = TimerWorkflow::set_guarded_interval(
            &TIMER,
            Duration::ZERO,
            "attestation_keys:init",
            || async {
                Self::refresh_once().await;
            },
            REFRESH_INTERVAL,
            "attestation_keys:interval",
            || async {
                Self::refresh_once().await;
            },
        );
    }

    // Refresh the locally cached root attestation key set once.
    async fn refresh_once() {
        if EnvOps::is_root() {
            return;
        }

        let root_pid = match EnvOps::root_pid() {
            Ok(pid) => pid,
            Err(err) => {
                log!(
                    Topic::Auth,
                    Warn,
                    "attestation key refresh skipped: root pid unavailable: {err}"
                );
                return;
            }
        };

        match RpcOps::call_rpc_result::<AttestationKeySet>(
            root_pid,
            protocol::CANIC_ATTESTATION_KEY_SET,
            (),
        )
        .await
        {
            Ok(key_set) => {
                let count = key_set.keys.len();
                AuthOps::replace_attestation_key_set(key_set);
                log!(
                    Topic::Auth,
                    Info,
                    "attestation key refresh ok: keys={count}"
                );
            }
            Err(err) => {
                if should_stop_refresh_loop(&err) {
                    let stopped = TimerWorkflow::clear_guarded(&TIMER);
                    log!(
                        Topic::Auth,
                        Info,
                        "attestation key refresh stopped: caller is no longer registered on the subnet registry (timer_cleared={stopped})"
                    );
                    return;
                }

                log!(Topic::Auth, Warn, "attestation key refresh failed: {err}");
            }
        }
    }
}

// Stop retrying when root explicitly denies this canister as no longer registered.
fn should_stop_refresh_loop(err: &crate::InternalError) -> bool {
    let public = err
        .public_error()
        .cloned()
        .unwrap_or_else(|| Error::from(err));

    public.code == ErrorCode::Unauthorized
        && public
            .message
            .contains("not registered on the subnet registry")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InternalError, InternalErrorOrigin, access::AccessError};

    #[test]
    fn stop_refresh_loop_on_unregistered_subnet_denial() {
        let err: InternalError = AccessError::Denied(
            "authentication error: caller 'aaaaa-aa' is not registered on the subnet registry"
                .to_string(),
        )
        .into();

        assert!(should_stop_refresh_loop(&err));
    }

    #[test]
    fn keep_refresh_loop_running_for_other_errors() {
        let err = InternalError::ops(InternalErrorOrigin::Ops, "transient failure");

        assert!(!should_stop_refresh_loop(&err));
    }
}
