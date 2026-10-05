//! Module: pic::caller_authority::candid_contract
//!
//! Compare generated protected command/status variants with the maintained public DTOs.

use super::*;
use candid::types::{Type, TypeInner, internal::TypeContainer, subtype};
use std::path::Path;

pub(super) fn qualify(workspace: &Path) {
    let config = workspace.join("apps/test/test-configs/managed-component-group.toml");
    let artifacts = crate::pic::artifacts::build_internal_test_wasm_canisters_with_env(
        workspace,
        &workspace.join("target/pic-caller-authority-candid"),
        &["canister_user_hub"],
        crate::pic::CanicWasmBuildProfile::Fast,
        &[
            (
                canic_core::role_contract::CANONICAL_BUILD_CONFIG_PATH_ENV,
                config.to_str().unwrap(),
            ),
            (canic_core::role_contract::CANONICAL_CANDID_BUILD_ENV, "1"),
        ],
    );
    let bytes =
        canic_host::canister_build::extract_candid_bytes(artifacts.path("canister_user_hub"))
            .unwrap();
    let program = std::str::from_utf8(&bytes)
        .unwrap()
        .parse::<candid_parser::IDLProg>()
        .unwrap();
    let mut env = candid::TypeEnv::new();
    let actor = candid_parser::check_prog(&mut env, &program)
        .unwrap()
        .unwrap();
    let command = env
        .get_method(&actor, canic::protocol::CANIC_COMMAND)
        .unwrap()
        .clone();
    assert!(command.modes.is_empty());
    assert_eq!(command.args.len(), 1);
    assert_eq!(command.rets.len(), 1);
    let request = variant(&env, &command.args[0], "CallerAuthority");
    equal::<CallerAuthorityCommand>(&mut env, &request);
    let release = variant(&env, &command.args[0], "ReleaseApplicationStartup");
    equal::<CallerAuthorityPublication>(&mut env, &release);
    let result = variant(&env, &command.rets[0], "Ok");
    let receipt = variant(&env, &result, "CallerAuthority");
    equal::<CallerAuthorityReceipt>(&mut env, &receipt);
    let status = env
        .get_method(&actor, canic::protocol::CANIC_CONTROL_STATUS)
        .unwrap()
        .clone();
    assert_eq!(status.modes, vec![candid::types::FuncMode::Query]);
    let request = variant(&env, &status.args[0], "CallerAuthority");
    equal::<OperationStatusRequest>(&mut env, &request);
    let result = variant(&env, &status.rets[0], "Ok");
    let status = variant(&env, &result, "CallerAuthority");
    equal::<CallerAuthorityStatus>(&mut env, &status);
}

fn variant(env: &candid::TypeEnv, ty: &Type, name: &str) -> Type {
    let ty = env.trace_type(ty).unwrap();
    let TypeInner::Variant(fields) = ty.as_ref() else {
        panic!("expected generated Candid variant")
    };
    fields
        .iter()
        .find(|field| field.id.get_id() == candid::idl_hash(name))
        .unwrap()
        .ty
        .clone()
}

fn equal<T: CandidType>(env: &mut candid::TypeEnv, actual: &Type) {
    let mut rust = TypeContainer::new();
    let ty = rust.add::<T>();
    let expected = env.merge_type(rust.env, ty);
    subtype::equal(&mut subtype::Gamma::new(), env, actual, &expected).unwrap();
}
