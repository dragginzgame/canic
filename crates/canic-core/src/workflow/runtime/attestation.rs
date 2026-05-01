use crate::{
    dto::auth::AttestationKeySet,
    ops::{
        auth::AuthOps,
        rpc::RpcOps,
        runtime::{env::EnvOps, timer::TimerId},
    },
    protocol,
    workflow::{prelude::*, runtime::timer::TimerWorkflow},
};
use std::{cell::RefCell, time::Duration};

thread_local! {
    static TIMER: RefCell<Option<TimerId>> = const { RefCell::new(None) };
}

///
/// RoleAttestationKeyRefreshWorkflow
///

pub struct RoleAttestationKeyRefreshWorkflow;

impl RoleAttestationKeyRefreshWorkflow {
    // Start one root attestation-key refresh for non-root canisters.
    pub fn start() {
        let _ =
            TimerWorkflow::set_guarded(&TIMER, Duration::ZERO, "attestation_keys:init", async {
                Self::refresh_once().await;
                let _ = TimerWorkflow::clear_guarded(&TIMER);
            });
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
                log!(Topic::Auth, Warn, "attestation key refresh failed: {err}");
            }
        }
    }
}
