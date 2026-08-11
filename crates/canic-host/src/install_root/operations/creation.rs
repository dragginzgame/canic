//! Module: install_root::operations::creation
//!
//! Responsibility: execute or observe one journal-authorized initial Canister creation effect.
//! Does not own: journal transitions, unknown-outcome policy, or role-specific authority.
//! Boundary: callers decide whether this process owns the paid effect and commit returned evidence.

use super::super::{
    commands::{
        icp_canister_create_command, open_creation_result_for_effect, read_created_canister,
    },
    install_icp,
};
use crate::{
    fleet_install_plan::PlannedCanisterCreationFunding,
    icp::{LocalReplicaTarget, run_output_to_file},
};
use candid::Principal;
use canic_core::ids::SubnetId;
use std::path::Path;

use super::{
    EffectAction,
    activation::{require_expected_controllers, require_uninstalled_created_canister},
};

pub(in crate::install_root) struct CreationEffectEvidence {
    pub canister: Option<Principal>,
    pub command_error: Option<String>,
}

pub(in crate::install_root) struct CreationEffectRequest<'a> {
    pub icp_executable: &'a str,
    pub icp_root: &'a Path,
    pub environment: &'a str,
    pub local_replica: Option<&'a LocalReplicaTarget>,
    pub result_path: &'a Path,
    pub subject: &'static str,
    pub placement_subnet: SubnetId,
    pub funding: &'a PlannedCanisterCreationFunding,
    pub controllers: &'a [Principal],
    pub action: EffectAction,
    pub expected_module_hash: [u8; 32],
}

pub(in crate::install_root) fn execute_or_observe_creation(
    request: CreationEffectRequest<'_>,
) -> Result<CreationEffectEvidence, Box<dyn std::error::Error>> {
    if request.controllers.is_empty() {
        return Err(format!(
            "{} creation authority must contain at least one explicit controller",
            request.subject
        )
        .into());
    }
    let mut command_error = None;
    if matches!(request.action, EffectAction::Execute) {
        let result = open_creation_result_for_effect(request.result_path, request.subject)?;
        let mut command = icp_canister_create_command(
            request.icp_executable,
            request.icp_root,
            request.environment,
            request.local_replica,
            request.placement_subnet,
            request.funding,
            request.controllers,
        );
        if let Err(error) = run_output_to_file(&mut command, &result) {
            command_error = Some(error.to_string());
        }
    }

    let canister = read_created_canister(request.result_path)?;
    if let Some(canister) = canister {
        let icp = install_icp(
            request.icp_executable,
            request.icp_root,
            request.environment,
            request.local_replica,
        );
        require_uninstalled_created_canister(
            &icp,
            canister,
            request.expected_module_hash,
            request.subject,
        )?;
        require_expected_controllers(&icp, canister, request.controllers, request.subject)?;
    }
    Ok(CreationEffectEvidence {
        canister,
        command_error,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::install_root::commands::prepare_creation_result;
    use std::fs;

    #[test]
    fn observe_only_reads_evidence_without_reissuing_creation() {
        let root = crate::test_support::temp_dir("canic-observe-creation-evidence");
        let result_path = root.join("created.json");
        prepare_creation_result(&result_path, "test Canister").expect("prepare result");
        let funding = PlannedCanisterCreationFunding::Cycles { cycles: 1 };

        let evidence = execute_or_observe_creation(CreationEffectRequest {
            icp_executable: "icp",
            icp_root: &root,
            environment: "local",
            local_replica: None,
            result_path: &result_path,
            subject: "test Canister",
            placement_subnet: SubnetId::from_principal(Principal::from_slice(&[42])),
            funding: &funding,
            controllers: &[Principal::from_slice(&[43])],
            action: EffectAction::ObserveOnly,
            expected_module_hash: [0; 32],
        })
        .expect("observe creation evidence");

        assert_eq!(evidence.canister, None);
        assert_eq!(evidence.command_error, None);
        fs::remove_dir_all(root).expect("remove temp root");
    }

    #[test]
    fn creation_rejects_ambient_controller_authority() {
        let root = crate::test_support::temp_dir("canic-creation-controller-authority");
        let funding = PlannedCanisterCreationFunding::Cycles { cycles: 1 };

        let result = execute_or_observe_creation(CreationEffectRequest {
            icp_executable: "icp",
            icp_root: &root,
            environment: "local",
            local_replica: None,
            result_path: &root.join("created.json"),
            subject: "test Canister",
            placement_subnet: SubnetId::from_principal(Principal::from_slice(&[42])),
            funding: &funding,
            controllers: &[],
            action: EffectAction::ObserveOnly,
            expected_module_hash: [0; 32],
        });
        let Err(error) = result else {
            panic!("ambient controller authority must fail closed");
        };

        assert!(
            error
                .to_string()
                .contains("must contain at least one explicit controller")
        );
    }
}
