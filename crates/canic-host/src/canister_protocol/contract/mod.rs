//! Module: canister_protocol::contract
//!
//! Responsibility: compare exact Candid field types for observation and effect admission.
//! Does not own: source selection, network calls or storage.
//! Boundary: parsed contracts are inspected without adapting their wire shapes.

use candid::{
    CandidType, TypeEnv,
    types::{Type, TypeInner, internal::TypeContainer},
};
use std::collections::HashSet;

/// Parse and type-check a text contract without printing diagnostics to stderr.
///
/// Callers own error reporting. The parser's pretty loader writes directly to
/// stderr, bypassing both structured diagnostics and libtest's output capture.
pub fn parse_text(source: &str) -> candid_parser::Result<(TypeEnv, Option<Type>)> {
    let program = source.parse::<candid_parser::IDLProg>()?;
    let mut environment = TypeEnv::new();
    let actor = candid_parser::check_prog(&mut environment, &program)?;
    Ok((environment, actor))
}

/// Find one exact variant label after resolving Candid type references.
pub fn variant(env: &TypeEnv, ty: &Type, name: &str) -> Option<Type> {
    let ty = env.trace_type(ty).ok()?;
    let TypeInner::Variant(fields) = ty.as_ref() else {
        return None;
    };
    fields
        .iter()
        .find(|field| field.id.get_id() == candid::idl_hash(name))
        .map(|field| field.ty.clone())
}

/// Find one exact record field after resolving Candid type references.
pub fn record(env: &TypeEnv, ty: &Type, name: &str) -> Option<Type> {
    let ty = env.trace_type(ty).ok()?;
    let TypeInner::Record(fields) = ty.as_ref() else {
        return None;
    };
    fields
        .iter()
        .find(|field| field.id.get_id() == candid::idl_hash(name))
        .map(|field| field.ty.clone())
}

/// Require structural equality with the maintained Rust wire type.
pub fn equal<T: CandidType>(env: &mut TypeEnv, expected: &Type) -> Option<()> {
    let mut rust = TypeContainer::new();
    let ty = rust.add::<T>();
    let actual = env.merge_type(rust.env, ty);
    candid::types::subtype::equal(&mut HashSet::new(), env, expected, &actual).ok()
}
