//! Module: icrc
//!
//! Responsibility: explicitly mount external ICRC discovery and optional consent endpoints.
//! Does not own: Canic lifecycle, configuration, ledger execution, or application consent policy.
//! Boundary: opt-in Candid adapter; owned application RPC remains IC Wire.
//!
//! Enable the `icrc` Cargo feature, then invoke `canic::icrc::mount!(limits = LIMITS)`
//! at module scope before `canic::finish!()`. Add `consent = handler` to mount
//! ICRC-21 alongside ICRC-10. The synchronous handler receives the exact upstream
//! request and returns its upstream result. It owns method admission, argument
//! interpretation, language/display support and consent policy. It must not execute
//! the requested application operation. Canic installs no handler registry.
//!
//! Limits are explicit `canic::endpoint::ArgumentLimits`; configuration and
//! `canic::start!` do not select or mount these methods. Mounting retains external
//! Candid with Platform's bounded external decoding, including its nesting ceiling.

pub use crate::__canic_mount_icrc as mount;

pub use ic_platform::icrc_ledger_types::icrc10::SupportedStandard;
pub use ic_platform::icrc_ledger_types::icrc21::{ConsentInfo, ConsentMessageRequest, Icrc21Error};

/// Project only the standards selected by this adapter's expansion.
#[doc(hidden)]
#[must_use]
pub fn supported_standards(consent: bool) -> Vec<SupportedStandard> {
    let mut standards = vec![SupportedStandard {
        name: "ICRC-10".to_string(),
        url: "https://github.com/dfinity/ICRC/blob/main/ICRCs/ICRC-10/ICRC-10.md".to_string(),
    }];
    if consent {
        standards.push(SupportedStandard {
            name: "ICRC-21".to_string(),
            url: "https://github.com/dfinity/ICRC/blob/main/ICRCs/ICRC-21/ICRC-21.md".to_string(),
        });
    }
    standards
}

/// Mount discovery, optionally with a caller-owned synchronous consent handler.
/// Discovery is a public query; consent is a public update, both with explicit limits.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_mount_icrc {
    (limits = $limits:path $(,)?) => {
        $crate::__canic_icrc_discovery!(limits = $limits, consent = false);
    };
    (limits = $limits:path, consent = $handler:path $(,)?) => {
        $crate::__canic_icrc_discovery!(limits = $limits, consent = true);
        #[$crate::candid_update(internal, public, on_access_denied = "reject", decode = $limits)]
        fn icrc21_canister_call_consent_message(
            request: ::canic::icrc::ConsentMessageRequest,
        ) -> ::std::result::Result<::canic::icrc::ConsentInfo, ::canic::icrc::Icrc21Error> {
            $handler(request)
        }
    };
}

/// Emit the discovery endpoint for one exact adapter selection.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_icrc_discovery {
    (limits = $limits:path, consent = $consent:literal) => {
        #[$crate::candid_query(internal, public, on_access_denied = "reject", decode = $limits)]
        pub fn icrc10_supported_standards() -> ::std::vec::Vec<::canic::icrc::SupportedStandard> {
            $crate::icrc::supported_standards($consent)
        }
    };
}
