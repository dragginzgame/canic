//! Framework call instrumentation consumes the shared owner's transport contracts.

use syn::{FnArg, Item, ReturnType, Type};

#[test]
fn framework_execution_uses_upstream_request_and_response_types() {
    let module = syn::parse_file(include_str!("../src/ops/ic/call/mod.rs")).unwrap();
    let execute = module
        .items
        .iter()
        .find_map(|item| match item {
            Item::Fn(function) if function.sig.ident == "execute" => Some(function),
            _ => None,
        })
        .expect("framework execution boundary");
    assert!(execute.sig.asyncness.is_some());
    assert!(execute.sig.generics.params.is_empty());
    let FnArg::Typed(request) = execute.sig.inputs.first().unwrap() else {
        panic!("execution accepts a prepared request");
    };
    assert_path(&request.ty, &["ic_call", "Call"]);
    let ReturnType::Type(_, output) = &execute.sig.output else {
        panic!("execution returns the upstream response or a typed failure");
    };
    let Type::Path(result) = output.as_ref() else {
        panic!("typed execution result");
    };
    let syn::PathArguments::AngleBracketed(arguments) =
        &result.path.segments.last().unwrap().arguments
    else {
        panic!("result arguments");
    };
    let syn::GenericArgument::Type(response) = arguments.args.first().unwrap() else {
        panic!("response type");
    };
    assert_path(response, &["ic_call", "Response"]);
}

#[test]
fn framework_instrumentation_is_stateless() {
    let module = syn::parse_file(include_str!("../src/ops/ic/call/mod.rs")).unwrap();
    for item in module.items {
        assert!(
            matches!(item, Item::Use(_) | Item::Fn(_) | Item::Mod(_)),
            "instrumentation consists of functions over upstream contracts"
        );
    }
}

fn assert_path(ty: &Type, expected: &[&str]) {
    let Type::Path(path) = ty else {
        panic!("upstream named type");
    };
    assert_eq!(
        path.path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>(),
        expected
    );
}
