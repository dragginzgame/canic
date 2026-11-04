//! Installed explicit ICRC selections composed with the ordinary managed lifecycle.

use canic::{
    Error, dto::fleet_admission::FleetAdmissionProjectionPhase, testing::ManagedAppFixture,
};
use canic_tests::pic::{
    application_probe,
    icrc::{self, MountProfile},
};
use ic_host_artifacts::wasm::{ExportKind, InspectionLimits};
use ic_platform::{
    candid::{
        contract::{encode_args, encode_one},
        decoder::decode_one,
    },
    icrc_ledger_types::{
        icrc10::SupportedStandard,
        icrc21::{
            ConsentInfo, ConsentMessage, ConsentMessageMetadata, ConsentMessageRequest,
            ConsentMessageSpec, DisplayMessageType, Icrc21Error,
        },
    },
    principal::ExternalPrincipal as Principal,
};
use ic_testkit::pocket_ic::{ClientError, ErrorCode};
use std::{collections::BTreeSet, time::Duration};

const DISCOVERY: &str = "icrc10_supported_standards";
const CONSENT: &str = "icrc21_canister_call_consent_message";

fn activate(fixture: &ManagedAppFixture) {
    assert_eq!(
        fixture.admission_status().unwrap().phase,
        FleetAdmissionProjectionPhase::Fenced
    );
    fixture.configure_and_wait_until_active(40).unwrap();
    assert_eq!(
        fixture.admission_status().unwrap().phase,
        FleetAdmissionProjectionPhase::Open
    );
}

fn request() -> ConsentMessageRequest {
    ConsentMessageRequest {
        method: application_probe::MANAGED_GUARD.method.to_owned(),
        arg: ic_wire::native::encode_request(
            application_probe::MANAGED_GUARD.method_id,
            &(),
            application_probe::MANAGED_GUARD.request_limits,
        )
        .unwrap(),
        user_preferences: ConsentMessageSpec {
            metadata: ConsentMessageMetadata {
                language: "en".to_owned(),
                utc_offset_minutes: Some(75),
            },
            device_spec: Some(DisplayMessageType::GenericDisplay),
        },
    }
}

fn consent(
    fixture: &ManagedAppFixture,
    caller: Principal,
    request: &ConsentMessageRequest,
) -> Result<ConsentInfo, Icrc21Error> {
    decode_one(
        &fixture
            .pic()
            .update_call(fixture.app(), caller, CONSENT, encode_one(request).unwrap())
            .unwrap(),
    )
    .unwrap()
}

fn standards(fixture: &ManagedAppFixture, bytes: Vec<u8>) -> Vec<SupportedStandard> {
    decode_one(
        &fixture
            .pic()
            .query_call(fixture.app(), Principal::anonymous(), DISCOVERY, bytes)
            .unwrap(),
    )
    .unwrap()
}

fn absent(result: Result<Vec<u8>, ClientError>) {
    let ClientError::Canister(rejection) = result.unwrap_err() else {
        panic!("expected a canister rejection");
    };
    assert_eq!(rejection.error_code, ErrorCode::CanisterMethodNotFound);
}

fn dispatches(fixture: &ManagedAppFixture) -> u32 {
    ic_testkit::pic::query_wire::<_, Result<u32, Error>>(
        fixture.pic(),
        fixture.app(),
        Principal::anonymous(),
        application_probe::MANAGED_WORKFLOW_RUNS,
        &(),
    )
    .unwrap()
    .unwrap()
}

fn inspect(wasm: &[u8], profile: MountProfile) {
    let facts = ic_host_artifacts::wasm::inspect(
        wasm,
        InspectionLimits {
            module_bytes: wasm.len(),
            sections: wasm.len() / 2,
            exports: u32::try_from(wasm.len()).unwrap(),
            custom_sections: wasm.len() / 2,
        },
    )
    .unwrap();
    for name in [
        "canister_init",
        "canister_post_upgrade",
        "canister_global_timer",
    ] {
        assert_eq!(facts.exports[name].kind, ExportKind::Function);
    }
    let actual = facts
        .exports
        .keys()
        .filter(|name| {
            name.split_whitespace()
                .nth(1)
                .is_some_and(|method| method.starts_with("icrc"))
        })
        .map(|name| (*name).to_owned())
        .collect::<BTreeSet<_>>();
    let mut expected = BTreeSet::new();
    if matches!(profile, MountProfile::Discovery | MountProfile::Consent) {
        expected.insert(format!("canister_query {DISCOVERY}"));
    }
    if profile == MountProfile::Consent {
        expected.insert(format!("canister_update {CONSENT}"));
    }
    assert_eq!(actual, expected, "{profile:?}");
}

#[test]
fn managed_profiles_mount_exact_external_methods_and_keep_one_lifecycle() {
    for &profile in MountProfile::ALL {
        let fixture = icrc::install(profile);
        inspect(&fixture.wasm, profile);
        let app = &fixture.application;
        activate(app);
        if matches!(profile, MountProfile::Discovery | MountProfile::Consent) {
            let response = standards(app, encode_args(()).unwrap());
            let mut expected = vec![(
                "ICRC-10",
                "https://github.com/dfinity/ICRC/blob/main/ICRCs/ICRC-10/ICRC-10.md",
            )];
            if profile == MountProfile::Consent {
                expected.push((
                    "ICRC-21",
                    "https://github.com/dfinity/ICRC/blob/main/ICRCs/ICRC-21/ICRC-21.md",
                ));
            }
            assert_eq!(
                response
                    .iter()
                    .map(|s| (s.name.as_str(), s.url.as_str()))
                    .collect::<Vec<_>>(),
                expected
            );
            // The IC also permits a query method to run as a replicated query.
            // Declared query/update modes are checked on the actual exports above.
            let replicated: Vec<SupportedStandard> = decode_one(
                &app.pic()
                    .update_call(
                        app.app(),
                        Principal::anonymous(),
                        DISCOVERY,
                        encode_args(()).unwrap(),
                    )
                    .unwrap(),
            )
            .unwrap();
            assert_eq!(
                replicated
                    .iter()
                    .map(|s| (s.name.as_str(), s.url.as_str()))
                    .collect::<Vec<_>>(),
                expected
            );
        } else {
            absent(app.pic().query_call(
                app.app(),
                Principal::anonymous(),
                DISCOVERY,
                encode_args(()).unwrap(),
            ));
            absent(app.pic().update_call(
                app.app(),
                Principal::anonymous(),
                DISCOVERY,
                encode_args(()).unwrap(),
            ));
        }
        absent(app.pic().query_call(
            app.app(),
            Principal::anonymous(),
            CONSENT,
            encode_one(request()).unwrap(),
        ));
        if profile != MountProfile::Consent {
            absent(app.pic().update_call(
                app.app(),
                Principal::anonymous(),
                CONSENT,
                encode_one(request()).unwrap(),
            ));
        }
    }
}

/// Positive application dispatch evidence, separate from the consent response.
#[derive(serde::Deserialize)]
struct DispatchReceipt {
    caller: ic_platform::Principal,
    workflow_runs: u32,
}

#[test]
fn consent_forwards_application_policy_without_executing_or_authorizing_the_call() {
    let fixture = icrc::install(MountProfile::Consent);
    let app = &fixture.application;
    activate(app);
    let input = request();
    assert_eq!(dispatches(app), 0);
    for caller in [Principal::anonymous(), ic_testkit::fake_principal(17)] {
        let response = consent(app, caller, &input).unwrap();
        assert_eq!(response.metadata, input.user_preferences.metadata);
        assert_eq!(
            response.consent_message,
            ConsentMessage::GenericDisplayMessage("fixture consent".to_owned())
        );
    }
    let mut invalid = input.clone();
    invalid.method = "unsupported".to_owned();
    assert!(matches!(
        consent(app, Principal::anonymous(), &invalid),
        Err(Icrc21Error::UnsupportedCanisterCall(_))
    ));
    invalid = input.clone();
    invalid.arg = vec![0xff];
    assert!(matches!(
        consent(app, Principal::anonymous(), &invalid),
        Err(Icrc21Error::UnsupportedCanisterCall(_))
    ));
    invalid = input.clone();
    invalid.user_preferences.metadata.language = "unsupported".to_owned();
    assert!(matches!(
        consent(app, Principal::anonymous(), &invalid),
        Err(Icrc21Error::ConsentMessageUnavailable(_))
    ));
    invalid = input;
    invalid.user_preferences.device_spec = Some(DisplayMessageType::FieldsDisplay);
    assert!(matches!(
        consent(app, Principal::anonymous(), &invalid),
        Err(Icrc21Error::ConsentMessageUnavailable(_))
    ));
    assert_eq!(dispatches(app), 0);
    let refused: Result<DispatchReceipt, Error> = ic_testkit::pic::update_wire(
        app.pic(),
        app.app(),
        Principal::anonymous(),
        application_probe::MANAGED_GUARD,
        &(),
    )
    .unwrap();
    assert!(refused.is_err());
    assert_eq!(dispatches(app), 0);
    let admitted = ic_testkit::fake_principal(15);
    let accepted: Result<DispatchReceipt, Error> = ic_testkit::pic::update_wire(
        app.pic(),
        app.app(),
        admitted,
        application_probe::MANAGED_GUARD,
        &(),
    )
    .unwrap();
    let accepted = accepted.unwrap();
    assert_eq!(accepted.caller, admitted.into());
    assert_eq!(accepted.workflow_runs, 1);
    assert_eq!(dispatches(app), 1);
}

#[test]
fn malformed_and_oversized_consent_envelopes_are_rejected_before_policy() {
    let fixture = icrc::install(MountProfile::Consent);
    let app = &fixture.application;
    activate(app);
    let mut oversized = request();
    oversized.arg = vec![0; 4096];
    for (bytes, expected) in [
        (vec![0xff], ErrorCode::CanisterCalledTrap),
        (
            encode_one(oversized).unwrap(),
            ErrorCode::CanisterRejectedMessage,
        ),
    ] {
        let rejection = app
            .pic()
            .update_call(app.app(), Principal::anonymous(), CONSENT, bytes)
            .unwrap_err();
        let ic_testkit::pocket_ic::ClientError::Canister(rejection) = rejection else {
            panic!("expected a canister rejection");
        };
        assert_eq!(rejection.error_code, expected);
    }
    assert_eq!(dispatches(app), 0);
    assert!(consent(app, Principal::anonymous(), &request()).is_ok());
}

fn unused_header(depth: u8) -> Vec<u8> {
    let mut bytes = b"DIDL".to_vec();
    bytes.push(depth);
    for index in 0..depth {
        bytes.extend_from_slice(&[
            0x6c,
            1,
            0,
            if index + 1 == depth { 0x7f } else { index + 1 },
        ]);
    }
    bytes.push(0); // Discovery has zero external arguments; these types are unused.
    bytes
}

#[test]
fn mounted_discovery_enforces_platform_nesting_even_for_unused_header_types() {
    let fixture = icrc::install(MountProfile::Discovery);
    let app = &fixture.application;
    activate(app);
    let regular = standards(app, encode_args(()).unwrap());
    let boundary = standards(app, unused_header(16));
    assert_eq!(regular[0].name, boundary[0].name);
    let error = app
        .pic()
        .query_call(
            app.app(),
            Principal::anonymous(),
            DISCOVERY,
            unused_header(17),
        )
        .unwrap_err();
    let ic_testkit::pocket_ic::ClientError::Canister(error) = error else {
        panic!("expected a canister rejection");
    };
    assert_eq!(error.error_code, ErrorCode::CanisterCalledTrap);
    assert_eq!(standards(app, encode_args(()).unwrap())[0].name, "ICRC-10");
}

#[test]
fn mounted_consent_survives_same_release_restoration_without_registration() {
    let fixture = icrc::install(MountProfile::Consent);
    let app = &fixture.application;
    activate(app);
    let before = consent(app, Principal::anonymous(), &request()).unwrap();
    app.upgrade_same_release(Duration::from_mins(5)).unwrap();
    app.wait_until_active(40).unwrap();
    assert_eq!(
        consent(app, Principal::anonymous(), &request()).unwrap(),
        before
    );
    assert_eq!(dispatches(app), 0);
}
