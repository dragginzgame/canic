use super::*;
use crate::endpoint::parse::{AccessExprAst, AccessPredicateAst, BuiltinPredicate, ParsedArgs};

#[test]
fn guarded_plain_sync_handlers_require_rejection_and_decoder_has_one_owner() {
    let signature: Signature = syn::parse_quote!(fn plain(value: u64) -> u64);
    let options = crate::endpoint::parse::parse_args(quote::quote!(
        requires(caller::is_controller()),
        on_access_denied = "reject",
        decode = LIMITS
    ))
    .unwrap();
    validate(EndpointKind::Update, options, &signature).unwrap();
    let options =
        crate::endpoint::parse::parse_args(quote::quote!(requires(caller::is_controller())))
            .unwrap();
    assert!(validate(EndpointKind::Update, options, &signature).is_err());
    let options = crate::endpoint::parse::parse_args(quote::quote!(
        public,
        decode = LIMITS,
        payload(max_bytes = 3)
    ))
    .unwrap();
    assert!(validate(EndpointKind::Query, options, &signature).is_err());
}

fn parsed_authenticated() -> ParsedArgs {
    ParsedArgs {
        forwarded: Vec::new(),
        export_name: None,
        payload_max_bytes: None,
        decode: None,
        reject_access: false,
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

fn parsed_attested_local_subnet() -> ParsedArgs {
    ParsedArgs {
        forwarded: Vec::new(),
        export_name: None,
        payload_max_bytes: None,
        decode: None,
        reject_access: false,
        requires: vec![AccessExprAst::Pred(AccessPredicateAst::Builtin(
            BuiltinPredicate::AttestedLocalSubnet,
        ))],
        internal: false,
        public: false,
        query_mode: QueryMode::Plain,
    }
}

#[test]
fn authenticated_requires_first_argument() {
    let sig: Signature = syn::parse_quote!(async fn hello() -> Result<(), ::canic::Error>);
    let err = validate(EndpointKind::Update, parsed_authenticated(), &sig).unwrap_err();
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
    validate(EndpointKind::Update, parsed_authenticated(), &sig).expect("authenticated arg ok");
}

#[test]
fn authenticated_rejects_wrong_first_arg_type() {
    let sig: Signature = syn::parse_quote!(
        async fn hello(user: ::candid::Principal) -> Result<(), ::canic::Error>
    );
    let err = validate(EndpointKind::Update, parsed_authenticated(), &sig).unwrap_err();
    assert!(
        err.to_string()
            .contains("authenticated(...) requires a first argument")
    );
}

#[test]
fn attested_local_subnet_requires_signed_role_attestation_first() {
    let missing: Signature = syn::parse_quote!(async fn hello() -> Result<(), ::canic::Error>);
    let err = validate(
        EndpointKind::Update,
        parsed_attested_local_subnet(),
        &missing,
    )
    .expect_err("missing attestation argument must fail");
    assert!(err.to_string().contains(
        "attested_local_subnet() requires a first argument of type `SignedRoleAttestation`"
    ));

    let accepted: Signature = syn::parse_quote!(
        async fn hello(
            attestation: ::canic::dto::auth::SignedRoleAttestation,
        ) -> Result<(), ::canic::Error>
    );
    validate(
        EndpointKind::Update,
        parsed_attested_local_subnet(),
        &accepted,
    )
    .expect("signed role attestation should satisfy local-Subnet auth shape");
}

#[test]
fn negated_caller_predicate_is_rejected() {
    let sig: Signature = syn::parse_quote!(async fn hello() -> Result<(), ::canic::Error>);
    let parsed = ParsedArgs {
        forwarded: Vec::new(),
        export_name: None,
        payload_max_bytes: None,
        decode: None,
        reject_access: false,
        requires: vec![AccessExprAst::Not(Box::new(AccessExprAst::Pred(
            AccessPredicateAst::Builtin(BuiltinPredicate::CallerIsController),
        )))],
        internal: false,
        public: false,
        query_mode: QueryMode::Plain,
    };

    let err = validate(EndpointKind::Update, parsed, &sig).unwrap_err();
    assert!(err.to_string().contains("not(...) must not wrap"));
}

#[test]
fn negated_service_authority_predicate_is_rejected() {
    let sig: Signature = syn::parse_quote!(async fn hello() -> Result<(), ::canic::Error>);
    let parsed = ParsedArgs {
        forwarded: Vec::new(),
        export_name: None,
        payload_max_bytes: None,
        decode: None,
        reject_access: false,
        requires: vec![AccessExprAst::Not(Box::new(AccessExprAst::Pred(
            AccessPredicateAst::Builtin(BuiltinPredicate::ServiceAuthority {
                service: crate::endpoint::parse::AuthScopeArg::Literal("database".to_string()),
            }),
        )))],
        internal: false,
        public: false,
        query_mode: QueryMode::Plain,
    };

    let err = validate(EndpointKind::Update, parsed, &sig).unwrap_err();
    assert!(err.to_string().contains("not(...) must not wrap"));
}

#[test]
fn managed_caller_permissions_cannot_be_negated() {
    let sig: Signature = syn::parse_quote!(async fn notify() -> Result<(), ::canic::Error>);
    let parsed = crate::endpoint::parse::parse_args(quote::quote!(requires(not(
        caller::has_permission("notify")
    ))))
    .unwrap();
    assert!(validate(EndpointKind::Update, parsed, &sig).is_err());
}

#[test]
fn ungated_endpoint_without_public_marker_is_rejected() {
    let sig: Signature = syn::parse_quote!(fn hello() -> Result<(), ::canic::Error>);
    let parsed = ParsedArgs {
        forwarded: Vec::new(),
        export_name: None,
        payload_max_bytes: None,
        decode: None,
        reject_access: false,
        requires: Vec::new(),
        internal: false,
        public: false,
        query_mode: QueryMode::Plain,
    };

    let err = validate(EndpointKind::Query, parsed, &sig).unwrap_err();
    assert!(err.to_string().contains("endpoint access must be explicit"));
}

#[test]
fn payload_limit_is_supported_for_queries() {
    let sig: Signature = syn::parse_quote!(fn hello() -> bool);
    let parsed = ParsedArgs {
        forwarded: Vec::new(),
        export_name: None,
        payload_max_bytes: Some(quote::quote!(1024)),
        decode: None,
        reject_access: false,
        requires: Vec::new(),
        internal: false,
        public: true,
        query_mode: QueryMode::Plain,
    };

    validate(EndpointKind::Query, parsed, &sig).unwrap();
}

#[test]
fn composite_query_marker_is_query_only() {
    let sig: Signature = syn::parse_quote!(fn hello() -> bool);
    let parsed = ParsedArgs {
        forwarded: vec![quote::quote!(composite = true)],
        export_name: None,
        payload_max_bytes: None,
        decode: None,
        reject_access: false,
        requires: Vec::new(),
        internal: false,
        public: true,
        query_mode: QueryMode::Composite,
    };

    let err = validate(EndpointKind::Update, parsed, &sig).unwrap_err();
    assert!(
        err.to_string()
            .contains("composite is supported only on canic_query")
    );
}
