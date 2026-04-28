use crate::{
    InternalError, InternalErrorOrigin,
    domain::policy::topology::TopologyPolicy,
    dto::auth::{DelegationProof, DelegationProvisionStatus},
    ops::{
        auth::audience,
        config::ConfigOps,
        ic::IcOps,
        storage::{
            auth::DelegationStateOps,
            index::{app::AppIndexOps, subnet::SubnetIndexOps},
            registry::subnet::SubnetRegistryOps,
        },
        topology::policy::mapper::RegistryPolicyInputMapper,
    },
    workflow::{
        auth::{DelegationPushOrigin, DelegationWorkflow},
        cascade::{state::StateCascadeWorkflow, topology::TopologyCascadeWorkflow},
        ic::provision::ProvisionWorkflow,
        prelude::*,
    },
};

///
/// PropagationWorkflow
///

pub struct PropagationWorkflow;

impl PropagationWorkflow {
    /// Propagate topology changes starting from the given canister.
    ///
    /// Used after structural mutations (create/adopt) to update
    /// parent/child relationships and derived topology views.
    pub async fn propagate_topology(target: Principal) -> Result<(), InternalError> {
        TopologyCascadeWorkflow::root_cascade_topology_for_pid(target).await
    }

    /// Propagate application/subnet state and index views after structural mutations.
    ///
    /// This rebuilds index snapshots from the registry, applies current
    /// app state, cascades it to root children, and finally re-asserts
    /// index ↔ registry consistency.
    pub async fn propagate_state(
        _target: Principal,
        role: &CanisterRole,
    ) -> Result<(), InternalError> {
        // The implicit wasm_store receives the normal topology cascade, but its
        // publication inventory is synchronized in root-owned subnet state after
        // creation rather than via the immediate create-time state cascade.
        if role.is_wasm_store() {
            return Ok(());
        }

        // Shared index/app-state changes are sibling-visible, so create/adopt
        // state propagation must refresh all root children, not only the target branch.
        let snapshot = ProvisionWorkflow::rebuild_indexes_from_registry(Some(role))?
            .with_app_state()
            .with_subnet_state()
            .build();

        StateCascadeWorkflow::root_cascade_state(&snapshot).await?;

        let registry_data = SubnetRegistryOps::data();
        let registry_input = RegistryPolicyInputMapper::record_to_policy_input(registry_data);
        let app_data = AppIndexOps::data();
        let subnet_data = SubnetIndexOps::data();

        TopologyPolicy::assert_index_consistent_with_registry(&registry_input, &app_data.entries)
            .map_err(InternalError::from)?;

        TopologyPolicy::assert_index_consistent_with_registry(
            &registry_input,
            &subnet_data.entries,
        )
        .map_err(InternalError::from)?;

        Self::propagate_delegation_proofs_to_new_verifier(_target, role).await?;

        Ok(())
    }

    // Push every unexpired root-cached signer proof that can authorize the new verifier role.
    async fn propagate_delegation_proofs_to_new_verifier(
        target: Principal,
        role: &CanisterRole,
    ) -> Result<(), InternalError> {
        let cfg = ConfigOps::delegated_tokens_config()?;
        if !cfg.enabled {
            return Ok(());
        }

        let canister_cfg = ConfigOps::current_subnet_canister(role)?;
        if !canister_cfg.delegated_auth.verifier {
            return Ok(());
        }

        let proofs = Self::unexpired_proofs_for_verifier_role(role);
        for proof in proofs {
            let response = DelegationWorkflow::push_verifier_targets(
                &proof,
                vec![target],
                DelegationPushOrigin::Provisioning,
            )
            .await;
            Self::ensure_delegation_proof_push_succeeded(target, &proof, &response)?;
        }

        Ok(())
    }

    // Select unexpired proofs whose audience covers the verifier role.
    fn unexpired_proofs_for_verifier_role(role: &CanisterRole) -> Vec<DelegationProof> {
        let now_secs = IcOps::now_secs();
        DelegationStateOps::unexpired_proofs_dto(now_secs)
            .into_iter()
            .filter(|proof| audience::role_allowed(role, &proof.cert.aud))
            .collect()
    }

    // Fail creation propagation if the new verifier did not receive one required proof.
    fn ensure_delegation_proof_push_succeeded(
        target: Principal,
        proof: &DelegationProof,
        response: &crate::dto::auth::DelegationVerifierProofPushResponse,
    ) -> Result<(), InternalError> {
        let Some(result) = response.results.iter().find(|entry| entry.target == target) else {
            return Err(InternalError::workflow(
                InternalErrorOrigin::Workflow,
                format!(
                    "delegation proof propagation missing target result target={target} shard={}",
                    proof.cert.shard_pid
                ),
            ));
        };

        if result.status == DelegationProvisionStatus::Ok {
            return Ok(());
        }

        let detail = result
            .error
            .as_ref()
            .map_or_else(|| "unknown error".to_string(), ToString::to_string);
        Err(InternalError::workflow(
            InternalErrorOrigin::Workflow,
            format!(
                "delegation proof propagation failed target={target} shard={} error={detail}",
                proof.cert.shard_pid
            ),
        ))
    }
}
