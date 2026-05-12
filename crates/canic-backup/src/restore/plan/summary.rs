use super::{
    RestoreIdentitySummary, RestoreOperationSummary, RestorePlanMember, RestoreReadinessSummary,
    RestoreSnapshotSummary, RestoreVerificationSummary,
};
use crate::manifest::{FleetBackupManifest, IdentityMode};

pub(super) fn restore_identity_summary(
    members: &[RestorePlanMember],
    mapping_supplied: bool,
) -> RestoreIdentitySummary {
    let mut summary = RestoreIdentitySummary {
        mapping_supplied,
        all_sources_mapped: false,
        fixed_members: 0,
        relocatable_members: 0,
        in_place_members: 0,
        mapped_members: 0,
        remapped_members: 0,
    };

    for member in members {
        match member.identity_mode {
            IdentityMode::Fixed => summary.fixed_members += 1,
            IdentityMode::Relocatable => summary.relocatable_members += 1,
        }

        if member.source_canister == member.target_canister {
            summary.in_place_members += 1;
        } else {
            summary.remapped_members += 1;
        }
        if mapping_supplied {
            summary.mapped_members += 1;
        }
    }

    summary.all_sources_mapped = mapping_supplied && summary.mapped_members == members.len();

    summary
}

pub(super) fn restore_snapshot_summary(members: &[RestorePlanMember]) -> RestoreSnapshotSummary {
    let members_with_module_hash = members
        .iter()
        .filter(|member| member.source_snapshot.module_hash.is_some())
        .count();
    let members_with_code_version = members
        .iter()
        .filter(|member| member.source_snapshot.code_version.is_some())
        .count();
    let members_with_checksum = members
        .iter()
        .filter(|member| member.source_snapshot.checksum.is_some())
        .count();

    RestoreSnapshotSummary {
        all_members_have_module_hash: members_with_module_hash == members.len(),
        all_members_have_code_version: members_with_code_version == members.len(),
        all_members_have_checksum: members_with_checksum == members.len(),
        members_with_module_hash,
        members_with_code_version,
        members_with_checksum,
    }
}

pub(super) fn restore_readiness_summary(
    snapshot: &RestoreSnapshotSummary,
    verification: &RestoreVerificationSummary,
) -> RestoreReadinessSummary {
    let mut reasons = Vec::new();

    if !snapshot.all_members_have_checksum {
        reasons.push("missing-snapshot-checksum".to_string());
    }
    if !verification.all_members_have_checks {
        reasons.push("missing-verification-checks".to_string());
    }

    RestoreReadinessSummary {
        ready: reasons.is_empty(),
        reasons,
    }
}

pub(super) fn restore_verification_summary(
    manifest: &FleetBackupManifest,
    members: &[RestorePlanMember],
) -> RestoreVerificationSummary {
    let fleet_checks = manifest.verification.fleet_checks.len();
    let member_check_groups = manifest.verification.member_checks.len();
    let member_checks = members
        .iter()
        .map(|member| member.verification_checks.len())
        .sum::<usize>();
    let members_with_checks = members
        .iter()
        .filter(|member| !member.verification_checks.is_empty())
        .count();

    RestoreVerificationSummary {
        verification_required: true,
        all_members_have_checks: members_with_checks == members.len(),
        fleet_checks,
        member_check_groups,
        member_checks,
        members_with_checks,
        total_checks: fleet_checks + member_checks,
    }
}

pub(super) const fn restore_operation_summary(
    member_count: usize,
    verification_summary: &RestoreVerificationSummary,
) -> RestoreOperationSummary {
    RestoreOperationSummary {
        planned_snapshot_uploads: member_count,
        planned_snapshot_loads: member_count,
        planned_verification_checks: verification_summary.total_checks,
        planned_operations: member_count + member_count + verification_summary.total_checks,
    }
}
