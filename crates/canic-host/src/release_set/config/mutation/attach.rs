use super::{
    AttachedAppRoleSource,
    support::{
        admit_canister_role_name, toml_string_literal, validate_attach_kind,
        validate_component_spec_id,
    },
};
use crate::release_set::config::{
    AppConfigDeclaration, AppConfigError, AppConfigMutationConflict, AppConfigOperation,
    model::AttachedAppRole,
};
use canic_core::{bootstrap::parse_config_model, ids::CanisterRole};

pub(in crate::release_set) fn attach_app_role_source(
    config_source: &str,
    expected_app: &str,
    role: &str,
    component_spec: &str,
    kind: &str,
) -> Result<AttachedAppRoleSource, AppConfigError> {
    let role = role.trim();
    let component_spec = component_spec.trim();
    let kind = kind.trim();
    admit_canister_role_name(role)?;
    validate_component_spec_id(component_spec)?;
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
                app: expected_app.to_string(),
                role: role.to_string(),
            },
        })?;
    let target_component_spec = config.component_specs.get(component_spec);
    let role_is_component = config
        .component_specs
        .values()
        .any(|spec| spec.component_role == role_id);
    let role_already_in_target = target_component_spec
        .is_some_and(|spec| spec.component_role == role_id || spec.children.contains_key(&role_id));
    if role_already_in_target
        || role_is_component
        || (target_component_spec.is_none() && config.attached_roles().contains(&role_id))
    {
        return Err(AppConfigError::MutationConflict {
            conflict: AppConfigMutationConflict::RoleAlreadyAttached {
                app: expected_app.to_string(),
                role: role.to_string(),
            },
        });
    }

    let mut source = config_source.trim_end().to_string();
    source.push_str("\n\n[component_specs.");
    source.push_str(&toml_string_literal(component_spec));
    let (effective_kind, topology) = if let Some(component_spec_config) = target_component_spec {
        source.push_str(".children.");
        source.push_str(&toml_string_literal(role));
        source.push_str("]\nkind = ");
        source.push_str(&toml_string_literal(kind));
        source.push_str("\nmaximum_instances = 1\n");
        (
            kind.to_string(),
            format!(
                "{component_spec}/{}/children/{role}",
                component_spec_config.component_role
            ),
        )
    } else {
        source.push_str("]\ncomponent_role = ");
        source.push_str(&toml_string_literal(role));
        source.push_str("\nmaximum_instances = 1\n");
        ("component".to_string(), format!("{component_spec}/{role}"))
    };

    parse_config_model(&source).map_err(|source| AppConfigError::CoreConfig {
        operation: AppConfigOperation::AttachRole,
        source,
    })?;

    Ok(AttachedAppRoleSource {
        source,
        role: AttachedAppRole {
            app: expected_app.to_string(),
            role: role.to_string(),
            display: format!("{expected_app}.{role}"),
            component_spec: component_spec.to_string(),
            kind: effective_kind,
            topology,
        },
    })
}
