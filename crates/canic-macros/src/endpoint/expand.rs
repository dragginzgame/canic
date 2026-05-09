//
// ============================================================================
// ACCESS PIPELINE & METRICS INVARIANTS
// ============================================================================
//
// This macro generates the complete access-control wrapper for canister
// endpoints. The code below is SECURITY-SENSITIVE.
//
// The following invariants are intentional and MUST be preserved:
//
// 1. Access pipeline semantics
//    --------------------------
//    Access checks are evaluated via `access::expr::eval_access`.
//    `requires(...)` always lowers to a single AccessExpr::All list.
//
//    Evaluation short-circuits on the FIRST failure.
//
// 2. Access metrics (denial-only)
//    -----------------------------
//    Access metrics are emitted ONLY on access denial paths.
//    Each denied request MUST emit EXACTLY ONE access metric via the
//    expression evaluator, tagged with the predicate kind that denied access.
//
//    Successful requests MUST emit NO access metrics.
//
//    These invariants are relied upon by access metrics aggregation logic.
//
// 3. Error handling
//    --------------
//    Access failures for gated endpoints must return a Result error; trapping
//    is forbidden outside lifecycle adapters. Infallible endpoints that can
//    deny access are rejected at compile time.
//
// 4. Macro constraints
//    ------------------
//    - requires(...) accepts only expression calls (all/any/not/custom + built-ins).
//    - `self` receivers are forbidden.
//    - Fallibility detection assumes a direct `Result<_, _>` return type.
//
// Any change to this file should be reviewed against ALL of the above
// invariants. Violating them will silently corrupt access metrics or
// authorization behavior.
//

use crate::endpoint::{
    EndpointKind,
    parse::{AccessExprAst, AccessPredicateAst, AuthScopeArg, BuiltinPredicate},
    validate::ValidatedArgs,
};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{GenericArgument, ItemFn, PathArguments, Signature, Type, visit::Visit};

//
// ============================================================================
// expand - code generation only
// ============================================================================
//

#[expect(clippy::default_trait_access)]
pub fn expand(kind: EndpointKind, args: ValidatedArgs, mut func: ItemFn) -> TokenStream2 {
    let attrs = func.attrs.clone();
    let orig_sig = func.sig.clone();
    let orig_name = orig_sig.ident.clone();
    let vis = func.vis.clone();
    let inputs = orig_sig.inputs.clone();
    let output = orig_sig.output.clone();
    let impl_async = orig_sig.asyncness.is_some();
    let returns_fallible = returns_fallible(&orig_sig);

    let access_plan = match build_access_plan(kind, &args, &orig_sig) {
        Ok(plan) => plan,
        Err(err) => return err.to_compile_error(),
    };
    if !returns_fallible && !matches!(access_plan, AccessPlan::None) {
        let message = "access-gated endpoints must return Result<_, Error> to avoid traps";
        return syn::Error::new_spanned(&orig_sig.ident, message).to_compile_error();
    }

    let wrapper_async = impl_async || access_plan.requires_async();

    let impl_name = format_ident!("__canic_impl_{}", orig_name);
    func.sig.ident = impl_name.clone();

    if requires_authenticated(&args.requires)
        && let Some(first_arg_ident) = first_typed_arg_ident(&orig_sig)
    {
        // authenticated([scope]) decodes ingress arg0 directly; keep the function arg lint-clean.
        let keepalive: syn::Stmt = syn::parse_quote!(let _ = &#first_arg_ident;);
        func.block.stmts.insert(0, keepalive);
    }

    let cdk_attr = cdk_attr(kind, &args.forwarded);
    let payload_registration = payload_registration(kind, &args, &orig_name);
    let dispatch_fn = dispatch(kind, wrapper_async);

    let wrapper_sig = syn::Signature {
        ident: orig_name.clone(),
        asyncness: if wrapper_async {
            Some(Default::default())
        } else {
            None
        },
        inputs,
        output,
        ..orig_sig.clone()
    };

    let call_ident = format_ident!("__canic_call");
    let call_decl = call_decl(kind, &call_ident, &orig_name);

    let access_stage = access_stage(&access_plan, &call_ident);

    let call_args = match extract_args(&orig_sig) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error(),
    };

    let dispatch_call = dispatch_call(
        wrapper_async,
        impl_async,
        dispatch_fn,
        &call_ident,
        impl_name,
        &call_args,
    );

    quote! {
        #payload_registration

        #(#attrs)*
        #[expect(clippy::missing_const_for_fn, clippy::unnecessary_wraps)]
        #cdk_attr
        #vis #wrapper_sig {
            #call_decl
            #access_stage
            #dispatch_call
        }

        #[expect(clippy::missing_const_for_fn, clippy::unnecessary_wraps)]
        #func
    }
}

//
// ============================================================================
// helpers
// ============================================================================
//

fn returns_fallible(sig: &syn::Signature) -> bool {
    let syn::ReturnType::Type(_, ty) = &sig.output else {
        return false;
    };
    let syn::Type::Path(ty) = &**ty else {
        return false;
    };

    ty.path
        .segments
        .last()
        .is_some_and(|seg| seg.ident == "Result")
}

fn dispatch(kind: EndpointKind, asyncness: bool) -> TokenStream2 {
    match (kind, asyncness) {
        (EndpointKind::Query, false) => {
            quote!(::canic::__internal::core::dispatch::dispatch_query)
        }
        (EndpointKind::Query, true) => {
            quote!(::canic::__internal::core::dispatch::dispatch_query_async)
        }
        (EndpointKind::Update, false) => {
            quote!(::canic::__internal::core::dispatch::dispatch_update)
        }
        (EndpointKind::Update, true) => {
            quote!(::canic::__internal::core::dispatch::dispatch_update_async)
        }
    }
}

fn payload_registration(
    kind: EndpointKind,
    args: &ValidatedArgs,
    name: &syn::Ident,
) -> TokenStream2 {
    if !matches!(kind, EndpointKind::Update) {
        return quote!();
    }

    let register_name = format_ident!("__canic_register_payload_limit_{}", name);
    let ctor_name = format_ident!("__canic_ctor_payload_limit_{}", name);
    let method_name = if let Some(name) = &args.export_name {
        quote!(#name)
    } else {
        quote!(stringify!(#name))
    };
    let max_bytes = args.payload_max_bytes.clone().unwrap_or_else(|| {
        quote!(::canic::__internal::core::ingress::payload::DEFAULT_UPDATE_INGRESS_MAX_BYTES)
    });

    quote! {
        const _: () = {
            fn #register_name() {
                ::canic::__internal::core::ingress::payload::register_update_limit(
                    #method_name,
                    #max_bytes,
                );
            }

            #[ ::canic::__internal::core::__reexports::ctor::ctor(
                unsafe,
                anonymous,
                crate_path = ::canic::__internal::core::__reexports::ctor
            ) ]
            fn #ctor_name() {
                #register_name();
            }
        };
    }
}

fn call_decl(kind: EndpointKind, call: &syn::Ident, name: &syn::Ident) -> TokenStream2 {
    let call_kind = match kind {
        EndpointKind::Query => {
            quote!(::canic::__internal::core::ids::EndpointCallKind::Query)
        }
        EndpointKind::Update => {
            quote!(::canic::__internal::core::ids::EndpointCallKind::Update)
        }
    };

    quote! {
        let #call = ::canic::__internal::core::ids::EndpointCall {
            endpoint: ::canic::__internal::core::ids::EndpointId::new(stringify!(#name)),
            kind: #call_kind,
        };
    }
}

fn access_stage(plan: &AccessPlan, call: &syn::Ident) -> TokenStream2 {
    let caller = format_ident!("__canic_caller");
    let authenticated_identity = format_ident!("__canic_authenticated_identity");
    let ctx = format_ident!("__canic_access_ctx");

    let deny = quote!(return Err(err.into()););

    match plan {
        AccessPlan::None => quote!(),
        AccessPlan::DefaultApp(guard) => {
            let guard_expr = guard_tokens(*guard);
            quote! {
                let #caller = ::canic::cdk::api::msg_caller();
                let #ctx = ::canic::__internal::core::access::expr::AccessContext {
                    caller: #caller,
                    authenticated_caller: #caller,
                    identity_source: ::canic::__internal::core::access::auth::AuthenticatedIdentitySource::RawCaller,
                    call: #call,
                };
                if let Err(err) = ::canic::__internal::core::access::expr::eval_default_app_guard(
                    #guard_expr,
                    &#ctx,
                ) {
                    #deny
                }
            }
        }
        AccessPlan::Expr(expr) => {
            let expr_ident = format_ident!("__canic_access_expr");
            quote! {
                let #caller = ::canic::cdk::api::msg_caller();
                let #authenticated_identity =
                    ::canic::__internal::core::access::auth::resolve_authenticated_identity(#caller);
                let #ctx = ::canic::__internal::core::access::expr::AccessContext {
                    caller: #authenticated_identity.transport_caller,
                    authenticated_caller: #authenticated_identity.authenticated_subject,
                    identity_source: #authenticated_identity.identity_source,
                    call: #call,
                };
                let #expr_ident = #expr;
                if let Err(err) = ::canic::__internal::core::access::expr::eval_access(&#expr_ident, &#ctx).await {
                    #deny
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Access expression synthesis
// ---------------------------------------------------------------------------

///
/// DefaultAppGuard
///

#[derive(Clone, Copy, Debug)]
enum DefaultAppGuard {
    AllowsUpdates,
    IsQueryable,
}

///
/// AccessPlan
///

#[derive(Debug)]
enum AccessPlan {
    None,
    DefaultApp(DefaultAppGuard),
    Expr(TokenStream2),
}

impl AccessPlan {
    const fn requires_async(&self) -> bool {
        matches!(self, Self::Expr(_))
    }
}

fn guard_tokens(guard: DefaultAppGuard) -> TokenStream2 {
    match guard {
        DefaultAppGuard::AllowsUpdates => {
            quote!(::canic::__internal::core::access::expr::DefaultAppGuard::AllowsUpdates)
        }
        DefaultAppGuard::IsQueryable => {
            quote!(::canic::__internal::core::access::expr::DefaultAppGuard::IsQueryable)
        }
    }
}

fn build_access_plan(
    kind: EndpointKind,
    args: &ValidatedArgs,
    sig: &Signature,
) -> syn::Result<AccessPlan> {
    let is_app_command = is_app_command_endpoint(sig);
    let is_internal = args.internal || is_app_command;
    let has_app_state = exprs_have_app_state_predicate(&args.requires);

    if is_internal && has_app_state {
        let message = if is_app_command {
            "AppCommand endpoints must never be gated on application state."
        } else {
            "Internal protocol endpoints must never be gated on application state."
        };
        return Err(syn::Error::new_spanned(&sig.ident, message));
    }

    let mut exprs = args.requires.clone();

    if !is_internal && !has_app_state {
        if exprs.is_empty() {
            let default_guard = match kind {
                EndpointKind::Update => DefaultAppGuard::AllowsUpdates,
                EndpointKind::Query => DefaultAppGuard::IsQueryable,
            };
            return Ok(AccessPlan::DefaultApp(default_guard));
        }

        let injected = match kind {
            EndpointKind::Update => BuiltinPredicate::AppAllowsUpdates,
            EndpointKind::Query => BuiltinPredicate::AppIsQueryable,
        };
        exprs.push(AccessExprAst::Pred(AccessPredicateAst::Builtin(injected)));
    }

    if exprs.is_empty() {
        return Ok(AccessPlan::None);
    }

    let exprs: Vec<_> = exprs.iter().map(expr_from_ast).collect();

    Ok(AccessPlan::Expr(quote! {
        ::canic::__internal::core::access::expr::AccessExpr::All(vec![#(#exprs),*])
    }))
}

fn expr_from_ast(expr: &AccessExprAst) -> TokenStream2 {
    match expr {
        AccessExprAst::All(exprs) => {
            let items = exprs.iter().map(expr_from_ast);
            quote!(::canic::__internal::core::access::expr::AccessExpr::All(
                vec![#(#items),*]
            ))
        }
        AccessExprAst::Any(exprs) => {
            let items = exprs.iter().map(expr_from_ast);
            quote!(::canic::__internal::core::access::expr::AccessExpr::Any(
                vec![#(#items),*]
            ))
        }
        AccessExprAst::Not(expr) => {
            let inner = expr_from_ast(expr);
            quote!(::canic::__internal::core::access::expr::AccessExpr::Not(Box::new(#inner)))
        }
        AccessExprAst::Pred(pred) => match pred {
            AccessPredicateAst::Builtin(builtin) => expr_from_builtin(builtin),
            AccessPredicateAst::Custom(expr) => {
                quote!(::canic::__internal::core::access::expr::custom(#expr))
            }
        },
    }
}

fn expr_from_builtin(pred: &BuiltinPredicate) -> TokenStream2 {
    match pred {
        BuiltinPredicate::AppAllowsUpdates => {
            quote!(::canic::__internal::core::access::expr::app::allows_updates())
        }
        BuiltinPredicate::AppIsQueryable => {
            quote!(::canic::__internal::core::access::expr::app::is_queryable())
        }
        BuiltinPredicate::SelfIsPrimeSubnet => {
            quote!(::canic::__internal::core::access::expr::env::is_prime_subnet())
        }
        BuiltinPredicate::SelfIsPrimeRoot => {
            quote!(::canic::__internal::core::access::expr::env::is_prime_root())
        }
        BuiltinPredicate::CallerIsController => {
            quote!(::canic::__internal::core::access::expr::caller::is_controller())
        }
        BuiltinPredicate::CallerIsParent => {
            quote!(::canic::__internal::core::access::expr::caller::is_parent())
        }
        BuiltinPredicate::CallerIsChild => {
            quote!(::canic::__internal::core::access::expr::caller::is_child())
        }
        BuiltinPredicate::CallerIsRoot => {
            quote!(::canic::__internal::core::access::expr::caller::is_root())
        }
        BuiltinPredicate::CallerIsSameCanister => {
            quote!(::canic::__internal::core::access::expr::caller::is_same_canister())
        }
        BuiltinPredicate::CallerIsRegisteredToSubnet => {
            quote!(::canic::__internal::core::access::expr::caller::is_registered_to_subnet())
        }
        BuiltinPredicate::CallerIsWhitelisted => {
            quote!(::canic::__internal::core::access::expr::caller::is_whitelisted())
        }
        BuiltinPredicate::Authenticated { required_scope } => match required_scope {
            Some(AuthScopeArg::Literal(required_scope)) => quote!(
                ::canic::__internal::core::access::expr::auth::authenticated_with_scope(
                    #required_scope
                )
            ),
            Some(AuthScopeArg::Expr(required_scope)) => quote!(
                ::canic::__internal::core::access::expr::auth::authenticated_with_scope(
                    #required_scope
                )
            ),
            None => quote!(
                ::canic::__internal::core::access::expr::auth::authenticated(
                    ::core::option::Option::None
                )
            ),
        },
        BuiltinPredicate::BuildIcOnly => {
            quote!(::canic::__internal::core::access::expr::env::build_ic_only())
        }
        BuiltinPredicate::BuildLocalOnly => {
            quote!(::canic::__internal::core::access::expr::env::build_local_only())
        }
    }
}

fn exprs_have_app_state_predicate(exprs: &[AccessExprAst]) -> bool {
    exprs.iter().any(expr_has_app_state_predicate)
}

fn requires_authenticated(exprs: &[AccessExprAst]) -> bool {
    exprs.iter().any(expr_has_authenticated_predicate)
}

fn expr_has_authenticated_predicate(expr: &AccessExprAst) -> bool {
    match expr {
        AccessExprAst::All(exprs) | AccessExprAst::Any(exprs) => {
            exprs.iter().any(expr_has_authenticated_predicate)
        }
        AccessExprAst::Not(expr) => expr_has_authenticated_predicate(expr),
        AccessExprAst::Pred(pred) => match pred {
            AccessPredicateAst::Builtin(builtin) => {
                matches!(builtin, BuiltinPredicate::Authenticated { .. })
            }
            AccessPredicateAst::Custom(_) => false,
        },
    }
}

fn first_typed_arg_ident(sig: &Signature) -> Option<syn::Ident> {
    let first = sig.inputs.first()?;
    let syn::FnArg::Typed(pat) = first else {
        return None;
    };
    let syn::Pat::Ident(id) = &*pat.pat else {
        return None;
    };
    Some(id.ident.clone())
}

fn expr_has_app_state_predicate(expr: &AccessExprAst) -> bool {
    match expr {
        AccessExprAst::All(exprs) | AccessExprAst::Any(exprs) => {
            exprs.iter().any(expr_has_app_state_predicate)
        }
        AccessExprAst::Not(expr) => expr_has_app_state_predicate(expr),
        AccessExprAst::Pred(pred) => match pred {
            AccessPredicateAst::Builtin(builtin) => builtin_is_app_state(builtin),
            AccessPredicateAst::Custom(tokens) => custom_has_app_state_is(tokens),
        },
    }
}

const fn builtin_is_app_state(pred: &BuiltinPredicate) -> bool {
    matches!(
        pred,
        BuiltinPredicate::AppAllowsUpdates | BuiltinPredicate::AppIsQueryable
    )
}

fn custom_has_app_state_is(tokens: &TokenStream2) -> bool {
    let Ok(expr) = syn::parse2::<syn::Expr>(tokens.clone()) else {
        return false;
    };
    let mut visitor = AppStateVisitor { found: false };
    visitor.visit_expr(&expr);
    visitor.found
}

///
/// AppStateVisitor
///

struct AppStateVisitor {
    found: bool,
}

impl Visit<'_> for AppStateVisitor {
    fn visit_path(&mut self, path: &syn::Path) {
        if path.segments.iter().any(|seg| seg.ident == "AppStateIs") {
            self.found = true;
            return;
        }
        syn::visit::visit_path(self, path);
    }
}

fn is_app_command_endpoint(sig: &Signature) -> bool {
    sig.inputs.iter().any(|input| match input {
        syn::FnArg::Typed(pat) => type_has_app_command(&pat.ty),
        syn::FnArg::Receiver(_) => true,
    })
}

fn type_has_app_command(ty: &Type) -> bool {
    match ty {
        Type::Path(path) => path_has_app_command(&path.path),
        Type::Reference(reference) => type_has_app_command(&reference.elem),
        Type::Group(group) => type_has_app_command(&group.elem),
        Type::Paren(paren) => type_has_app_command(&paren.elem),
        Type::Tuple(tuple) => tuple.elems.iter().any(type_has_app_command),
        _ => false,
    }
}

fn path_has_app_command(path: &syn::Path) -> bool {
    path.segments.iter().any(|seg| {
        if seg.ident == "AppCommand" {
            return true;
        }

        match &seg.arguments {
            PathArguments::AngleBracketed(args) => args.args.iter().any(|arg| match arg {
                GenericArgument::Type(ty) => type_has_app_command(ty),
                _ => false,
            }),
            _ => false,
        }
    })
}

//
// ============================================================================
// dispatch + completion
// ============================================================================
//

fn dispatch_call(
    wrapper_async: bool,
    impl_async: bool,
    dispatch: TokenStream2,
    call: &syn::Ident,
    impl_name: syn::Ident,
    args: &[TokenStream2],
) -> TokenStream2 {
    if wrapper_async {
        if impl_async {
            quote! {
                #dispatch(#call, || async move {
                    #impl_name(#(#args),*).await
                }).await
            }
        } else {
            quote! {
                #dispatch(#call, || async move {
                    #impl_name(#(#args),*)
                }).await
            }
        }
    } else {
        quote! {
            #dispatch(#call, || {
                #impl_name(#(#args),*)
            })
        }
    }
}

fn extract_args(sig: &syn::Signature) -> syn::Result<Vec<TokenStream2>> {
    let mut out = Vec::new();
    for input in &sig.inputs {
        match input {
            syn::FnArg::Typed(pat) => match &*pat.pat {
                syn::Pat::Ident(id) => out.push(quote!(#id)),
                _ => {
                    return Err(syn::Error::new_spanned(
                        &pat.pat,
                        "destructuring parameters not supported",
                    ));
                }
            },
            syn::FnArg::Receiver(r) => {
                return Err(syn::Error::new_spanned(
                    r,
                    "`self` not supported in canic endpoints",
                ));
            }
        }
    }
    Ok(out)
}

fn cdk_attr(kind: EndpointKind, forwarded: &[TokenStream2]) -> TokenStream2 {
    match kind {
        EndpointKind::Query => {
            if forwarded.is_empty() {
                quote!(#[::canic::cdk::query])
            } else {
                quote!(#[::canic::cdk::query(#(#forwarded),*)])
            }
        }
        EndpointKind::Update => {
            if forwarded.is_empty() {
                quote!(#[::canic::cdk::update])
            } else {
                quote!(#[::canic::cdk::update(#(#forwarded),*)])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_args(requires: Vec<AccessExprAst>) -> ValidatedArgs {
        ValidatedArgs {
            forwarded: Vec::new(),
            export_name: None,
            payload_max_bytes: None,
            requires,
            internal: false,
        }
    }

    #[test]
    fn endpoint_expansion_omits_removed_endpoint_metric_hooks() {
        let args = make_args(Vec::new());
        let func: ItemFn = syn::parse_quote!(
            fn ping() -> Result<(), ::canic::Error> {
                Ok(())
            }
        );

        let expanded = expand(EndpointKind::Query, args, func).to_string();

        assert!(!expanded.contains("EndpointAttemptMetrics"));
        assert!(!expanded.contains("EndpointResultMetrics"));
    }

    #[test]
    fn update_expansion_registers_payload_limit_for_exported_name() {
        let mut args = make_args(Vec::new());
        args.export_name = Some(syn::LitStr::new(
            "wire_ping",
            proc_macro2::Span::call_site(),
        ));
        args.payload_max_bytes = Some(quote!(64 * 1024));
        let func: ItemFn = syn::parse_quote!(
            fn ping() -> Result<(), ::canic::Error> {
                Ok(())
            }
        );

        let expanded = expand(EndpointKind::Update, args, func).to_string();

        assert!(expanded.contains("register_update_limit"));
        assert!(expanded.contains("\"wire_ping\""));
        assert!(expanded.contains("64 * 1024"));
    }

    #[test]
    fn default_app_guard_keeps_sync_wrapper_sync() {
        let sig: Signature = syn::parse_quote!(fn ping() -> Result<(), ::canic::Error>);
        let args = make_args(Vec::new());
        let plan = build_access_plan(EndpointKind::Update, &args, &sig).expect("access plan");

        assert!(!plan.requires_async());
        assert!(!(sig.asyncness.is_some() || plan.requires_async()));
    }

    #[test]
    fn explicit_requires_forces_async_wrapper() {
        let sig: Signature = syn::parse_quote!(fn ping() -> Result<(), ::canic::Error>);
        let args = make_args(vec![AccessExprAst::Pred(AccessPredicateAst::Builtin(
            BuiltinPredicate::CallerIsController,
        ))]);
        let plan = build_access_plan(EndpointKind::Update, &args, &sig).expect("access plan");

        assert!(plan.requires_async());
        assert!(sig.asyncness.is_some() || plan.requires_async());
    }

    #[test]
    fn app_command_endpoints_skip_app_guard_and_reject_gating() {
        let sig: Signature = syn::parse_quote!(
            fn apply(cmd: ::canic::dto::state::AppCommand) -> Result<(), ::canic::Error>
        );

        let args = make_args(Vec::new());
        let plan = build_access_plan(EndpointKind::Update, &args, &sig).expect("access plan");
        assert!(matches!(plan, AccessPlan::None));

        let args = make_args(vec![AccessExprAst::Pred(AccessPredicateAst::Builtin(
            BuiltinPredicate::AppAllowsUpdates,
        ))]);
        let err = build_access_plan(EndpointKind::Update, &args, &sig).unwrap_err();
        assert!(
            err.to_string()
                .contains("AppCommand endpoints must never be gated on application state.")
        );
    }

    #[test]
    fn access_stage_expr_builds_context_from_resolved_identity() {
        let sig: Signature = syn::parse_quote!(fn ping() -> Result<(), ::canic::Error>);
        let args = make_args(vec![AccessExprAst::Pred(AccessPredicateAst::Builtin(
            BuiltinPredicate::CallerIsController,
        ))]);
        let plan = build_access_plan(EndpointKind::Update, &args, &sig).expect("access plan");
        let call = format_ident!("__canic_call");
        let stage = access_stage(&plan, &call).to_string();
        let compact = stage.split_whitespace().collect::<String>();

        assert!(compact.contains("resolve_authenticated_identity("));
        assert!(compact.contains("caller:__canic_authenticated_identity.transport_caller"));
        assert!(
            compact.contains(
                "authenticated_caller:__canic_authenticated_identity.authenticated_subject"
            )
        );
        assert!(compact.contains("identity_source:__canic_authenticated_identity.identity_source"));
    }

    #[test]
    fn access_stage_default_guard_marks_identity_source_raw_caller() {
        let sig: Signature = syn::parse_quote!(fn ping() -> Result<(), ::canic::Error>);
        let args = make_args(Vec::new());
        let plan = build_access_plan(EndpointKind::Update, &args, &sig).expect("access plan");
        let call = format_ident!("__canic_call");
        let stage = access_stage(&plan, &call).to_string();
        let compact = stage.split_whitespace().collect::<String>();

        assert!(compact.contains("identity_source::canic::__internal::core::access::auth::AuthenticatedIdentitySource::RawCaller")
            || compact.contains("identity_source:::canic::__internal::core::access::auth::AuthenticatedIdentitySource::RawCaller"));
    }
}
