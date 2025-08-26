pub mod root;

// icu_endpoints
#[macro_export]
macro_rules! icu_endpoints {
    () => {
        //
        // IC API ENDPOINTS (IMPORTANT!!)
        // these are specific endpoints defined by the IC spec
        //

        // ic_cycles_accept
        #[::icu::cdk::query]
        fn ic_cycles_accept(max_amount: u128) -> u128 {
            $crate::cdk::api::msg_cycles_accept(max_amount)
        }

        // icu_canister_upgrade_children
        // canister_id : None means upgrade all children
        #[::icu::cdk::update]
        async fn icu_canister_upgrade_children(
            canister_id: Option<::candid::Principal>,
        ) -> Result<
            Vec<Result<::icu::ops::response::UpgradeCanisterResponse, ::icu::Error>>,
            ::icu::Error,
        > {
            $crate::auth_require_any!(::icu::auth::is_controller)?;

            let mut results = Vec::new();

            for (child_pid, _) in $crate::memory::CanisterChildren::export() {
                if canister_id.is_none() || canister_id == Some(child_pid) {
                    // Push the result (either Ok(resp) or Err(err)) into the vec
                    let result = $crate::ops::request::upgrade_canister_request(child_pid).await;
                    results.push(result);
                }
            }

            Ok(results)
        }

        #[::icu::cdk::update]
        async fn icu_state_update(
            bundle: ::icu::ops::state::StateBundle,
        ) -> Result<(), ::icu::Error> {
            $crate::auth_require_any!(::icu::auth::is_parent)?;

            $crate::ops::state::save_state(&bundle);

            Ok(())
        }

        #[::icu::cdk::update]
        async fn icu_state_cascade(
            bundle: ::icu::ops::state::StateBundle,
        ) -> Result<(), ::icu::Error> {
            $crate::auth_require_any!(::icu::auth::is_parent)?;

            $crate::ops::state::save_state(&bundle);
            $crate::ops::state::cascade(&bundle).await
        }

        //
        // ICRC ENDPOINTS
        //

        #[::icu::cdk::query]
        pub fn icrc10_supported_standards() -> Vec<(String, String)> {
            $crate::state::icrc::Icrc10Registry::supported_standards()
        }

        #[::icu::cdk::query]
        async fn icrc21_canister_call_consent_message(
            req: ::icu::spec::icrc::icrc21::ConsentMessageRequest,
        ) -> ::icu::spec::icrc::icrc21::ConsentMessageResponse {
            $crate::state::icrc::Icrc21Registry::consent_message(req)
        }

        //
        // ICU CANISTER HELPERS
        //

        #[::icu::cdk::query]
        fn icu_canister_cycle_balance() -> u128 {
            $crate::cdk::api::canister_cycle_balance()
        }

        #[::icu::cdk::query]
        fn icu_canister_version() -> u64 {
            $crate::cdk::api::canister_version()
        }

        #[::icu::cdk::query]
        fn icu_time() -> u64 {
            $crate::cdk::api::time()
        }

        //
        // ICU MEMORY HELPERS
        //

        #[::icu::cdk::query]
        fn icu_app_state() -> ::icu::memory::AppStateData {
            $crate::memory::AppState::export()
        }

        #[::icu::cdk::query]
        fn icu_canister_children() -> ::icu::memory::CanisterChildrenView {
            $crate::memory::CanisterChildren::export()
        }

        #[::icu::cdk::query]
        fn icu_canister_state() -> ::icu::memory::CanisterStateData {
            $crate::memory::CanisterState::export()
        }

        #[::icu::cdk::query]
        fn icu_canister_directory() -> ::icu::memory::CanisterDirectoryView {
            $crate::memory::CanisterDirectory::export()
        }

        #[::icu::cdk::query]
        fn icu_cycle_tracker() -> ::icu::memory::CycleTrackerView {
            $crate::memory::CycleTracker::export()
        }

        //
        // ICU DELEGATION ENDPOINTS
        //

        #[::icu::cdk::query]
        async fn icu_delegation_get(
            session_pid: Principal,
        ) -> Result<::icu::state::delegation::DelegationSessionView, ::icu::Error> {
            $crate::state::delegation::DelegationRegistry::get(session_pid)
        }

        #[::icu::cdk::update]
        async fn icu_delegation_track(
            session_pid: Principal,
        ) -> Result<::icu::state::delegation::DelegationSessionView, ::icu::Error> {
            $crate::state::delegation::DelegationRegistry::track(msg_caller(), session_pid)
        }

        #[::icu::cdk::update]
        async fn icu_delegation_register(
            args: ::icu::state::delegation::RegisterSessionArgs,
        ) -> Result<(), ::icu::Error> {
            $crate::auth_require_any!(::icu::auth::is_whitelisted)?;

            $crate::state::delegation::DelegationRegistry::register_session(msg_caller(), args)
        }

        #[::icu::cdk::update]
        async fn icu_delegation_revoke(pid: Principal) -> Result<(), ::icu::Error> {
            use ::icu::auth::{is_parent, is_principal};

            // make sure the caller == pid to revoke
            // or a parent canister
            let expected = pid;
            $crate::auth_require_any!(is_parent, |caller| is_principal(caller, expected))?;

            $crate::state::delegation::DelegationRegistry::revoke_session_or_wallet(pid)
        }

        //
        // ICTS ENDPOINTS
        //

        #[::icu::cdk::query]
        fn icts_name() -> String {
            env!("CARGO_PKG_NAME").to_string()
        }

        #[::icu::cdk::query]
        fn icts_version() -> String {
            env!("CARGO_PKG_VERSION").to_string()
        }

        #[::icu::cdk::query]
        fn icts_description() -> String {
            env!("CARGO_PKG_DESCRIPTION").to_string()
        }

        #[::icu::cdk::query]
        fn icts_metadata() -> Vec<(String, String)> {
            vec![
                ("name".to_string(), icts_name()),
                ("version".to_string(), icts_version()),
                ("description".to_string(), icts_description()),
            ]
        }

        #[::icu::cdk::update]
        async fn icts_canister_status()
        -> Result<::icu::cdk::management_canister::CanisterStatusResult, String> {
            use $crate::cdk::{
                api::canister_self,
                management_canister::{CanisterStatusArgs, canister_status},
            };

            if &msg_caller().to_string() != "ylse7-raaaa-aaaal-qsrsa-cai" {
                return Err("Unauthorized".to_string());
            }

            canister_status(&CanisterStatusArgs {
                canister_id: canister_self(),
            })
            .await
            .map_err(|e| e.to_string())
        }
    };
}
