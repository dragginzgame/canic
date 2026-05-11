use crate::endpoints::{
    EndpointsCommandError,
    model::{
        EndpointCardinality, EndpointEntry, EndpointField, EndpointMode, EndpointServiceMethod,
        EndpointType,
    },
    render::render_candid_method_name,
};
use candid::{
    TypeEnv,
    types::{FuncMode, Function, Label, Type, TypeInner},
};
use candid_parser::utils::CandidSource;

pub(super) fn parse_candid_service_endpoints(
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
