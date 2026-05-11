use crate::{
    args::{
        default_icp, flag_arg, internal_icp_arg, internal_network_arg, local_network,
        parse_matches, print_help_or_version, value_arg,
    },
    version_text,
};
use candid::{
    TypeEnv,
    types::{FuncMode, Function, Label, Type, TypeInner},
};
use candid_parser::utils::CandidSource;
use canic_backup::discovery::{RegistryEntry, parse_registry_entries};
use canic_host::{
    icp::IcpCli, install_root::read_named_fleet_install_state, release_set::icp_root, replica_query,
};
use clap::Command as ClapCommand;
use serde::Serialize;
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

const CANDID_SERVICE_METADATA: &str = "candid:service";
const HELP_AFTER: &str = "\
Examples:
  canic endpoints test app
  canic endpoints test scale_hub --json
  canic endpoints test tl4x7-vh777-77776-aaacq-cai";

///
/// EndpointsCommandError
///

#[derive(Debug, ThisError)]
pub enum EndpointsCommandError {
    #[error("{0}")]
    Usage(String),

    #[error("canister interface did not contain a service block")]
    MissingService,

    #[error("failed to parse Candid interface: {0}")]
    InvalidCandid(String),

    #[error(
        "live metadata was unavailable for {canister} in fleet {fleet} and no local Candid artifact could be resolved"
    )]
    NoInterfaceArtifact { fleet: String, canister: String },

    #[error("local Candid artifact not found for role {role}; looked under {root}")]
    MissingRoleArtifact { role: String, root: String },

    #[error("failed to read local Candid artifact {path}: {source}")]
    ReadDid {
        path: String,
        source: std::io::Error,
    },

    #[error("failed to render endpoint output: {0}")]
    Json(#[from] serde_json::Error),
}

///
/// EndpointsOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct EndpointsOptions {
    fleet: String,
    canister: String,
    network: Option<String>,
    icp: String,
    json: bool,
}

impl EndpointsOptions {
    fn parse<I>(args: I) -> Result<Self, EndpointsCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(command(), args).map_err(|_| EndpointsCommandError::Usage(usage()))?;
        Ok(Self {
            fleet: string_value(&matches, "fleet").expect("clap requires fleet"),
            canister: string_value(&matches, "canister").expect("clap requires canister"),
            network: string_value(&matches, "network"),
            icp: string_value(&matches, "icp").unwrap_or_else(default_icp),
            json: matches.get_flag("json"),
        })
    }
}

///
/// EndpointReport
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct EndpointReport {
    source: String,
    endpoints: Vec<EndpointEntry>,
}

///
/// EndpointEntry
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct EndpointEntry {
    name: String,
    candid: String,
    modes: Vec<EndpointMode>,
    arguments: Vec<EndpointType>,
    returns: Vec<EndpointType>,
}

impl EndpointEntry {
    fn render(&self) -> String {
        self.candid.clone()
    }
}

///
/// EndpointMode
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum EndpointMode {
    Update,
    Query,
    CompositeQuery,
    Oneway,
}

///
/// EndpointCardinality
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum EndpointCardinality {
    Single,
    Optional,
    Many,
}

///
/// EndpointType
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum EndpointType {
    Primitive {
        candid: String,
        cardinality: EndpointCardinality,
        name: String,
    },
    Named {
        candid: String,
        cardinality: EndpointCardinality,
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        resolved: Option<Box<Self>>,
    },
    Optional {
        candid: String,
        cardinality: EndpointCardinality,
        inner: Box<Self>,
    },
    Vector {
        candid: String,
        cardinality: EndpointCardinality,
        inner: Box<Self>,
    },
    Record {
        candid: String,
        cardinality: EndpointCardinality,
        fields: Vec<EndpointField>,
    },
    Variant {
        candid: String,
        cardinality: EndpointCardinality,
        cases: Vec<EndpointField>,
    },
    Function {
        candid: String,
        cardinality: EndpointCardinality,
        modes: Vec<EndpointMode>,
        arguments: Vec<Self>,
        returns: Vec<Self>,
    },
    Service {
        candid: String,
        cardinality: EndpointCardinality,
        methods: Vec<EndpointServiceMethod>,
    },
    Class {
        candid: String,
        cardinality: EndpointCardinality,
        initializers: Vec<Self>,
        service: Box<Self>,
    },
}

///
/// EndpointField
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct EndpointField {
    label: String,
    id: u32,
    ty: EndpointType,
}

///
/// EndpointServiceMethod
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct EndpointServiceMethod {
    name: String,
    ty: EndpointType,
}

///
/// EndpointTarget
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct EndpointTarget {
    canister: String,
    role: Option<String>,
}

/// Run the canister endpoint listing command.
pub fn run<I>(args: I) -> Result<(), EndpointsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let options = EndpointsOptions::parse(args)?;
    let report = endpoint_report(&options)?;
    if options.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!(
            "{}",
            report
                .endpoints
                .iter()
                .map(EndpointEntry::render)
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
    Ok(())
}

fn endpoint_report(options: &EndpointsOptions) -> Result<EndpointReport, EndpointsCommandError> {
    let target = resolve_endpoint_target(options);
    if let Ok(target) = &target
        && let Ok(candid) = read_live_candid(options, target)
    {
        return Ok(EndpointReport {
            source: format!("{} metadata", options.canister),
            endpoints: parse_candid_service_endpoints(&candid)?,
        });
    }

    let role = target
        .ok()
        .and_then(|target| target.role)
        .or_else(|| (!is_principal_like(&options.canister)).then(|| options.canister.clone()));
    let Some(role) = role else {
        return Err(EndpointsCommandError::NoInterfaceArtifact {
            fleet: options.fleet.clone(),
            canister: options.canister.clone(),
        });
    };
    let path = resolve_role_did(options, &role)?;
    let candid = read_did(&path)?;
    Ok(EndpointReport {
        source: path.display().to_string(),
        endpoints: parse_candid_service_endpoints(&candid)?,
    })
}

fn read_live_candid(
    options: &EndpointsOptions,
    target: &EndpointTarget,
) -> Result<String, Box<dyn std::error::Error>> {
    Ok(IcpCli::new(&options.icp, None, options.network.clone())
        .canister_metadata_output(&target.canister, CANDID_SERVICE_METADATA)?)
}

fn resolve_endpoint_target(
    options: &EndpointsOptions,
) -> Result<EndpointTarget, Box<dyn std::error::Error>> {
    if is_principal_like(&options.canister) {
        let role = load_fleet_registry(options).ok().and_then(|registry| {
            registry
                .into_iter()
                .find(|entry| entry.pid == options.canister)
                .and_then(|entry| entry.role)
        });
        return Ok(EndpointTarget {
            canister: options.canister.clone(),
            role,
        });
    }

    let registry = load_fleet_registry(options)?;
    let entry = registry
        .iter()
        .find(|entry| entry.role.as_deref() == Some(options.canister.as_str()))
        .ok_or_else(|| -> Box<dyn std::error::Error> {
            format!(
                "role {} was not found in fleet {}",
                options.canister, options.fleet
            )
            .into()
        })?;
    Ok(EndpointTarget {
        canister: entry.pid.clone(),
        role: entry.role.clone(),
    })
}

fn load_fleet_registry(
    options: &EndpointsOptions,
) -> Result<Vec<RegistryEntry>, Box<dyn std::error::Error>> {
    let network = state_network(options);
    let state = read_named_fleet_install_state(&network, &options.fleet)?.ok_or_else(|| {
        format!(
            "fleet {} is not installed on network {network}",
            options.fleet
        )
    })?;
    let registry_json = if replica_query::should_use_local_replica_query(options.network.as_deref())
    {
        replica_query::query_subnet_registry_json(
            options.network.as_deref(),
            &state.root_canister_id,
        )?
    } else {
        IcpCli::new(&options.icp, None, options.network.clone()).canister_call_output(
            &state.root_canister_id,
            "canic_subnet_registry",
            Some("json"),
        )?
    };
    Ok(parse_registry_entries(&registry_json)?)
}

fn resolve_role_did(
    options: &EndpointsOptions,
    role: &str,
) -> Result<PathBuf, EndpointsCommandError> {
    let root = icp_root().unwrap_or_else(|_| PathBuf::from("."));
    for network in artifact_network_candidates(options) {
        let path = root
            .join(".icp")
            .join(&network)
            .join("canisters")
            .join(role)
            .join(format!("{role}.did"));
        if path.is_file() {
            return Ok(path);
        }
    }

    Err(EndpointsCommandError::MissingRoleArtifact {
        role: role.to_string(),
        root: root.display().to_string(),
    })
}

fn artifact_network_candidates(options: &EndpointsOptions) -> Vec<String> {
    let mut networks = Vec::new();
    if let Some(network) = &options.network {
        networks.push(network.clone());
    }
    networks.push(local_network());
    networks.sort();
    networks.dedup();
    networks
}

fn state_network(options: &EndpointsOptions) -> String {
    options.network.clone().unwrap_or_else(local_network)
}

fn read_did(path: &Path) -> Result<String, EndpointsCommandError> {
    fs::read_to_string(path).map_err(|source| EndpointsCommandError::ReadDid {
        path: path.display().to_string(),
        source,
    })
}

fn parse_candid_service_endpoints(
    candid: &str,
) -> Result<Vec<EndpointEntry>, EndpointsCommandError> {
    let (env, actor) = CandidSource::Text(candid)
        .load()
        .map_err(|err| EndpointsCommandError::InvalidCandid(err.to_string()))?;
    let Some(actor) = actor else {
        return Err(EndpointsCommandError::MissingService);
    };
    let service = env
        .as_service(&actor)
        .map_err(|_| EndpointsCommandError::MissingService)?;
    service
        .iter()
        .map(|(name, ty)| endpoint_entry(&env, name, ty))
        .collect()
}

fn endpoint_entry(
    env: &TypeEnv,
    name: &str,
    ty: &Type,
) -> Result<EndpointEntry, EndpointsCommandError> {
    let function = env
        .as_func(ty)
        .map_err(|err| EndpointsCommandError::InvalidCandid(err.to_string()))?;
    Ok(EndpointEntry {
        name: name.to_string(),
        candid: format!("{} : {};", render_candid_method_name(name), function),
        modes: endpoint_modes(function),
        arguments: endpoint_types(env, &function.args),
        returns: endpoint_types(env, &function.rets),
    })
}

fn endpoint_types(env: &TypeEnv, types: &[Type]) -> Vec<EndpointType> {
    types
        .iter()
        .map(|ty| endpoint_type(env, ty, &mut Vec::new()))
        .collect()
}

fn endpoint_type(env: &TypeEnv, ty: &Type, named_stack: &mut Vec<String>) -> EndpointType {
    match ty.as_ref() {
        TypeInner::Null => primitive_type(ty, "null"),
        TypeInner::Bool => primitive_type(ty, "bool"),
        TypeInner::Nat => primitive_type(ty, "nat"),
        TypeInner::Int => primitive_type(ty, "int"),
        TypeInner::Nat8 => primitive_type(ty, "nat8"),
        TypeInner::Nat16 => primitive_type(ty, "nat16"),
        TypeInner::Nat32 => primitive_type(ty, "nat32"),
        TypeInner::Nat64 => primitive_type(ty, "nat64"),
        TypeInner::Int8 => primitive_type(ty, "int8"),
        TypeInner::Int16 => primitive_type(ty, "int16"),
        TypeInner::Int32 => primitive_type(ty, "int32"),
        TypeInner::Int64 => primitive_type(ty, "int64"),
        TypeInner::Float32 => primitive_type(ty, "float32"),
        TypeInner::Float64 => primitive_type(ty, "float64"),
        TypeInner::Text => primitive_type(ty, "text"),
        TypeInner::Reserved => primitive_type(ty, "reserved"),
        TypeInner::Empty => primitive_type(ty, "empty"),
        TypeInner::Principal => primitive_type(ty, "principal"),
        TypeInner::Future => primitive_type(ty, "future"),
        TypeInner::Unknown => primitive_type(ty, "unknown"),
        TypeInner::Knot(id) => EndpointType::Named {
            candid: ty.to_string(),
            cardinality: EndpointCardinality::Single,
            name: id.to_string(),
            resolved: None,
        },
        TypeInner::Var(name) => named_type(env, ty, name, named_stack),
        TypeInner::Opt(inner) => EndpointType::Optional {
            candid: ty.to_string(),
            cardinality: EndpointCardinality::Optional,
            inner: Box::new(endpoint_type(env, inner, named_stack)),
        },
        TypeInner::Vec(inner) => EndpointType::Vector {
            candid: ty.to_string(),
            cardinality: EndpointCardinality::Many,
            inner: Box::new(endpoint_type(env, inner, named_stack)),
        },
        TypeInner::Record(fields) => EndpointType::Record {
            candid: ty.to_string(),
            cardinality: EndpointCardinality::Single,
            fields: endpoint_fields(env, fields, named_stack),
        },
        TypeInner::Variant(fields) => EndpointType::Variant {
            candid: ty.to_string(),
            cardinality: EndpointCardinality::Single,
            cases: endpoint_fields(env, fields, named_stack),
        },
        TypeInner::Func(function) => EndpointType::Function {
            candid: ty.to_string(),
            cardinality: EndpointCardinality::Single,
            modes: endpoint_modes(function),
            arguments: endpoint_types(env, &function.args),
            returns: endpoint_types(env, &function.rets),
        },
        TypeInner::Service(methods) => EndpointType::Service {
            candid: ty.to_string(),
            cardinality: EndpointCardinality::Single,
            methods: methods
                .iter()
                .map(|(name, ty)| EndpointServiceMethod {
                    name: name.clone(),
                    ty: endpoint_type(env, ty, named_stack),
                })
                .collect(),
        },
        TypeInner::Class(initializers, service) => EndpointType::Class {
            candid: ty.to_string(),
            cardinality: EndpointCardinality::Single,
            initializers: endpoint_types(env, initializers),
            service: Box::new(endpoint_type(env, service, named_stack)),
        },
    }
}

fn primitive_type(ty: &Type, name: &str) -> EndpointType {
    EndpointType::Primitive {
        candid: ty.to_string(),
        cardinality: EndpointCardinality::Single,
        name: name.to_string(),
    }
}

fn named_type(env: &TypeEnv, ty: &Type, name: &str, named_stack: &mut Vec<String>) -> EndpointType {
    let (cardinality, resolved) = if named_stack.iter().any(|seen| seen == name) {
        (EndpointCardinality::Single, None)
    } else if let Ok(resolved) = env.find_type(name) {
        named_stack.push(name.to_string());
        let cardinality = endpoint_cardinality(env, resolved, named_stack);
        let resolved = endpoint_type(env, resolved, named_stack);
        named_stack.pop();
        (cardinality, Some(Box::new(resolved)))
    } else {
        (EndpointCardinality::Single, None)
    };
    EndpointType::Named {
        candid: ty.to_string(),
        cardinality,
        name: name.to_string(),
        resolved,
    }
}

fn endpoint_cardinality(
    env: &TypeEnv,
    ty: &Type,
    named_stack: &mut Vec<String>,
) -> EndpointCardinality {
    match ty.as_ref() {
        TypeInner::Opt(_) => EndpointCardinality::Optional,
        TypeInner::Vec(_) => EndpointCardinality::Many,
        TypeInner::Var(name) if !named_stack.iter().any(|seen| seen == name) => {
            if let Ok(resolved) = env.find_type(name) {
                named_stack.push(name.clone());
                let cardinality = endpoint_cardinality(env, resolved, named_stack);
                named_stack.pop();
                cardinality
            } else {
                EndpointCardinality::Single
            }
        }
        _ => EndpointCardinality::Single,
    }
}

fn endpoint_fields(
    env: &TypeEnv,
    fields: &[candid::types::Field],
    named_stack: &mut Vec<String>,
) -> Vec<EndpointField> {
    fields
        .iter()
        .map(|field| EndpointField {
            label: field_label(&field.id),
            id: field.id.get_id(),
            ty: endpoint_type(env, &field.ty, named_stack),
        })
        .collect()
}

fn field_label(label: &Label) -> String {
    match label {
        Label::Named(name) => name.clone(),
        Label::Id(id) | Label::Unnamed(id) => id.to_string(),
    }
}

fn endpoint_modes(function: &Function) -> Vec<EndpointMode> {
    if function.modes.is_empty() {
        return vec![EndpointMode::Update];
    }
    function
        .modes
        .iter()
        .map(|mode| match mode {
            FuncMode::Query => EndpointMode::Query,
            FuncMode::CompositeQuery => EndpointMode::CompositeQuery,
            FuncMode::Oneway => EndpointMode::Oneway,
        })
        .collect()
}

fn render_candid_method_name(name: &str) -> String {
    if is_candid_identifier(name) && !is_candid_reserved_word(name) {
        name.to_string()
    } else {
        format!("{name:?}")
    }
}

fn is_candid_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn is_candid_reserved_word(name: &str) -> bool {
    matches!(
        name,
        "blob"
            | "bool"
            | "composite_query"
            | "empty"
            | "false"
            | "float32"
            | "float64"
            | "func"
            | "import"
            | "int"
            | "int8"
            | "int16"
            | "int32"
            | "int64"
            | "nat"
            | "nat8"
            | "nat16"
            | "nat32"
            | "nat64"
            | "null"
            | "oneway"
            | "opt"
            | "principal"
            | "query"
            | "record"
            | "reserved"
            | "service"
            | "text"
            | "true"
            | "type"
            | "variant"
            | "vec"
    )
}

fn is_principal_like(value: &str) -> bool {
    value.contains('-')
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
}

fn string_value(matches: &clap::ArgMatches, id: &str) -> Option<String> {
    matches.try_get_one::<String>(id).ok().flatten().cloned()
}

fn command() -> ClapCommand {
    ClapCommand::new("endpoints")
        .bin_name("canic endpoints")
        .disable_help_flag(true)
        .about("List callable methods exposed by a canister Candid interface")
        .arg(
            value_arg("fleet")
                .value_name("fleet")
                .required(true)
                .help("Fleet name to inspect"),
        )
        .arg(
            value_arg("canister")
                .value_name("canister-or-role")
                .required(true)
                .help("Canister principal or role name to inspect"),
        )
        .arg(internal_network_arg())
        .arg(internal_icp_arg())
        .arg(flag_arg("json").long("json").help("Print JSON output"))
        .after_help(HELP_AFTER)
}

fn usage() -> String {
    let mut command = command();
    command.render_help().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const CANDID: &str = r#"
type Nested = record { field : text };
service : (record { init : text }) -> {
  canic_ready : () -> (bool) query;
  "icrc10-supported-standards" : () -> (vec record { text; text }) query;
  canic_update : (Nested) -> (
      variant { Ok; Err : text },
    );
}
"#;

    // Ensure generated Candid service files can be reduced to endpoint signatures.
    #[test]
    fn parses_candid_service_endpoints() {
        let endpoints = super::parse_candid_service_endpoints(CANDID).expect("parse endpoints");
        let canic_ready = endpoints
            .iter()
            .find(|endpoint| endpoint.name == "canic_ready")
            .expect("canic_ready endpoint");
        let icrc10 = endpoints
            .iter()
            .find(|endpoint| endpoint.name == "icrc10-supported-standards")
            .expect("icrc10 endpoint");
        let canic_update = endpoints
            .iter()
            .find(|endpoint| endpoint.name == "canic_update")
            .expect("canic_update endpoint");

        assert_eq!(endpoints.len(), 3);
        assert_eq!(canic_ready.candid, "canic_ready : () -> (bool) query;");
        assert_eq!(canic_ready.modes, vec![EndpointMode::Query]);
        assert_eq!(
            icrc10.candid,
            "\"icrc10-supported-standards\" : () -> (vec record { text; text }) query;"
        );
        assert_eq!(canic_update.modes, vec![EndpointMode::Update]);
        assert_eq!(canic_update.arguments.len(), 1);
        assert!(matches!(
            &canic_update.arguments[0],
            EndpointType::Named {
                name,
                resolved: Some(_),
                ..
            } if name == "Nested"
        ));
        assert!(matches!(
            &canic_update.returns[0],
            EndpointType::Variant { cases, .. } if cases.len() == 2
        ));
    }

    // Ensure multiline argument lists are parsed as structured endpoint types.
    #[test]
    fn parses_multiline_endpoint_arguments() {
        let candid = r#"
service : {
  "import" : (
    record {
      payload : text;
    },
  ) -> (variant { Ok; Err : text });
}
"#;

        let endpoints = super::parse_candid_service_endpoints(candid).expect("parse endpoints");

        assert_eq!(endpoints.len(), 1);
        assert_eq!(endpoints[0].name, "import");
        assert!(endpoints[0].candid.starts_with("\"import\" : "));
        assert_eq!(endpoints[0].arguments.len(), 1);
        assert!(matches!(
            &endpoints[0].arguments[0],
            EndpointType::Record { fields, .. }
                if fields.len() == 1 && fields[0].label == "payload"
        ));
        assert!(matches!(
            &endpoints[0].returns[0],
            EndpointType::Variant { cases, .. }
                if cases.iter().any(|case| case.label == "Ok")
                    && cases.iter().any(|case| case.label == "Err")
        ));
    }

    // Ensure multiple arguments retain cardinality and named type structure.
    #[test]
    fn parses_multiple_endpoint_arguments() {
        let candid = r"
type PageRequest = record { cursor : opt text };
service : {
  update : (opt text, record { items : vec record { id : nat; label : text } }, PageRequest) -> ();
}
";

        let endpoints = super::parse_candid_service_endpoints(candid).expect("parse endpoints");

        assert_eq!(endpoints.len(), 1);
        assert_eq!(endpoints[0].arguments.len(), 3);
        assert!(matches!(
            &endpoints[0].arguments[0],
            EndpointType::Optional {
                cardinality: EndpointCardinality::Optional,
                inner,
                ..
            } if matches!(inner.as_ref(), EndpointType::Primitive { name, .. } if name == "text")
        ));
        assert!(matches!(
            &endpoints[0].arguments[1],
            EndpointType::Record { fields, .. }
                if fields.iter().any(|field| matches!(
                    &field.ty,
                    EndpointType::Vector {
                        cardinality: EndpointCardinality::Many,
                        ..
                    }
                ))
        ));
        assert!(matches!(
            &endpoints[0].arguments[2],
            EndpointType::Named {
                name,
                resolved: Some(_),
                ..
            } if name == "PageRequest"
        ));
    }

    // Ensure fields named service before the top-level service do not confuse discovery.
    #[test]
    fn ignores_service_named_record_fields() {
        let candid = r#"
type Envelope = record {
  "service" : text;
  payload : text;
};
service : {
  ready : () -> (bool) query;
}
"#;

        let endpoints = super::parse_candid_service_endpoints(candid).expect("parse endpoints");

        assert_eq!(endpoints.len(), 1);
        assert_eq!(endpoints[0].name, "ready");
        assert_eq!(endpoints[0].candid, "ready : () -> (bool) query;");
    }

    // Ensure plain output renders one Candid method declaration.
    #[test]
    fn renders_plain_endpoint_signature() {
        let endpoint = EndpointEntry {
            name: "canic_log".to_string(),
            candid: "canic_log : (opt text, opt text, Level, PageRequest) -> ();".to_string(),
            modes: vec![EndpointMode::Update],
            arguments: Vec::new(),
            returns: Vec::new(),
        };

        assert_eq!(
            endpoint.render(),
            "canic_log : (opt text, opt text, Level, PageRequest) -> ();"
        );
    }

    // Ensure JSON exposes structured types instead of requiring callers to parse strings.
    #[test]
    fn serializes_structured_endpoint_json() {
        let candid = r"
type MaybeText = opt text;
type Level = variant { Debug; Info; Error : text };
service : {
  canic_log : (MaybeText, Level) -> ();
}
";

        let endpoints = super::parse_candid_service_endpoints(candid).expect("parse endpoints");
        let json = serde_json::to_string(&endpoints[0]).expect("serialize endpoint");

        assert!(json.contains(r#""kind":"optional""#));
        assert!(json.contains(r#""cardinality":"optional""#));
        assert!(json.contains(r#""kind":"named""#));
        assert!(json.contains(r#""name":"MaybeText""#));
        assert!(json.contains(r#""name":"Level""#));
        assert!(json.contains(r#""kind":"variant""#));
        assert!(json.contains(r#""label":"Error""#));
    }

    // Ensure endpoint options parse local and live lookup controls.
    #[test]
    fn parses_endpoint_options() {
        let options = EndpointsOptions::parse([
            OsString::from("test"),
            OsString::from("app"),
            OsString::from(crate::args::INTERNAL_NETWORK_OPTION),
            OsString::from("local"),
            OsString::from(crate::args::INTERNAL_ICP_OPTION),
            OsString::from("/bin/icp"),
            OsString::from("--json"),
        ])
        .expect("parse options");

        assert_eq!(options.fleet, "test");
        assert_eq!(options.canister, "app");
        assert_eq!(options.network.as_deref(), Some("local"));
        assert_eq!(options.icp, "/bin/icp");
        assert!(options.json);
    }

    // Ensure direct Candid-file selection is not part of fleet-scoped endpoint lookup.
    #[test]
    fn rejects_did_option() {
        let err = EndpointsOptions::parse([
            OsString::from("test"),
            OsString::from("app"),
            OsString::from("--did"),
            OsString::from("app.did"),
        ])
        .expect_err("did override should be removed");

        assert!(matches!(err, EndpointsCommandError::Usage(_)));
    }

    // Ensure explicit role fallback is not part of fleet-scoped endpoint lookup.
    #[test]
    fn rejects_role_option() {
        let err = EndpointsOptions::parse([
            OsString::from("test"),
            OsString::from("tl4x7-vh777-77776-aaacq-cai"),
            OsString::from("--role"),
            OsString::from("scale_hub"),
        ])
        .expect_err("role override should be removed");

        assert!(matches!(err, EndpointsCommandError::Usage(_)));
    }
}
