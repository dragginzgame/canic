//! Module: workflow::runtime::auth::renewal
//!
//! Responsibility: schedule root-managed delegated-proof renewal sweeps.
//! Does not own: renewal policy, proof preparation internals, or issuer installs.

use crate::{
    InternalError, InternalErrorClass, InternalErrorOrigin,
    config::schema::DelegatedTokenConfig,
    domain::{auth::DelegatedAuthNetwork, runtime::FailureSeverity},
    ids::BuildNetwork,
    log,
    log::Topic,
    ops::{
        auth::{AuthOps, PrepareChainKeyRootDelegationBatchInput},
        config::ConfigOps,
        ic::IcOps,
        runtime::{
            env::EnvOps,
            metrics::delegated_auth::DelegatedAuthMetrics,
            recent_failure::{RecentFailureInput, RecentFailureOps},
            timer::TimerId,
        },
    },
    workflow::{
        config::{WORKFLOW_AUTH_RENEWAL_INTERVAL, WORKFLOW_INIT_DELAY},
        runtime::{
            auth::{provisioning, root_delegation_batch},
            timer::TimerWorkflow,
        },
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
        Self::start_if_configured_after(WORKFLOW_INIT_DELAY)
    }

    pub(super) fn start_soon_if_configured() -> Result<(), InternalError> {
        Self::start_if_configured_after(Duration::ZERO)
    }

    fn start_if_configured_after(init_delay: Duration) -> Result<(), InternalError> {
        if !EnvOps::is_root() {
            return Ok(());
        }
        if !AuthOps::has_enabled_root_issuer_renewal_templates() {
            return Ok(());
        }
        let config = ConfigOps::delegated_tokens_config()?;
        if !config.enabled {
            log!(
                Topic::Auth,
                Warn,
                "root delegated-proof renewal timer skipped: delegated-token auth is disabled"
            );
            return Ok(());
        }
        require_chain_key_root_proof_mode(&config)?;

        let _ = TimerWorkflow::set_guarded_interval(
            &RENEWAL_TIMER,
            init_delay,
            "auth_renewal:init",
            || async {
                Self::run_timer_sweep().await;
            },
            RENEWAL_INTERVAL,
            "auth_renewal:interval",
            || async {
                Self::run_timer_sweep().await;
            },
        );

        Ok(())
    }

    pub(super) async fn sweep() -> Result<bool, InternalError> {
        if !AuthOps::has_enabled_root_issuer_renewal_templates() {
            return Ok(false);
        }

        DelegatedAuthMetrics::record_renewal_sweep_started();
        let result = Self::sweep_configured().await;
        match &result {
            Ok(_) => DelegatedAuthMetrics::record_renewal_sweep_completed(),
            Err(_) => DelegatedAuthMetrics::record_renewal_sweep_failed(),
        }
        result
    }

    async fn sweep_configured() -> Result<bool, InternalError> {
        let config = ConfigOps::delegated_tokens_config()?;
        if !config.enabled {
            return Ok(false);
        }
        require_chain_key_root_proof_mode(&config)?;
        let build_network = build_network_from_delegated_auth_config(&config)?;
        let min_accepted_proof_epoch = chain_key_min_accepted_proof_epoch(&config)?;
        let max_cert_ttl_ns = delegated_token_max_ttl_ns()?;
        let now_ns = IcOps::now_nanos();
        let prepared = root_delegation_batch::prepare_due_chain_key_root_delegation_batch(
            PrepareChainKeyRootDelegationBatchInput {
                build_network,
                max_cert_ttl_ns,
                min_accepted_proof_epoch,
                required_issuer_pid: None,
                now_ns,
            },
        )?;
        let signed =
            AuthOps::sign_next_chain_key_root_delegation_batch(build_network, now_ns).await?;
        let installed = match AuthOps::start_next_chain_key_root_delegation_batch_install(now_ns)? {
            Some(request) => {
                provisioning::install_chain_key_delegation_proof_batch(request, now_ns)
                    .await
                    .installed_any
            }
            None => false,
        };

        if let Some(batch_id) = prepared.batch_id {
            log!(
                Topic::Auth,
                Info,
                "root chain-key delegated-proof renewal prepared batch_id={:?} issuers={} skipped={}",
                batch_id,
                prepared.prepared_issuers,
                prepared.skipped_templates
            );
        }

        Ok(prepared.batch_id.is_some() || signed.signed || signed.reused_signed || installed)
    }

    async fn run_timer_sweep() {
        if let Err(err) = Self::sweep().await {
            Self::record_timer_failure(&err);
        }
    }

    fn record_timer_failure(err: &InternalError) {
        let (class, origin) = err.log_fields();
        RecentFailureOps::record(RecentFailureInput {
            occurred_at_ns: IcOps::now_nanos(),
            subsystem: "auth_renewal".to_string(),
            code: renewal_failure_code(class, origin),
            severity: FailureSeverity::Error,
            summary: format!("class={class} origin={origin}: {err}"),
            correlation_id: None,
        });
        log!(
            Topic::Auth,
            Warn,
            "root delegated-proof renewal sweep failed class={class} origin={origin}: {err}"
        );
    }
}

fn renewal_failure_code(class: InternalErrorClass, origin: InternalErrorOrigin) -> String {
    format!(
        "renewal_sweep_failed/{}/{}",
        internal_error_class_code(class),
        internal_error_origin_code(origin)
    )
}

const fn internal_error_class_code(class: InternalErrorClass) -> &'static str {
    match class {
        InternalErrorClass::Access => "access",
        InternalErrorClass::Domain => "domain",
        InternalErrorClass::Infra => "infra",
        InternalErrorClass::Ops => "ops",
        InternalErrorClass::Workflow => "workflow",
        InternalErrorClass::Invariant => "invariant",
    }
}

const fn internal_error_origin_code(origin: InternalErrorOrigin) -> &'static str {
    match origin {
        InternalErrorOrigin::Access => "access",
        InternalErrorOrigin::Config => "config",
        InternalErrorOrigin::Domain => "domain",
        InternalErrorOrigin::Infra => "infra",
        InternalErrorOrigin::Ops => "ops",
        InternalErrorOrigin::Storage => "storage",
        InternalErrorOrigin::Workflow => "workflow",
    }
}

fn require_chain_key_root_proof_mode(config: &DelegatedTokenConfig) -> Result<(), InternalError> {
    if config.root_proof_mode.trim() == "chain_key_batch" {
        return Ok(());
    }
    Err(InternalError::invariant(
        InternalErrorOrigin::Workflow,
        "delegated-auth renewal requires root_proof_mode=\"chain_key_batch\"",
    ))
}

fn build_network_from_delegated_auth_config(
    config: &DelegatedTokenConfig,
) -> Result<BuildNetwork, InternalError> {
    let network = DelegatedAuthNetwork::parse(config.network.trim()).ok_or_else(|| {
        InternalError::invalid_input(
            "auth.delegated_tokens.network must be one of mainnet, local, pocketic, testnet",
        )
    })?;
    if network.is_mainnet() {
        Ok(BuildNetwork::Ic)
    } else {
        Ok(BuildNetwork::Local)
    }
}

fn chain_key_min_accepted_proof_epoch(config: &DelegatedTokenConfig) -> Result<u64, InternalError> {
    config
        .chain_key_root_proof
        .min_accepted_proof_epoch
        .ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Workflow,
                "auth.delegated_tokens.chain_key_root_proof.min_accepted_proof_epoch is required for chain-key renewal",
            )
        })
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

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::seams;

    #[test]
    fn timer_failure_preserves_typed_classification_in_recent_diagnostics() {
        let _guard = seams::lock();
        RecentFailureOps::reset();
        let err = InternalError::invariant(InternalErrorOrigin::Workflow, "renewal invariant");

        RootDelegationRenewalWorkflow::record_timer_failure(&err);

        let failures = RecentFailureOps::snapshot();
        assert_eq!(err.class(), InternalErrorClass::Invariant);
        assert_eq!(err.origin(), InternalErrorOrigin::Workflow);
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].subsystem, "auth_renewal");
        assert_eq!(failures[0].code, "renewal_sweep_failed/invariant/workflow");
        assert_eq!(failures[0].severity, FailureSeverity::Error);
        RecentFailureOps::reset();
    }
}
