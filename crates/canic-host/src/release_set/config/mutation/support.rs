pub(super) fn validate_role_name(role: &str) -> Result<(), Box<dyn std::error::Error>> {
    if role.is_empty() {
        return Err("role must not be empty".into());
    }
    if !role
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    {
        return Err("role must contain only ASCII letters, numbers, '_' or '-'".into());
    }
    Ok(())
}

pub(super) fn validate_subnet_name(subnet: &str) -> Result<(), Box<dyn std::error::Error>> {
    if subnet.is_empty() {
        return Err("subnet must not be empty".into());
    }
    if !subnet
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    {
        return Err("subnet must contain only ASCII letters, numbers, '_' or '-'".into());
    }
    Ok(())
}

pub(super) fn validate_attach_kind(kind: &str) -> Result<(), Box<dyn std::error::Error>> {
    if matches!(
        kind,
        "service" | "singleton" | "shard" | "replica" | "instance"
    ) {
        return Ok(());
    }

    Err("kind must be one of: service, singleton, shard, replica, instance".into())
}

pub(super) fn toml_assignment_key(line: &str) -> Option<&str> {
    let (key, _) = line.split_once('=')?;
    Some(key.trim())
}

pub(super) fn toml_string_literal(value: &str) -> String {
    let mut escaped = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            ch => escaped.push(ch),
        }
    }
    escaped.push('"');
    escaped
}
