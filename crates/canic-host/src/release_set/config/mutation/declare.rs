use super::{
    DeclaredAppRoleSource,
    support::{admit_canister_role_name, toml_string_literal},
};
use crate::release_set::config::{
    AppConfigError, AppConfigMutationConflict, AppConfigNameField, AppConfigNameIssue,
    AppConfigOperation, model::DeclaredAppRole,
};
use canic_core::{bootstrap::parse_config_model, ids::CanisterRole};

pub(in crate::release_set) fn declare_app_role_source(
    config_source: &str,
    expected_app: &str,
    role: &str,
    package: &str,
) -> Result<DeclaredAppRoleSource, AppConfigError> {
    let role = role.trim();
    let package = package.trim();
    admit_canister_role_name(role)?;
    if package.is_empty() {
        return Err(AppConfigError::InvalidName {
            field: AppConfigNameField::Package,
            issue: AppConfigNameIssue::Empty,
            value: package.to_string(),
        });
    }
    if role == "root" {
        return Err(AppConfigError::MutationConflict {
            conflict: AppConfigMutationConflict::RootRoleDeclare,
        });
    }

    let config =
        parse_config_model(config_source).map_err(|source| AppConfigError::CoreConfig {
            operation: AppConfigOperation::DeclareRole,
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
    if config.declares_role(&role_id) {
        return Err(AppConfigError::MutationConflict {
            conflict: AppConfigMutationConflict::RoleAlreadyDeclared {
                app: expected_app.to_string(),
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

    parse_config_model(&source).map_err(|source| AppConfigError::CoreConfig {
        operation: AppConfigOperation::DeclareRole,
        source,
    })?;

    Ok(DeclaredAppRoleSource {
        source,
        role: DeclaredAppRole {
            app: expected_app.to_string(),
            role: role.to_string(),
            display: format!("{expected_app}.{role}"),
            package: package.to_string(),
        },
    })
}
