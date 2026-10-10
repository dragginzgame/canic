//! Qualify generated role declarations against the canonical wire owner.
//! Capability selection may remove selectors; every retained payload stays exact.

use candid::{
    CandidType,
    types::{FuncMode, Type, TypeEnv, TypeInner, internal::TypeContainer, subtype},
};
use canic_contracts::{dto, dto::wire, protocol};
use std::{any::TypeId, collections::HashSet, fs};

fn selected<T: CandidType + 'static>(env: &TypeEnv, actual: &Type) {
    let mut env = env.clone();
    let mut rust = TypeContainer::new();
    let expected = rust.add::<T>();
    let expected = env.merge_type(rust.env, expected);
    let actual = env.trace_type(actual).expect("generated type");
    let expected = env.trace_type(&expected).expect("canonical type");
    match (actual.as_ref(), expected.as_ref()) {
        (TypeInner::Variant(actual), TypeInner::Variant(expected)) => {
            assert!(!actual.is_empty(), "selected union has selectors");
            for field in actual {
                let canonical = expected
                    .iter()
                    .find(|candidate| candidate.id == field.id)
                    .expect("selected label belongs to the canonical union");
                // Managed observation relays use the same capability selection
                // inside their reply. All other nested DTOs remain exact.
                if TypeId::of::<T>()
                    == TypeId::of::<wire::managed_command::CanisterCommandResponse>()
                    && field.id.get_id() == candid::idl_hash("Observe")
                {
                    selected::<wire::relay::RelayedObservabilityResponse>(&env, &field.ty);
                } else {
                    subtype::equal(&mut HashSet::new(), &env, &field.ty, &canonical.ty)
                        .expect("selected payload equals its canonical declaration");
                }
            }
        }
        _ => subtype::equal(&mut HashSet::new(), &env, &actual, &expected)
            .expect("generated type equals its canonical declaration"),
    }
}

fn field(env: &TypeEnv, ty: &Type, name: &str) -> Type {
    let ty = env.trace_type(ty).expect("result type");
    let TypeInner::Variant(fields) = ty.as_ref() else {
        panic!("expected Result variant");
    };
    assert_eq!(fields.len(), 2, "Result has exactly Ok and Err");
    fields
        .iter()
        .find(|field| field.id.get_id() == candid::idl_hash(name))
        .expect("Result label")
        .ty
        .clone()
}

fn method<Q: CandidType + 'static, R: CandidType + 'static>(
    env: &TypeEnv,
    actor: &Type,
    name: &str,
    modes: &[FuncMode],
) {
    let method = env.get_method(actor, name).expect("selected role method");
    assert_eq!(method.modes, modes, "{name} modes");
    assert_eq!(method.args.len(), 1, "{name} request arity");
    assert_eq!(method.rets.len(), 1, "{name} reply arity");
    selected::<Q>(env, &method.args[0]);
    selected::<R>(env, &field(env, &method.rets[0], "Ok"));
    selected::<dto::error::Error>(env, &field(env, &method.rets[0], "Err"));
    println!("{name}: exact canonical selector payloads");
}

fn query<Q: CandidType + 'static, R: CandidType + 'static>(
    env: &TypeEnv,
    actor: &Type,
    name: &str,
) {
    method::<Q, R>(env, actor, name, &[FuncMode::Query]);
}

fn command<Q: CandidType + 'static, R: CandidType + 'static>(
    env: &TypeEnv,
    actor: &Type,
    name: &str,
) {
    method::<Q, R>(env, actor, name, &[]);
}

fn qualify_root(env: &TypeEnv, actor: &Type) {
    command::<wire::root_command::RootCommand, wire::root_command::RootCommandResponse>(
        env,
        actor,
        protocol::CANIC_ROOT_COMMAND,
    );
    query::<wire::root::RootStatusRequest, wire::root::RootStatusResponse>(
        env,
        actor,
        protocol::CANIC_ROOT_STATUS,
    );
    query::<wire::root::RootOperationStatusRequest, wire::root::RootOperationStatusResponse>(
        env,
        actor,
        protocol::CANIC_ROOT_OPERATION_STATUS,
    );
    if env
        .get_method(actor, protocol::CANIC_ROOT_AUTH_STATUS)
        .is_ok()
    {
        query::<wire::root::RootAuthStatusRequest, wire::root::RootAuthStatusResponse>(
            env,
            actor,
            protocol::CANIC_ROOT_AUTH_STATUS,
        );
    }
    query::<wire::root::PublicStatusRequest, wire::root::PublicStatusResponse>(
        env,
        actor,
        protocol::CANIC_PUBLIC_STATUS,
    );
    query::<wire::root::ObservabilityRequest, wire::root::ObservabilityResponse>(
        env,
        actor,
        protocol::CANIC_OBSERVABILITY,
    );
    command::<
        dto::component_registry::RootMembershipRequest,
        dto::component_registry::RootMembershipResponse,
    >(env, actor, protocol::CANIC_ROOT_MEMBERSHIP);
}

fn qualify_managed(env: &TypeEnv, actor: &Type) {
    command::<wire::managed_command::CanisterCommand, wire::managed_command::CanisterCommandResponse>(
        env,
        actor,
        protocol::CANIC_COMMAND,
    );
    query::<wire::managed::PublicStatusRequest, wire::managed::PublicStatusResponse>(
        env,
        actor,
        protocol::CANIC_PUBLIC_STATUS,
    );
    query::<wire::managed::ObservabilityRequest, wire::managed::ObservabilityResponse>(
        env,
        actor,
        protocol::CANIC_OBSERVABILITY,
    );
    for name in [
        protocol::CANIC_AUTH_STATUS,
        protocol::CANIC_CONTROL_STATUS,
        protocol::CANIC_ADMISSION_STATUS,
    ] {
        if env.get_method(actor, name).is_ok() {
            match name {
                protocol::CANIC_AUTH_STATUS => query::<
                    wire::managed::AuthStatusRequest,
                    wire::managed::AuthStatusResponse,
                >(env, actor, name),
                protocol::CANIC_CONTROL_STATUS => query::<
                    wire::managed::ControlStatusRequest,
                    wire::managed::ControlStatusResponse,
                >(env, actor, name),
                _ => query::<
                    wire::managed::AdmissionStatusRequest,
                    wire::managed::AdmissionStatusResponse,
                >(env, actor, name),
            }
        }
    }
}

fn qualify_local(env: &TypeEnv, actor: &Type) {
    query::<wire::local::PublicStatusRequest, wire::local::PublicStatusResponse>(
        env,
        actor,
        protocol::CANIC_PUBLIC_STATUS,
    );
    query::<wire::local::ObservabilityRequest, wire::local::ObservabilityResponse>(
        env,
        actor,
        protocol::CANIC_OBSERVABILITY,
    );
}

fn qualify_coordinator(env: &TypeEnv, actor: &Type) {
    command::<
        dto::fleet_coordinator::CoordinatorCommand,
        dto::fleet_coordinator::CoordinatorCommandResponse,
    >(env, actor, protocol::CANIC_COORDINATOR_COMMAND);
    query::<
        dto::fleet_coordinator::CoordinatorOperationReadRequest,
        dto::fleet_coordinator::CoordinatorOperationReadResponse,
    >(env, actor, protocol::CANIC_COORDINATOR_OPERATION_STATUS);
    query::<
        dto::fleet_coordinator::CoordinatorRegistryRequest,
        dto::fleet_coordinator::CoordinatorRegistryResponse,
    >(env, actor, protocol::CANIC_COORDINATOR_REGISTRY);
    query::<wire::coordinator::PublicStatusRequest, wire::coordinator::PublicStatusResponse>(
        env,
        actor,
        protocol::CANIC_PUBLIC_STATUS,
    );
    query::<
        dto::fleet_coordinator::CoordinatorObservabilityRequest,
        dto::fleet_coordinator::CoordinatorObservabilityResponse,
    >(env, actor, protocol::CANIC_OBSERVABILITY);
}

fn qualify_store(env: &TypeEnv, actor: &Type) {
    command::<dto::template::StoreCommand, dto::template::StoreCommandResponse>(
        env,
        actor,
        protocol::CANIC_WASM_STORE_COMMAND,
    );
    query::<dto::template::StoreStatusRequest, dto::template::StoreStatusResponse>(
        env,
        actor,
        protocol::CANIC_WASM_STORE_STATUS,
    );
    query::<dto::template::StoreCatalogRequest, dto::template::StoreCatalogResponse>(
        env,
        actor,
        protocol::CANIC_WASM_STORE_CATALOG,
    );
    query::<wire::store::PublicStatusRequest, wire::store::PublicStatusResponse>(
        env,
        actor,
        protocol::CANIC_PUBLIC_STATUS,
    );
    query::<dto::template::StoreObservabilityRequest, dto::template::StoreObservabilityResponse>(
        env,
        actor,
        protocol::CANIC_OBSERVABILITY,
    );
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let role = args.next().ok_or("requires role and generated .did path")?;
    let path = args.next().ok_or("requires generated .did path")?;
    if args.next().is_some() {
        return Err("expected exactly role and generated .did path".into());
    }
    let program = fs::read_to_string(path)?.parse::<candid_parser::IDLProg>()?;
    let mut env = TypeEnv::new();
    let actor = candid_parser::check_prog(&mut env, &program)?.ok_or("requires service")?;
    match role.as_str() {
        "root" => qualify_root(&env, &actor),
        "managed" => qualify_managed(&env, &actor),
        "local" => qualify_local(&env, &actor),
        "fleet_coordinator" => qualify_coordinator(&env, &actor),
        "wasm_store" => qualify_store(&env, &actor),
        _ => return Err("unknown role".into()),
    }
    Ok(())
}
