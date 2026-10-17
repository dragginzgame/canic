//! Emit service endpoints into the owning canister's Candid and dispatch scope.

/// Host attributes used by the shared service declaration owner.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_blob_endpoint {
    ($d:tt) => {
        // A lexical emitter registers methods before Canic exports Candid.
        // An imported macro alias can defer those registrations past the export.
        macro_rules! endpoint {
            (query, request, $d($d item:tt)*) => {
                #[canic::canic_query(public, on_access_denied = "reject", decode = REQUEST_LIMITS)]
                $d($d item)*
            };
            (update, request, $d($d item:tt)*) => {
                #[canic::canic_update(public, on_access_denied = "reject", decode = REQUEST_LIMITS)]
                $d($d item)*
            };
            (update, manifest, $d($d item:tt)*) => {
                #[canic::canic_update(public, on_access_denied = "reject", decode = MANIFEST_LIMITS)]
                $d($d item)*
            };
            (certificate, request, $d wire_name:literal, $d($d item:tt)*) => {
                #[canic::canic_update(public, on_access_denied = "reject", decode = REQUEST_LIMITS, name = $d wire_name)]
                $d($d item)*
            };
        }
        $crate::__private::ic_blob_storage_contracts::mount_service_endpoints!(
            emit = endpoint,
            workflow = workflow,
            context = context,
            contracts = ic_blob_storage_contracts,
            clock = ic_cdk::api::time,
            caller = ic_cdk::api::msg_caller,
            trap = ic_cdk::trap,
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
            pub(super) use $crate::__private::{ic_blob_storage_contracts, ic_cdk};
            use $crate::{
                lifecycle::{MANIFEST_LIMITS, REQUEST_LIMITS},
                workflow,
            };
            fn context() -> ic_blob_storage_contracts::upload::binding::UploadContext {
                ic_blob_storage_contracts::upload::binding::UploadContext {
                    service: ic_cdk::api::canister_self(),
                    actor: ic_cdk::api::msg_caller(),
                }
            }
            $crate::__canic_blob_endpoint!($);
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
