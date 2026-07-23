use super::{
    AttachedFleetRoleSource,
    support::{
        admit_canister_role_name, toml_string_literal, validate_attach_kind, validate_subnet_name,
    },
};
use crate::release_set::config::{
    AppConfigDeclaration, AppConfigError, AppConfigMutationConflict, AppConfigOperation,
    model::AttachedFleetRole,
};
use canic_core::{bootstrap::parse_config_model, ids::CanisterRole};

pub(in crate::release_set) fn attach_fleet_role_source(
    config_source: &str,
    expected_app: &str,
    role: &str,
    subnet: &str,
    kind: &str,
) -> Result<AttachedFleetRoleSource, AppConfigError> {
    let role = role.trim();
    let subnet = subnet.trim();
    let kind = kind.trim();
    admit_canister_role_name(role)?;
    validate_subnet_name(subnet)?;
    validate_attach_kind(kind)?;
    if role == "root" {
        return Err(AppConfigError::MutationConflict {
            conflict: AppConfigMutationConflict::RootRoleAttach,
        });
    }

    let config =
        parse_config_model(config_source).map_err(|source| AppConfigError::CoreConfig {
            operation: AppConfigOperation::AttachRole,
            source,
        })?;
    let actual_app = config.app_id().as_str();
    if actual_app != expected_app {
        return Err(AppConfigError::AppMismatch {
            actual: actual_app.to_string(),
            expected: expected_app.to_string(),
        });
    }

    let role_id = CanisterRole::owned(role.to_string());
    config
        .roles
        .get(&role_id)
        .ok_or_else(|| AppConfigError::DeclarationMissing {
            declaration: AppConfigDeclaration::Role {
                fleet: expected_app.to_string(),
                role: role.to_string(),
            },
        })?;
    if config.attached_roles().contains(&role_id) {
        return Err(AppConfigError::MutationConflict {
            conflict: AppConfigMutationConflict::RoleAlreadyAttached {
                fleet: expected_app.to_string(),
                role: role.to_string(),
            },
        });
    }

    let mut source = config_source.trim_end().to_string();
    source.push_str("\n\n[subnets.");
    source.push_str(&toml_string_literal(subnet));
    source.push_str(".canisters.");
    source.push_str(&toml_string_literal(role));
    source.push_str("]\nkind = ");
    source.push_str(&toml_string_literal(kind));
    source.push('\n');

    parse_config_model(&source).map_err(|source| AppConfigError::CoreConfig {
        operation: AppConfigOperation::AttachRole,
        source,
    })?;

    Ok(AttachedFleetRoleSource {
        source,
        role: AttachedFleetRole {
            fleet: expected_app.to_string(),
            role: role.to_string(),
            display: format!("{expected_app}.{role}"),
            subnet: subnet.to_string(),
            kind: kind.to_string(),
            topology: format!("{subnet}/{role}"),
        },
    })
}
