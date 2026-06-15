use candid::{
    TypeEnv,
    types::{FuncMode, Function, Label, Type, TypeInner},
};
use candid_parser::utils::CandidSource;
use serde::Serialize;
use thiserror::Error as ThisError;

///
/// CandidEndpointError
///

#[derive(Debug, ThisError)]
pub enum CandidEndpointError {
    #[error("canister interface did not contain a service block")]
    MissingService,

    #[error("failed to parse Candid interface: {0}")]
    InvalidCandid(String),
}

///
/// EndpointEntry
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EndpointEntry {
    pub name: String,
    pub candid: String,
    pub modes: Vec<EndpointMode>,
    pub arguments: Vec<EndpointType>,
    pub returns: Vec<EndpointType>,
}

///
/// EndpointMode
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EndpointMode {
    Query,
    CompositeQuery,
    Oneway,
}

///
/// EndpointCardinality
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EndpointCardinality {
    Single,
    Optional,
    Many,
}

///
/// EndpointType
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EndpointType {
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
pub struct EndpointField {
    pub label: String,
    pub id: u32,
    pub ty: EndpointType,
}

///
/// EndpointServiceMethod
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EndpointServiceMethod {
    pub name: String,
    pub ty: EndpointType,
}

/// Parse a Candid service interface into structured endpoint descriptions.
pub fn parse_candid_service_endpoints(
    candid: &str,
) -> Result<Vec<EndpointEntry>, CandidEndpointError> {
    let (env, actor) = CandidSource::Text(candid)
        .load()
        .map_err(|err| CandidEndpointError::InvalidCandid(err.to_string()))?;
    let Some(actor) = actor else {
        return Err(CandidEndpointError::MissingService);
    };
    let service = env
        .as_service(&actor)
        .map_err(|_| CandidEndpointError::MissingService)?;
    service
        .iter()
        .map(|(name, ty)| endpoint_entry(&env, name, ty))
        .collect()
}

fn endpoint_entry(
    env: &TypeEnv,
    name: &str,
    ty: &Type,
) -> Result<EndpointEntry, CandidEndpointError> {
    let function = env
        .as_func(ty)
        .map_err(|err| CandidEndpointError::InvalidCandid(err.to_string()))?;
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

/// Render a Candid method name, quoting identifiers that Candid requires quoted.
#[must_use]
pub fn render_candid_method_name(name: &str) -> String {
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

#[cfg(test)]
mod tests;
