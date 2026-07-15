//! Module: workflow::runtime::auth::root_delegation_batch
//!
//! Responsibility: apply issuer policy before committing root delegation batches.
//! Does not own: due-template selection, proof construction, signing, or persistence.

use crate::{
    InternalError,
    domain::policy::pure::auth::{
        RootDelegationProofPreparePolicyInput, validate_root_delegation_proof_prepare_policy,
    },
    ops::auth::{
        AuthOps, ChainKeyRootDelegationBatchPreparation, ChainKeyRootDelegationBatchSweepResult,
        ChainKeyRootDelegationIssuerApproval, PrepareChainKeyRootDelegationBatchInput,
    },
};

pub(super) fn prepare_due_chain_key_root_delegation_batch(
    input: PrepareChainKeyRootDelegationBatchInput,
) -> Result<ChainKeyRootDelegationBatchSweepResult, InternalError> {
    match AuthOps::plan_due_chain_key_root_delegation_batch(input)? {
        ChainKeyRootDelegationBatchPreparation::Complete(result) => {
            Ok(ChainKeyRootDelegationBatchSweepResult {
                batch_id: result.batch_id,
                prepared_issuers: result.prepared_issuers,
                skipped_templates: result.skipped_templates,
                reused_in_flight: result.reused_in_flight,
            })
        }
        ChainKeyRootDelegationBatchPreparation::RequiresPolicy(plan) => {
            let approvals = plan
                .issuer_templates()
                .map(|template| {
                    let decision = validate_root_delegation_proof_prepare_policy(
                        AuthOps::root_issuer_policy(template.issuer_pid).as_ref(),
                        RootDelegationProofPreparePolicyInput {
                            issuer_pid: template.issuer_pid,
                            audience: &template.audience,
                            grants: &template.grants,
                            cert_ttl_ns: plan.cert_ttl_ns(),
                            issued_at_ns: plan.issued_at_ns(),
                        },
                    )
                    .map_err(|err| InternalError::forbidden(err.to_string()))?;

                    Ok(ChainKeyRootDelegationIssuerApproval {
                        issuer_pid: template.issuer_pid,
                        expires_at_ns: decision.expires_at_ns,
                        refresh_after_ns: decision.refresh_after_ns,
                    })
                })
                .collect::<Result<Vec<_>, InternalError>>()?;

            AuthOps::commit_chain_key_root_delegation_batch(plan, approvals)
        }
    }
}
