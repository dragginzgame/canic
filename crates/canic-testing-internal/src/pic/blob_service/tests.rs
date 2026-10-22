//! Qualify the adapter in both framework-owned managed consumer shapes.
//!
//! The caller supplies manifest-verified artifacts and owns the PocketIC server.
//! These cases exercise local composition, not provider uploads or funding.

use std::time::Duration;

use super::artifacts::{self, Composition};

use canic_blob_service::__private::ic_blob_storage_contracts as contracts;

pub(super) fn decode_blob<T: serde::Serialize + serde::de::DeserializeOwned>(
    method: &str,
    bytes: &[u8],
) -> Result<T, contracts::wire::Error> {
    contracts::wire::decode_response(
        method,
        bytes,
        canic::testing::CANDID_REPLY_LIMITS.reply_bytes,
    )
}
use canic::testing::{
    ManagedApplicationInit, ManagedComponentGroupQualificationInput,
    ManagedRoleQualificationArtifact, install_managed_component_group,
};
use canic_blob_service::__private::ic_blob_storage_contracts::identity::ProviderRootHash;
use canic_blob_service::dto::{
    configuration::{
        HostConfigurationView, HostFailure, ServiceBillingInput, ServiceConfigurationInput,
        ServiceFundingInput, ServiceInstallationInput, ServiceReadInput, ServiceResourceInput,
    },
    reference::ReferenceUpload,
    tenant::{TenantEnrollmentResponse, TenantFailure, TenantScope, TenantUpdateRequest},
    upload::{
        admission::{UploadAdmissionFailure, UploadAdmissionMutation, UploadAdmissionRequest},
        capacity::{UploadCapacityFailure, UploadCapacityResponse},
        certificate::UploadCertificateAssessmentResponse,
        exposure::UploadExposureFailure,
    },
};
use ic_platform::{candid::contract::CandidType, principal::ExternalPrincipal as Principal};

const NAMESPACE: u128 = 7;

#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
struct EmbeddedInput {
    initial_count: u64,
    blob: Vec<u8>,
}

const fn principal(byte: u8) -> Principal {
    Principal::from_slice(&[byte; 29])
}

fn installation(service: Principal) -> ServiceInstallationInput {
    ServiceInstallationInput {
        configuration: ServiceConfigurationInput {
            service: service.into(),
            operator: principal(7).into(),
            payment_account: principal(8).into(),
            namespace: NAMESPACE,
            resources: ServiceResourceInput {
                max_tenants: 2,
                max_object_bytes: 10,
                max_headers: 8,
                max_header_bytes: 1024,
                max_chunks: 4,
                max_tenant_chunks: 2,
                max_objects: 4,
                max_tenant_objects: 2,
                max_physical_bytes: 30_000,
                max_liability_bytes: 20_000,
                max_tenant_logical_bytes: 10_000,
                max_references_per_object: 2,
                max_receipts_per_object: 3,
                max_active: 4,
                max_tenant_active: 2,
            },
            billing: ServiceBillingInput {
                cashier: principal(10).into(),
                reserve: 100,
                minimum_balance: 100,
                target_balance: 200,
                max_gateway_entries: 8,
                max_gateway_unique: 4,
            },
            funding: ServiceFundingInput {
                allocated: 10_000,
                renewal_ceiling: 10_000,
                reserve: 100,
                max_attempts: 4,
            },
            reads: ServiceReadInput {
                sessions: 2,
                tenant_sessions: 1,
                reply_bytes: 2048,
                bytes: 4096,
                tenant_bytes: 2048,
            },
        },
        project: "canic-managed-blob".into(),
        completion_verifier: principal(11).into(),
    }
}

fn dedicated_init(service: Principal) -> Vec<u8> {
    contracts::wire::encode_request("blob_install", installation(service))
        .expect("encode dedicated installation")
}

fn embedded_init(service: Principal) -> Vec<u8> {
    ic_platform::candid::contract::encode_one(EmbeddedInput {
        initial_count: 41,
        blob: dedicated_init(service),
    })
    .expect("encode embedded installation")
}

/// Inputs for one owning consumer composition, supplied by its managed build.
struct ConsumerInput {
    role: &'static str,
    config: &'static str,
    encode: fn(Principal) -> Vec<u8>,
}

#[expect(
    clippy::too_many_lines,
    reason = "one managed composition journey retains installation, fencing and exact restoration observations"
)]
fn qualify(embedded: bool) {
    let composition = if embedded {
        Composition::Embedded
    } else {
        Composition::Dedicated
    };
    if artifacts::write_worker_result(composition) {
        return;
    }
    let _serial = crate::pic::acquire_pic_unit_test_serial_guard();
    let ConsumerInput {
        role,
        config,
        encode,
    } = if embedded {
        ConsumerInput {
            role: "backend",
            config: include_str!(
                "../../../../../integrations/blob-service/embedded-consumer/canic.toml"
            ),
            encode: embedded_init,
        }
    } else {
        ConsumerInput {
            role: "blob",
            config: include_str!("../../../../../integrations/blob-service/consumer/canic.toml"),
            encode: dedicated_init,
        }
    };
    let rust_path = if embedded {
        crate::pic::cases::rust_test_path(&managed_composition_embedded)
    } else {
        crate::pic::cases::rust_test_path(&managed_composition_dedicated)
    };
    let build = artifacts::release_build(rust_path);
    let (consumer, config_path) = composition.paths();
    let snapshot = canic_host::release_set::AppConfigSnapshot::load(&config_path).unwrap();
    let current =
        canic_host::release_set::load_persisted_current_release_set_manifest(&consumer, build)
            .unwrap();
    canic_host::release_build::validate_finalized_release_build_manifest(
        &consumer,
        build,
        &current.path,
    )
    .expect("complete managed artifact hashes and runtime identities");
    let application = canic_host::release_set::load_persisted_application_artifact_union(
        &consumer,
        snapshot.component_topology(),
        build,
    )
    .expect("managed application artifact manifest");
    let entry = application
        .union
        .entries
        .iter()
        .find(|entry| entry.role.as_str() == role)
        .expect("selected Blob role in application manifest");
    let wasm_path = consumer.join(&entry.wasm_relative_path);
    canic_host::canister_build::validate_wasm_endpoints(
        &wasm_path,
        &wasm_path.with_extension("did"),
    )
    .expect("Candid and raw declarations match callable consumer endpoints");
    let wasm = std::fs::read(wasm_path).expect("verified consumer Wasm");
    let build_id = build.to_string();
    let reinstall_wasm = wasm.clone();
    let mut artifact = ManagedRoleQualificationArtifact::new(role.parse().unwrap(), wasm);
    artifact.application_init_args = ManagedApplicationInit::ForCanister(encode);
    let input = ManagedComponentGroupQualificationInput::new(
        config,
        role,
        &build_id,
        vec![principal(7), principal(8), principal(9)],
        vec![artifact],
    );
    let fixture = install_managed_component_group(input).expect("install managed consumer");
    let nodes = fixture.nodes();
    let service = nodes
        .iter()
        .find(|node| node.role.as_str() == role)
        .expect("configured consumer node")
        .canister_id;
    let pic = fixture.pic();
    let operator = principal(7);
    let tenant = principal(8);
    let stranger = principal(9);
    let resume = || {
        let result: Result<(), canic_blob_service::dto::recovery::CurrentInstanceRecoveryFailure> =
            decode_blob(
                "blob_resume_current_instance",
                &pic.update_call(
                    service,
                    operator,
                    "blob_resume_current_instance",
                    contracts::wire::encode_request("blob_resume_current_instance", ()).unwrap(),
                )
                .expect("Blob transport"),
            )
            .expect("recovery transport");
        assert_eq!(result, Ok(()));
    };
    resume();
    let configuration = || {
        let result: Result<HostConfigurationView, HostFailure> = decode_blob(
            "blob_configuration",
            &pic.query_call(
                service,
                operator,
                "blob_configuration",
                contracts::wire::encode_request("blob_configuration", ()).unwrap(),
            )
            .expect("Blob transport"),
        )
        .expect("configuration transport");
        result.expect("operator configuration")
    };
    let installed = configuration();
    assert_eq!(installed.configuration, installation(service).configuration);
    assert_eq!(
        installed.release,
        canic_blob_service::__private::ic_blob_storage_contracts::CONTRACT_VERSION
    );
    assert!(!installed.fenced);
    refuse_invalid_configuration(&fixture, service, &reinstall_wasm, embedded, &installed);
    resume();
    assert_eq!(configuration(), installed);
    let denied: Result<HostConfigurationView, HostFailure> = decode_blob(
        "blob_configuration",
        &pic.query_call(
            service,
            stranger,
            "blob_configuration",
            contracts::wire::encode_request("blob_configuration", ()).unwrap(),
        )
        .expect("Blob transport"),
    )
    .expect("stranger query transport");
    assert_eq!(denied, Err(HostFailure::Denied));

    let scope = TenantScope {
        service: service.into(),
        namespace: NAMESPACE,
        tenant: tenant.into(),
    };
    let capacity = || {
        let result: Result<UploadCapacityResponse, UploadCapacityFailure> = decode_blob(
            "blob_upload_capacity",
            &pic.query_call(
                service,
                tenant,
                "blob_upload_capacity",
                contracts::wire::encode_request("blob_upload_capacity", scope).unwrap(),
            )
            .expect("Blob transport"),
        )
        .expect("tenant capacity transport");
        result
    };
    assert_eq!(capacity(), Err(UploadCapacityFailure::NotEnrolled));
    let request = TenantUpdateRequest {
        scope,
        expected: None,
        active: true,
    };
    refuse_invalid_tenant_requests(&fixture, service, operator, request);
    assert_eq!(capacity(), Err(UploadCapacityFailure::NotEnrolled));
    let denied: Result<TenantEnrollmentResponse, TenantFailure> =
        contracts::wire::tenant::decode_update_response(
            &pic.update_call(
                service,
                stranger,
                "blob_update_tenant",
                contracts::wire::tenant::update_request(request).unwrap(),
            )
            .expect("Blob transport"),
        )
        .expect("stranger mutation transport");
    assert_eq!(denied, Err(TenantFailure::Denied));
    let wrong = TenantScope {
        service: principal(12).into(),
        ..scope
    };
    let invalid: Result<TenantEnrollmentResponse, TenantFailure> =
        contracts::wire::tenant::decode_inspection_response(
            &pic.query_call(
                service,
                operator,
                "blob_tenant",
                contracts::wire::tenant::inspection_request(wrong).unwrap(),
            )
            .expect("Blob transport"),
        )
        .expect("wrong-service query transport");
    assert_eq!(invalid, Err(TenantFailure::Binding));

    // Discard the mutation result, then reconcile from authenticated current state.
    let _: Result<TenantEnrollmentResponse, TenantFailure> =
        contracts::wire::tenant::decode_update_response(
            &pic.update_call(
                service,
                operator,
                "blob_update_tenant",
                contracts::wire::tenant::update_request(request).unwrap(),
            )
            .expect("Blob transport"),
        )
        .expect("accepted mutation transport");
    let enrolled: Result<TenantEnrollmentResponse, TenantFailure> =
        contracts::wire::tenant::decode_inspection_response(
            &pic.query_call(
                service,
                tenant,
                "blob_tenant",
                contracts::wire::tenant::inspection_request(scope).unwrap(),
            )
            .expect("Blob transport"),
        )
        .expect("tenant reconciliation transport");
    let enrolled = enrolled.expect("retained enrollment after discarded result");
    assert!(enrolled.enrollment.expect("enrollment").active);
    let replay = TenantUpdateRequest {
        expected: enrolled.enrollment,
        ..request
    };
    let replayed: Result<TenantEnrollmentResponse, TenantFailure> =
        contracts::wire::tenant::decode_update_response(
            &pic.update_call(
                service,
                operator,
                "blob_update_tenant",
                contracts::wire::tenant::update_request(replay).unwrap(),
            )
            .expect("Blob transport"),
        )
        .expect("exact current-state retry transport");
    assert_eq!(replayed.unwrap().enrollment, enrolled.enrollment);
    let initial_capacity = capacity().expect("enrolled tenant capacity");
    assert_eq!(initial_capacity.scope, scope);
    assert_eq!(initial_capacity.enrollment, enrolled.enrollment.unwrap());
    assert_eq!(initial_capacity.remaining_logical_bytes, 10_000);
    assert_eq!(initial_capacity.remaining_physical_bytes, 30_000);
    assert_eq!(initial_capacity.remaining_liability_bytes, 20_000);
    assert_eq!(initial_capacity.remaining_bytes, 10_000);
    assert!(!initial_capacity.fenced);
    let denied: Result<UploadCapacityResponse, UploadCapacityFailure> = decode_blob(
        "blob_upload_capacity",
        &pic.query_call(
            service,
            stranger,
            "blob_upload_capacity",
            contracts::wire::encode_request("blob_upload_capacity", scope).unwrap(),
        )
        .expect("Blob transport"),
    )
    .expect("stranger capacity transport");
    assert_eq!(denied, Err(UploadCapacityFailure::Denied));

    // The tenant grants exact per-upload authority to two distinct browser actors.
    let expires_at_ns = pic
        .get_time()
        .as_nanos_since_unix_epoch()
        .checked_add(3_600_000_000_000)
        .expect("bounded permission deadline");
    let permissions = [principal(13), principal(14)]
        .into_iter()
        .enumerate()
        .map(|(index, uploader)| UploadAdmissionRequest {
            upload: ReferenceUpload {
                service: service.into(),
                tenant: tenant.into(),
                namespace: NAMESPACE,
                upload: index as u128 + 1,
                object: index as u128 + 1,
                incarnation: 1,
                first_reference: 1,
                root: super::journey::root(
                    u8::try_from(index).expect("fixture object index fits u8") + 1,
                ),
                bytes: 1,
            },
            uploader: uploader.into(),
            expires_at_ns,
        })
        .collect::<Vec<_>>();
    super::journey::admission_refusals(pic, permissions[0]);
    for permission in &permissions {
        let denied: Result<UploadAdmissionMutation, UploadAdmissionFailure> = decode_blob(
            "blob_admit_upload",
            &pic.update_call(
                service,
                permission.uploader.into(),
                "blob_admit_upload",
                contracts::wire::encode_request("blob_admit_upload", *permission).unwrap(),
            )
            .expect("Blob transport"),
        )
        .expect("uploader cannot grant tenant authority");
        assert_eq!(denied, Err(UploadAdmissionFailure::Denied));
        let granted: Result<UploadAdmissionMutation, UploadAdmissionFailure> = decode_blob(
            "blob_admit_upload",
            &pic.update_call(
                service,
                tenant,
                "blob_admit_upload",
                contracts::wire::encode_request("blob_admit_upload", *permission).unwrap(),
            )
            .expect("Blob transport"),
        )
        .expect("tenant permission transport");
        let granted = granted.expect("tenant grants uploader permission");
        assert_eq!(granted.admission.permission, *permission);
        assert!(!granted.replayed);
        let root = ProviderRootHash::try_from(permission.upload.root.as_slice())
            .expect("exact provider root bytes")
            .to_string();
        for actor in [permission.uploader.into(), stranger] {
            let assessment: Result<UploadCertificateAssessmentResponse, UploadExposureFailure> =
                decode_blob(
                    "blob_upload_certificate_assessment",
                    &pic.query_call(
                        service,
                        actor,
                        "blob_upload_certificate_assessment",
                        contracts::wire::encode_request(
                            "blob_upload_certificate_assessment",
                            root.clone(),
                        )
                        .unwrap(),
                    )
                    .expect("Blob transport"),
                )
                .expect("certificate assessment transport");
            let expected = if actor == permission.uploader.into() {
                UploadExposureFailure::Unprepared
            } else {
                UploadExposureFailure::Permission(UploadAdmissionFailure::Denied)
            };
            assert_eq!(assessment, Err(expected));
        }
    }

    let admitted_capacity = capacity().expect("reserved upload capacity");
    assert_eq!(
        admitted_capacity,
        UploadCapacityResponse {
            remaining_objects: 0,
            remaining_active_uploads: 0,
            remaining_manifest_chunks: 0,
            remaining_logical_bytes: 9_998,
            remaining_physical_bytes: 29_998,
            remaining_liability_bytes: 19_998,
            remaining_bytes: 9_998,
            ..initial_capacity
        }
    );
    if embedded {
        let count: u64 = (|| -> Result<_, Box<dyn std::error::Error + Send + Sync>> {
            let request_bytes = ic_platform::candid::contract::encode_args(())?;
            let response_bytes = ic_testkit::pic::try_pocket_ic_operation(|| {
                (pic).update_call(
                    service,
                    ic_platform::principal::ExternalPrincipal::anonymous(),
                    "application_increment",
                    request_bytes,
                )
            })
            .map_err(ic_testkit::pic::PocketIcCallFailure::Operation)?
            .map_err(ic_testkit::pic::PocketIcCallFailure::Rejected)?;
            ic_platform::candid::decode_args(&response_bytes, canic::testing::CANDID_REPLY_LIMITS)
                .map_err(Into::into)
        })()
        .map(|(reply,)| reply)
        .expect("application mutation");
        assert_eq!(count, 42);
    }
    super::metrics::check(pic, service, 2, false, embedded.then_some(42));
    fixture
        .upgrade_same_release(service, Duration::from_secs(300))
        .expect("same-release restoration");
    assert!(configuration().fenced);
    assert_eq!(
        capacity().expect("restored fenced capacity"),
        UploadCapacityResponse {
            fenced: true,
            ..admitted_capacity
        }
    );
    super::metrics::check(pic, service, 2, true, embedded.then_some(42));
    resume();
    resume();
    assert!(!configuration().fenced);
    super::metrics::check(pic, service, 2, false, embedded.then_some(42));
    // Background framework updates may legitimately fence the shared owner.
    // Sampling reports that fence; only authenticated recovery can clear it.
    resume();
    assert_eq!(capacity().unwrap(), admitted_capacity);
    let restored: Result<TenantEnrollmentResponse, TenantFailure> =
        contracts::wire::tenant::decode_inspection_response(
            &pic.query_call(
                service,
                tenant,
                "blob_tenant",
                contracts::wire::tenant::inspection_request(scope).unwrap(),
            )
            .expect("Blob transport"),
        )
        .expect("restored tenant transport");
    assert_eq!(restored.unwrap().enrollment, enrolled.enrollment);
    for &permission in &permissions {
        let replayed: Result<UploadAdmissionMutation, UploadAdmissionFailure> = decode_blob(
            "blob_admit_upload",
            &pic.update_call(
                service,
                tenant,
                "blob_admit_upload",
                contracts::wire::encode_request("blob_admit_upload", permission).unwrap(),
            )
            .expect("Blob transport"),
        )
        .expect("restored uploader permission transport");
        let replayed = replayed.expect("retained exact permission after restoration");
        assert!(replayed.replayed);
        assert_eq!(replayed.admission.permission, permission);
    }
    if embedded {
        let count: u64 = (|| -> Result<_, Box<dyn std::error::Error + Send + Sync>> {
            let request_bytes = ic_platform::candid::contract::encode_args(())?;
            let response_bytes = ic_testkit::pic::try_pocket_ic_operation(|| {
                (pic).query_call(
                    service,
                    ic_platform::principal::ExternalPrincipal::anonymous(),
                    "application_count",
                    request_bytes,
                )
            })
            .map_err(ic_testkit::pic::PocketIcCallFailure::Operation)?
            .map_err(ic_testkit::pic::PocketIcCallFailure::Rejected)?;
            ic_platform::candid::decode_args(&response_bytes, canic::testing::CANDID_REPLY_LIMITS)
                .map_err(Into::into)
        })()
        .map(|(reply,)| reply)
        .expect("restored application counter");
        assert_eq!(count, 42);
    }
    let statements: Vec<_> = permissions
        .into_iter()
        .map(|permission| {
            super::journey::qualify(pic, permission, || {
                fixture
                    .upgrade_same_release(service, Duration::from_secs(300))
                    .unwrap();
                resume();
            })
        })
        .collect();
    fixture
        .upgrade_same_release(service, Duration::from_secs(300))
        .unwrap();
    for &statement in &statements {
        assert_eq!(
            super::journey::check_attestation(pic, &statement.request, true),
            statement
        );
    }
    resume();
    let original_permission = statements[0].request.permission;
    for statement in statements {
        assert_eq!(
            super::journey::check_attestation(pic, &statement.request, false),
            statement
        );
    }
    super::journey::qualify_refusals(pic, original_permission);
}

#[test]
pub(super) fn managed_composition_dedicated() {
    qualify(false);
}

#[test]
pub(super) fn managed_composition_embedded() {
    qualify(true);
}

#[derive(serde::Serialize)]
struct UnmodeledInstallationInput {
    configuration: ServiceConfigurationInput,
    project: String,
    completion_verifier: Principal,
    unmodeled: Principal,
}

#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
struct UnmodeledEmbeddedInput {
    initial_count: u64,
    blob: Vec<u8>,
}

fn refuse_invalid_tenant_requests(
    fixture: &canic::testing::ManagedComponentGroupFixture,
    service: Principal,
    operator: Principal,
    request: TenantUpdateRequest,
) {
    let pic = fixture.pic();
    let stable = pic.get_stable_memory(service);
    let mut trailing = contracts::wire::tenant::update_request(request).unwrap();
    trailing.push(0);
    let ceiling = contracts::wire::request_limit("blob_update_tenant").unwrap();
    for bytes in [
        Vec::new(),
        contracts::wire::encode_request("blob_configuration", ()).unwrap(),
        trailing,
        vec![0; ceiling + 1],
    ] {
        let expected = if bytes.len() > ceiling {
            ic_testkit::pic::RejectCode::CanisterReject
        } else {
            ic_testkit::pic::RejectCode::CanisterError
        };
        let refusal = pic
            .update_call(service, operator, "blob_update_tenant", bytes)
            .expect_err("malformed, wrong-method or oversized ingress must refuse");
        assert_eq!(refusal.reject_code, expected);
        assert_eq!(pic.get_stable_memory(service), stable);
    }
}

fn refuse_invalid_configuration(
    fixture: &canic::testing::ManagedComponentGroupFixture,
    service: Principal,
    wasm: &[u8],
    embedded: bool,
    installed: &HostConfigurationView,
) {
    use canic::{dto::abi::v1::CanisterInitPayload, testing::CanisterInstallExt as _};
    let pic = fixture.pic();
    let (protected, application): (CanisterInitPayload, Option<Vec<u8>>) =
        ic_platform::candid::decoder::decode_args(fixture.installation_arguments(service).unwrap())
            .unwrap();
    let unmodeled = UnmodeledInstallationInput {
        configuration: installation(service).configuration,
        project: installation(service).project,
        completion_verifier: installation(service).completion_verifier.into(),
        unmodeled: Principal::anonymous(),
    };
    let unknown_field = if embedded {
        ic_platform::candid::contract::encode_one(UnmodeledEmbeddedInput {
            initial_count: 41,
            blob: contracts::wire::encode_request("blob_install", unmodeled).unwrap(),
        })
        .unwrap()
    } else {
        contracts::wire::encode_request("blob_install", unmodeled).unwrap()
    };
    let extra_argument = if embedded {
        ic_platform::candid::contract::encode_args((
            EmbeddedInput {
                initial_count: 41,
                blob: dedicated_init(service),
            },
            "extra",
        ))
        .unwrap()
    } else {
        {
            let mut bytes = dedicated_init(service);
            bytes.push(0xf6);
            bytes
        }
    };
    let stable = pic.get_stable_memory(service);
    for invalid in [unknown_field, extra_argument] {
        assert!(invalid.len() < contracts::wire::request_limit("blob_install").unwrap());
        let args =
            ic_platform::candid::contract::encode_args((protected.clone(), Some(invalid))).unwrap();
        pic.wait_out_install_code_rate_limit(Duration::from_secs(300));
        let refusal = pic
            .reinstall_canister(service, wasm.to_vec(), args, Some(fixture.root()))
            .unwrap_err();
        assert_eq!(
            refusal.reject_code,
            ic_testkit::pic::RejectCode::CanisterError
        );
        assert_eq!(pic.get_stable_memory(service), stable);
        let current: Result<HostConfigurationView, HostFailure> = decode_blob(
            "blob_configuration",
            &pic.query_call(
                service,
                principal(7),
                "blob_configuration",
                contracts::wire::encode_request("blob_configuration", ()).unwrap(),
            )
            .expect("Blob transport"),
        )
        .unwrap();
        // Failed management installation preserves bytes but must not silently
        // activate the reopened Blob stores. Recovery remains operator-owned.
        assert_eq!(
            current.unwrap(),
            HostConfigurationView {
                fenced: true,
                ..installed.clone()
            }
        );
        if embedded {
            let count: u64 = (|| -> Result<_, Box<dyn std::error::Error + Send + Sync>> {
                let request_bytes = ic_platform::candid::contract::encode_args(())?;
                let reply = ic_testkit::pic::try_pocket_ic_operation(|| {
                    (pic).query_call(
                        service,
                        ic_platform::principal::ExternalPrincipal::anonymous(),
                        "application_count",
                        request_bytes,
                    )
                })
                .map_err(ic_testkit::pic::PocketIcCallFailure::Operation)?
                .map_err(ic_testkit::pic::PocketIcCallFailure::Rejected)?;
                ic_platform::candid::decode_args(&reply, canic::testing::CANDID_REPLY_LIMITS)
                    .map_err(Into::into)
            })()
            .map(|(reply,)| reply)
            .unwrap();
            assert_eq!(count, 41);
        }
    }
    // The exact original nested arguments remain unchanged, not re-encoded as a
    // permissive DTO after rejection. No successful reinstall is attempted.
    assert!(application.is_some());
}
