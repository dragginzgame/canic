//! Module: workflow::runtime::auth::renewal
//!
//! Responsibility: schedule root-managed delegated-proof renewal sweeps.
//! Does not own: renewal policy, proof preparation internals, or issuer installs.

use crate::{
    InternalError,
    ops::{
        auth::AuthOps,
        config::ConfigOps,
        ic::IcOps,
        runtime::{env::EnvOps, metrics::delegated_auth::DelegatedAuthMetrics, timer::TimerId},
    },
    workflow::{
        config::{WORKFLOW_AUTH_RENEWAL_INTERVAL, WORKFLOW_INIT_DELAY},
        prelude::*,
        runtime::timer::TimerWorkflow,
    },
};
use std::{cell::RefCell, time::Duration};

thread_local! {
    static RENEWAL_TIMER: RefCell<Option<TimerId>> = const { RefCell::new(None) };
}

const DEFAULT_DELEGATED_TOKEN_MAX_TTL_SECS: u64 = 24 * 60 * 60;
const RENEWAL_INTERVAL: Duration = WORKFLOW_AUTH_RENEWAL_INTERVAL;

pub(super) struct RootDelegationRenewalWorkflow;

impl RootDelegationRenewalWorkflow {
    pub(super) fn start_if_configured() -> Result<(), InternalError> {
        if !EnvOps::is_root() {
            return Ok(());
        }
        if !AuthOps::has_enabled_root_issuer_renewal_templates() {
            return Ok(());
        }
        if !ConfigOps::delegated_tokens_config()?.enabled {
            log!(
                Topic::Auth,
                Warn,
                "root delegated-proof renewal timer skipped: delegated-token auth is disabled"
            );
            return Ok(());
        }

        let _ = TimerWorkflow::set_guarded_interval(
            &RENEWAL_TIMER,
            WORKFLOW_INIT_DELAY,
            "auth_renewal:init",
            || async {
                let _ = Self::sweep();
            },
            RENEWAL_INTERVAL,
            "auth_renewal:interval",
            || async {
                let _ = Self::sweep();
            },
        );

        Ok(())
    }

    pub(super) fn sweep() -> Result<bool, InternalError> {
        if !AuthOps::has_enabled_root_issuer_renewal_templates() {
            return Ok(false);
        }

        let max_cert_ttl_ns = delegated_token_max_ttl_ns()?;
        let now_ns = IcOps::now_nanos();
        DelegatedAuthMetrics::record_renewal_sweep_started();
        let result = match AuthOps::prepare_due_delegation_renewals(max_cert_ttl_ns, now_ns) {
            Ok(result) => {
                DelegatedAuthMetrics::record_renewal_sweep_completed();
                result
            }
            Err(err) => {
                DelegatedAuthMetrics::record_renewal_sweep_failed();
                return Err(err);
            }
        };

        if let Some(batch_id) = result.prepared_batch_id {
            log!(
                Topic::Auth,
                Info,
                "root delegated-proof renewal prepared batch_id={:?} attempts={} skipped={}",
                batch_id,
                result.prepared_attempts,
                result.skipped_templates
            );
            return Ok(true);
        }

        Ok(false)
    }
}

fn delegated_token_max_ttl_ns() -> Result<u64, InternalError> {
    let cfg = ConfigOps::delegated_tokens_config()?;
    let max_ttl_secs = cfg
        .max_ttl_secs
        .unwrap_or(DEFAULT_DELEGATED_TOKEN_MAX_TTL_SECS);
    max_ttl_secs.checked_mul(1_000_000_000).ok_or_else(|| {
        InternalError::invalid_input("auth.delegated_tokens.max_ttl_secs overflows nanoseconds")
    })
}
