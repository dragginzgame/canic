use super::{
    DeclaredFleetRoleSource,
    support::{admit_canister_role_name, toml_string_literal},
};
use crate::release_set::config::{
    FleetConfigDeclaration, FleetConfigError, FleetConfigMutationConflict, FleetConfigNameField,
    FleetConfigNameIssue, FleetConfigOperation, model::DeclaredFleetRole,
};
use canic_core::{bootstrap::parse_config_model, ids::CanisterRole};

pub(in crate::release_set) fn declare_fleet_role_source(
    config_source: &str,
    expected_fleet: &str,
    role: &str,
    package: &str,
) -> Result<DeclaredFleetRoleSource, FleetConfigError> {
    let role = role.trim();
    let package = package.trim();
    admit_canister_role_name(role)?;
    if package.is_empty() {
        return Err(FleetConfigError::InvalidName {
            field: FleetConfigNameField::Package,
            issue: FleetConfigNameIssue::Empty,
            value: package.to_string(),
        });
    }
    if role == "root" {
        return Err(FleetConfigError::MutationConflict {
            conflict: FleetConfigMutationConflict::RootRoleDeclare,
        });
    }

    let config =
        parse_config_model(config_source).map_err(|source| FleetConfigError::CoreConfig {
            operation: FleetConfigOperation::DeclareRole,
            source,
        })?;
    let actual_fleet = config
        .fleet_name()
        .ok_or(FleetConfigError::DeclarationMissing {
            declaration: FleetConfigDeclaration::FleetName,
        })?;
    if actual_fleet != expected_fleet {
        return Err(FleetConfigError::FleetMismatch {
            actual: actual_fleet.to_string(),
            expected: expected_fleet.to_string(),
        });
    }

    let role_id = CanisterRole::owned(role.to_string());
    if config.declares_role(&role_id) {
        return Err(FleetConfigError::MutationConflict {
            conflict: FleetConfigMutationConflict::RoleAlreadyDeclared {
                fleet: expected_fleet.to_string(),
                role: role.to_string(),
            },
        });
    }

    let mut source = config_source.trim_end().to_string();
    source.push_str("\n\n[roles.");
    source.push_str(&toml_string_literal(role));
    source.push_str("]\nkind = \"canister\"\npackage = ");
    source.push_str(&toml_string_literal(package));
    source.push('\n');

    parse_config_model(&source).map_err(|source| FleetConfigError::CoreConfig {
        operation: FleetConfigOperation::DeclareRole,
        source,
    })?;

    Ok(DeclaredFleetRoleSource {
        source,
        role: DeclaredFleetRole {
            fleet: expected_fleet.to_string(),
            role: role.to_string(),
            display: format!("{expected_fleet}.{role}"),
            package: package.to_string(),
        },
    })
}
