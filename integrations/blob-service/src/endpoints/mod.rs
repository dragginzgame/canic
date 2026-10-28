//! Emit owned raw endpoints and the external provider Candid contract.

/// Host attributes used by the shared service declaration owner.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_blob_endpoint {
    ($d:tt) => {
        // A lexical emitter registers the provider contract before Candid export.
        // An imported macro alias can defer those registrations past the export.
        macro_rules! decode {
            (tenant_inspection, $d method:expr, $d bytes:expr) => { ic_blob_storage_contracts::wire::tenant::decode_inspection_request($d bytes).expect("tenant inspection") };
            (tenant_update, $d method:expr, $d bytes:expr) => { ic_blob_storage_contracts::wire::tenant::decode_update_request($d bytes).expect("tenant update") };
            ($d category:ident, $d method:expr, $d bytes:expr) => { ic_blob_storage_contracts::wire::decode_request($d method, $d bytes, ic_blob_storage_contracts::wire::request_limit($d method).expect("Blob request limit")).expect("Blob Wire request") };
        }
        macro_rules! encode {
            (tenant_inspection, $d method:expr, $d value:expr) => { ic_blob_storage_contracts::wire::tenant::inspection_response($d value).expect("tenant inspection reply") };
            (tenant_update, $d method:expr, $d value:expr) => { ic_blob_storage_contracts::wire::tenant::update_response($d value).expect("tenant update reply") };
            ($d category:ident, $d method:expr, $d value:expr) => { ic_blob_storage_contracts::wire::encode_response($d method, $d value).expect("Blob Wire reply") };
        }
        macro_rules! arguments {
            ($d category:ident, $d method:expr;) => { let _: () = decode!($d category, $d method, &$crate::__private::transport::arguments(ic_blob_storage_contracts::wire::request_limit($d method).expect("Blob request limit"))); };
            ($d category:ident, $d method:expr; $d arg:ident : $d ty:ty) => { let $d arg: $d ty = decode!($d category, $d method, &$crate::__private::transport::arguments(ic_blob_storage_contracts::wire::request_limit($d method).expect("Blob request limit"))); };
        }
        macro_rules! gc_arguments {
            (roots; $d arg:ident : $d ty:ty) => {
                let $d arg: $d ty = ic_blob_storage_contracts::provider::gc::decode_roots(
                    &$crate::__private::transport::arguments(ic_blob_storage_contracts::provider::gc::MAX_REQUEST_BYTES)
                ).expect("bounded provider roots");
            };
            (empty;) => {
                ic_blob_storage_contracts::provider::gc::decode_empty(
                    &$crate::__private::transport::arguments(ic_blob_storage_contracts::provider::gc::MAX_REQUEST_BYTES)
                ).expect("empty provider request");
            };
        }
        macro_rules! describe {
            (IcWire, $d name:ident, $d spec:expr) => {
                mod $d name {
                    use super::ic_blob_storage_contracts;
                    pub const CONTRACT: ic_blob_storage_contracts::endpoints::WireContract =
                        match ($d spec).wire() {
                            Ok(contract) => contract,
                            Err(_) => panic!("Blob owned Wire descriptor"),
                        };
                    pub const ID: u64 = CONTRACT.id;
                }
            };
            (Candid, $d name:ident, $d spec:expr) => {};
        }
        macro_rules! endpoint {
            (provider, query, $d category:ident, $d wire:literal, fn $d name:ident($d($d arg:ident : $d ty:ty),*) -> $d reply:ty $d body:block) => {
                mod $d name {
                    use super::*;
                    #[cfg_attr(not(target_arch = "wasm32"), ic_platform::candid::contract::candid_method(query, rename = $d wire))]
                    fn $d name($d($d arg: $d ty),*) -> $d reply $d body
                    #[canic::canic_raw_query(public, name = $d wire, payload(max_bytes = ic_blob_storage_contracts::provider::gc::MAX_REQUEST_BYTES))]
                    fn dispatch() {
                        gc_arguments!($d category; $d($d arg: $d ty),*);
                        let response = $d name($d($d arg),*);
                        ic_platform::cdk::api::msg_reply(ic_platform::candid::contract::encode_one(response).expect("provider reply"));
                    }
                }
            };
            (provider, update, roots, $d wire:literal, async fn $d name:ident($d arg:ident : $d ty:ty) $d body:block) => {
                mod $d name {
                    use super::*;
                    #[cfg_attr(not(target_arch = "wasm32"), ic_platform::candid::contract::candid_method(update, rename = $d wire))]
                    async fn $d name($d arg: $d ty) $d body
                    #[canic::canic_raw_update(public, name = $d wire, payload(max_bytes = ic_blob_storage_contracts::provider::gc::MAX_REQUEST_BYTES))]
                    async fn dispatch() {
                        gc_arguments!(roots; $d arg: $d ty);
                        $d name($d arg).await;
                        ic_platform::cdk::api::msg_reply(ic_platform::candid::contract::encode_args(()).expect("provider acknowledgment reply"));
                    }
                }
            };
            (query, $d category:ident, fn $d name:ident($d($d arg:ident : $d ty:ty),* $d(,)?) -> $d reply:ty $d body:block) => {
                #[canic::canic_raw_query(public, wire(id = $d name::ID, request = $d name::CONTRACT.request, response = $d name::CONTRACT.response))]
                fn $d name() {
                    arguments!($d category, stringify!($d name); $d($d arg : $d ty),*);
                    let response: $d reply = (|| $d body)();
                    ic_platform::cdk::api::msg_reply(encode!($d category, stringify!($d name), response));
                }
            };
            (update, $d category:ident, async fn blob_admit_upload($d($d arg:ident : $d ty:ty),* $d(,)?) -> $d reply:ty $d body:block) => {
                endpoint!(@observed $d category, blob_admit_upload, admission, ($d($d arg : $d ty),*), $d reply, $d body);
            };
            (update, $d category:ident, async fn blob_apply_reference($d($d arg:ident : $d ty:ty),* $d(,)?) -> $d reply:ty $d body:block) => {
                endpoint!(@observed $d category, blob_apply_reference, reference, ($d($d arg : $d ty),*), $d reply, $d body);
            };
            (@observed $d category:ident, $d name:ident, $d observe:ident, ($d($d arg:ident : $d ty:ty),*), $d reply:ty, $d body:block) => {
                #[canic::canic_raw_update(public, wire(id = $d name::ID, request = $d name::CONTRACT.request, response = $d name::CONTRACT.response))]
                async fn $d name() {
                    arguments!($d category, stringify!($d name); $d($d arg : $d ty),*);
                    let bytes = u32::try_from(ic_platform::ic0::msg_arg_data_size()).expect("bounded Blob argument size");
                    let started = ic_platform::cdk::api::call_context_instruction_counter();
                    let response: $d reply = (async move $d body).await;
                    let instructions = ic_platform::cdk::api::call_context_instruction_counter().saturating_sub(started);
                    $crate::__private::operation_metrics::$d observe(&response, instructions, bytes);
                    ic_platform::cdk::api::msg_reply(encode!($d category, stringify!($d name), response));
                }
            };
            (update, $d category:ident, async fn $d name:ident($d($d arg:ident : $d ty:ty),* $d(,)?) -> $d reply:ty $d body:block) => {
                #[canic::canic_raw_update(public, wire(id = $d name::ID, request = $d name::CONTRACT.request, response = $d name::CONTRACT.response))]
                async fn $d name() {
                    arguments!($d category, stringify!($d name); $d($d arg : $d ty),*);
                    let response: $d reply = (async move $d body).await;
                    ic_platform::cdk::api::msg_reply(encode!($d category, stringify!($d name), response));
                }
            };
            (certificate, request, $d wire_name:literal, $d($d item:tt)*) => {
                #[canic::canic_update(public, on_access_denied = "reject", decode = REQUEST_LIMITS, name = $d wire_name)]
                $d($d item)*
            };
        }
        $crate::__private::ic_blob_storage_contracts::mount_service_endpoints!(
            emit = endpoint,
            describe = describe,
            workflow = workflow,
            context = context,
            contracts = ic_blob_storage_contracts,
            clock = ic_platform::cdk::api::time,
            caller = caller,
            trap = ic_platform::cdk::api::trap,
        );
    };
}

/// Mount blob endpoints inside an application canister.
///
/// Invoke once at crate root after selecting the service namespace in the host
/// allocation pool and adding [`crate::memory_requests`] to its explicit host
/// composition. The application retains its sole `canic::start!`
/// and `canic::finish!`, composes synchronous [`crate::lifecycle`] functions and
/// includes [`crate::metrics::sample`] in its application sampler. This macro
/// neither installs the service nor registers a lifecycle or sampler.
#[macro_export]
macro_rules! mount {
    () => {

        mod __canic_blob_endpoints {
            pub(super) use $crate::__private::{ic_blob_storage_contracts, ic_platform};
            use $crate::{
                lifecycle::REQUEST_LIMITS,
                __private::Workflow as workflow,
            };
            fn caller() -> ic_platform::Principal {
                ic_platform::cdk::api::msg_caller()
            }
            fn context() -> ic_blob_storage_contracts::upload::binding::UploadContext {
                ic_blob_storage_contracts::upload::binding::UploadContext {
                    service: ic_platform::cdk::api::canister_self(),
                    actor: caller(),
                }
            }
            $crate::__canic_blob_endpoint!($);
            $crate::__canic_blob_schema!();
        }

        // Candid's declaration pass resolves every endpoint type in this scope.
        #[allow(unexpected_cfgs, reason = "the owning Canic build registers this cfg")]
        #[cfg(canic_export_candid)]
        use __canic_blob_endpoints::*;
    };
}

/// Compose a dedicated managed blob canister using the same embeddable parts.
///
/// The consumer owns build configuration and exact App/role metadata. This
/// convenience macro grants the service namespace, installs the blob-only sampler,
/// and supplies the sole lifecycle and Candid export. Use [`crate::mount!`] when
/// an application already owns those surfaces.
#[macro_export]
macro_rules! canister {
    () => {
        $crate::mount!();
        canic::start!(
            memory = $crate::__private::memory::host(),
            argument_limits = $crate::lifecycle::ENVELOPE_LIMITS,
            lifecycle_participant(
                init = $crate::lifecycle::install_from_args,
                post_upgrade = __canic_blob_restore
            ),
        );

        fn __canic_blob_restore() {
            $crate::lifecycle::restore();
            $crate::lifecycle::register_blob_sampler();
        }
        #[expect(clippy::unused_async, reason = "Canic deferred lifecycle signature")]
        async fn canic_setup() {}
        async fn canic_install(_: Option<Vec<u8>>) {}
        #[expect(
            clippy::unused_async,
            reason = "restoration is synchronous in the lifecycle participant"
        )]
        async fn canic_upgrade() {}

        canic::finish!();
    };
}
