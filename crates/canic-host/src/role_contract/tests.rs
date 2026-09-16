use super::*;
use crate::release_set::AppConfigSnapshot;
use canic_core::{ids::CanisterRole, role_contract::RoleContractFinding};
use std::time::Instant;

#[test]
fn batched_contracts_preserve_isolated_roles_order_and_rejections() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../apps/test/test-configs/generated-mixed-topology.toml");
    let config = AppConfigSnapshot::load(&path).unwrap();
    let mut roles = config
        .model()
        .deployable_roles()
        .into_iter()
        .collect::<Vec<_>>();
    roles.push(CanisterRole::owned("undeclared".to_string()));
    roles.push(CanisterRole::ROOT);
    roles.push(roles[0].clone());

    let started = Instant::now();
    let isolated = roles
        .iter()
        .map(|role| {
            if role.is_root() {
                return resolve_canonical_root_contract(config.model());
            }
            match validate_declared_role_package(
                &path,
                config.model(),
                role,
                PackageValidationMode::Passive,
            ) {
                RolePackageValidation::Supported(evidence) => {
                    resolve_declared_role_package_contract(config.model(), &evidence)
                }
                RolePackageValidation::Unsupported(finding) => RoleContractResolution::Rejected {
                    errors: vec![finding],
                },
            }
        })
        .collect::<Vec<_>>();
    let isolated_elapsed = started.elapsed();
    let started = Instant::now();
    let batched = resolve_declared_role_contracts(
        &path,
        config.model(),
        &roles,
        PackageValidationMode::Passive,
    );
    eprintln!(
        "role contracts: isolated={isolated_elapsed:?}, batched={:?}",
        started.elapsed()
    );
    assert_eq!(batched, isolated);
    for (role, resolution) in roles.iter().zip(&batched) {
        if role.as_str() == "undeclared" {
            assert!(
                matches!(resolution, RoleContractResolution::Rejected { errors }
                if errors == &vec![RoleContractFinding::RoleUnknown { role: role.clone() }])
            );
        } else {
            assert!(
                matches!(resolution, RoleContractResolution::Resolved { contract }
                if &contract.role == role),
                "{role}: {resolution:?}"
            );
        }
    }
    assert!(resolve_declared_role_contracts(
        &path,
        config.model(),
        &[],
        PackageValidationMode::Passive,
    ).is_empty());
}
