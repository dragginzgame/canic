//! Module: macros::endpoints::standards
//!
//! Responsibility: emit separately mandated standards endpoints.
//! Does not own: Canic role commands, role status, or application endpoints.
//! Boundary: standards remain named methods outside the Canic method budget.

/// Emit the ICRC standards-facing query endpoints selected for this profile.
#[macro_export]
macro_rules! canic_emit_icrc_standards_endpoints {
    () => {
        #[$crate::canic_query(internal, public)]
        pub fn icrc10_supported_standards() -> Vec<(String, String)> {
            $crate::__internal::core::api::icrc::Icrc10Query::supported_standards()
        }

        #[cfg(canic_icrc21_enabled)]
        #[$crate::canic_query(internal, public)]
        async fn icrc21_canister_call_consent_message(
            req: ::canic::dto::icrc21::ConsentMessageRequest,
        ) -> ::canic::dto::icrc21::ConsentMessageResponse {
            $crate::__internal::core::api::icrc::Icrc21Query::consent_message(req)
        }
    };
}
