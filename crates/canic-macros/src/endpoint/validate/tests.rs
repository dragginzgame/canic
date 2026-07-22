use super::*;
use crate::endpoint::parse::{AccessExprAst, AccessPredicateAst, BuiltinPredicate, ParsedArgs};

fn parsed_authenticated() -> ParsedArgs {
    ParsedArgs {
        forwarded: Vec::new(),
        export_name: None,
        payload_max_bytes: None,
        requires: vec![AccessExprAst::Pred(AccessPredicateAst::Builtin(
            BuiltinPredicate::Authenticated {
                required_scope: None,
            },
        ))],
        internal: false,
        public: false,
        query_mode: QueryMode::Plain,
    }
}

fn parsed_registered_to_subnet(internal: bool) -> ParsedArgs {
    ParsedArgs {
        forwarded: Vec::new(),
        export_name: None,
        payload_max_bytes: None,
        requires: vec![AccessExprAst::Pred(AccessPredicateAst::Builtin(
            BuiltinPredicate::CallerIsRegisteredToSubnet,
        ))],
        internal,
        public: false,
        query_mode: QueryMode::Plain,
    }
}

#[test]
fn authenticated_requires_first_argument() {
    let sig: Signature = syn::parse_quote!(async fn hello() -> Result<(), ::canic::Error>);
    let err = validate(EndpointKind::Update, parsed_authenticated(), &sig, true).unwrap_err();
    assert!(
        err.to_string()
            .contains("authenticated(...) requires a first argument")
    );
}

#[test]
fn authenticated_accepts_delegated_token_first_arg() {
    let sig: Signature = syn::parse_quote!(
        async fn hello(token: ::canic::dto::auth::DelegatedToken) -> Result<(), ::canic::Error>
    );
    validate(EndpointKind::Update, parsed_authenticated(), &sig, true)
        .expect("authenticated arg ok");
}

#[test]
fn authenticated_rejects_wrong_first_arg_type() {
    let sig: Signature = syn::parse_quote!(
        async fn hello(user: ::candid::Principal) -> Result<(), ::canic::Error>
    );
    let err = validate(EndpointKind::Update, parsed_authenticated(), &sig, true).unwrap_err();
    assert!(
        err.to_string()
            .contains("authenticated(...) requires a first argument")
    );
}

#[test]
fn registered_to_subnet_requires_internal_endpoint() {
    let sig: Signature = syn::parse_quote!(async fn hello() -> Result<(), ::canic::Error>);
    let err = validate(
        EndpointKind::Update,
        parsed_registered_to_subnet(false),
        &sig,
        true,
    )
    .unwrap_err();
    assert!(
        err.to_string()
            .contains("caller topology predicates are internal-only")
    );
}

#[test]
fn registered_to_subnet_is_allowed_for_internal_endpoint() {
    let sig: Signature = syn::parse_quote!(async fn hello() -> Result<(), ::canic::Error>);
    validate(
        EndpointKind::Update,
        parsed_registered_to_subnet(true),
        &sig,
        true,
    )
    .expect("internal predicate ok");
}

#[test]
fn negated_caller_predicate_is_rejected() {
    let sig: Signature = syn::parse_quote!(async fn hello() -> Result<(), ::canic::Error>);
    let parsed = ParsedArgs {
        forwarded: Vec::new(),
        export_name: None,
        payload_max_bytes: None,
        requires: vec![AccessExprAst::Not(Box::new(AccessExprAst::Pred(
            AccessPredicateAst::Builtin(BuiltinPredicate::CallerIsController),
        )))],
        internal: false,
        public: false,
        query_mode: QueryMode::Plain,
    };

    let err = validate(EndpointKind::Update, parsed, &sig, true).unwrap_err();
    assert!(
        err.to_string()
            .contains("not(...) must not wrap caller::* or auth::* predicates")
    );
}

#[test]
fn ungated_endpoint_without_public_marker_is_rejected() {
    let sig: Signature = syn::parse_quote!(fn hello() -> Result<(), ::canic::Error>);
    let parsed = ParsedArgs {
        forwarded: Vec::new(),
        export_name: None,
        payload_max_bytes: None,
        requires: Vec::new(),
        internal: false,
        public: false,
        query_mode: QueryMode::Plain,
    };

    let err = validate(EndpointKind::Query, parsed, &sig, false).unwrap_err();
    assert!(err.to_string().contains("endpoint access must be explicit"));
}

#[test]
fn payload_limit_is_update_only() {
    let sig: Signature = syn::parse_quote!(fn hello() -> bool);
    let parsed = ParsedArgs {
        forwarded: Vec::new(),
        export_name: None,
        payload_max_bytes: Some(quote::quote!(1024)),
        requires: Vec::new(),
        internal: false,
        public: true,
        query_mode: QueryMode::Plain,
    };

    let err = validate(EndpointKind::Query, parsed, &sig, false).unwrap_err();
    assert!(
        err.to_string()
            .contains("payload(...) is supported only on canic_update")
    );
}

#[test]
fn composite_query_marker_is_query_only() {
    let sig: Signature = syn::parse_quote!(fn hello() -> bool);
    let parsed = ParsedArgs {
        forwarded: vec![quote::quote!(composite = true)],
        export_name: None,
        payload_max_bytes: None,
        requires: Vec::new(),
        internal: false,
        public: true,
        query_mode: QueryMode::Composite,
    };

    let err = validate(EndpointKind::Update, parsed, &sig, false).unwrap_err();
    assert!(
        err.to_string()
            .contains("composite is supported only on canic_query")
    );
}
