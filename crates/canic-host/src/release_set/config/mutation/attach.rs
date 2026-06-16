use super::{
    AttachedFleetRoleSource,
    support::{
        toml_string_literal, validate_attach_kind, validate_role_name, validate_subnet_name,
    },
};
use crate::release_set::config::model::AttachedFleetRole;
use canic_core::{bootstrap::parse_config_model, ids::CanisterRole};

pub(in crate::release_set) fn attach_fleet_role_source(
    config_source: &str,
    expected_fleet: &str,
    role: &str,
    subnet: &str,
    kind: &str,
) -> Result<AttachedFleetRoleSource, Box<dyn std::error::Error>> {
    let role = role.trim();
    let subnet = subnet.trim();
    let kind = kind.trim();
    validate_role_name(role)?;
    validate_subnet_name(subnet)?;
    validate_attach_kind(kind)?;
    if role == "root" {
        return Err("root role must already be attached through root topology".into());
    }

    let config = parse_config_model(config_source).map_err(|err| err.to_string())?;
    let actual_fleet = config
        .fleet_name()
        .ok_or_else(|| "missing required [fleet].name in canic.toml".to_string())?;
    if actual_fleet != expected_fleet {
        return Err(format!(
            "selected config declares fleet {actual_fleet:?}, not {expected_fleet:?}"
        )
        .into());
    }

    let role_id = CanisterRole::owned(role.to_string());
    config
        .roles
        .get(&role_id)
        .ok_or_else(|| format!("role {expected_fleet}.{role} is not declared"))?;
    if config.attached_roles().contains(&role_id) {
        return Err(format!("role {expected_fleet}.{role} is already attached").into());
    }

    let mut source = config_source.trim_end().to_string();
    source.push_str("\n\n[subnets.");
    source.push_str(&toml_string_literal(subnet));
    source.push_str(".canisters.");
    source.push_str(&toml_string_literal(role));
    source.push_str("]\nkind = ");
    source.push_str(&toml_string_literal(kind));
    source.push('\n');

    parse_config_model(&source).map_err(|err| err.to_string())?;

    Ok(AttachedFleetRoleSource {
        source,
        role: AttachedFleetRole {
            fleet: expected_fleet.to_string(),
            role: role.to_string(),
            display: format!("{expected_fleet}.{role}"),
            subnet: subnet.to_string(),
            kind: kind.to_string(),
            topology: format!("{subnet}/{role}"),
        },
    })
}
