//! Qualify the adapter in both framework-owned managed consumer shapes.
//!
//! The caller supplies manifest-verified artifacts and owns the PocketIC server.
//! These cases exercise local composition, not provider uploads or funding.

use std::{fs, time::Duration};

use candid::{CandidType, Principal};
use canic::testing::{
    CandidCallExt, ManagedApplicationInit, ManagedComponentGroupQualificationInput,
    ManagedRoleQualificationArtifact, install_managed_component_group,
};
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
use ic_blob_storage_contracts::identity::ProviderRootHash;

const NAMESPACE: u128 = 7;

#[derive(CandidType)]
struct EmbeddedInput {
    initial_count: u64,
    blob: ServiceInstallationInput,
}

fn principal(byte: u8) -> Principal {
    Principal::from_slice(&[byte; 29])
}

fn installation(service: Principal) -> ServiceInstallationInput {
    ServiceInstallationInput {
        configuration: ServiceConfigurationInput {
            service,
            operator: principal(7),
            payment_account: principal(8),
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
                cashier: principal(10),
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
        completion_verifier: principal(11),
    }
}

fn dedicated_init(service: Principal) -> Vec<u8> {
    candid::encode_one(installation(service)).expect("encode dedicated installation")
}

fn embedded_init(service: Principal) -> Vec<u8> {
    candid::encode_one(EmbeddedInput {
        initial_count: 41,
        blob: installation(service),
    })
    .expect("encode embedded installation")
}

/// Inputs for one owning consumer composition, supplied by its managed build.
struct ConsumerInput {
    role: &'static str,
    config: &'static str,
    prefix: &'static str,
    encode: fn(Principal) -> Vec<u8>,
}

fn qualify(embedded: bool) {
    let ConsumerInput {
        role,
        config,
        prefix,
        encode,
    } = if embedded {
        ConsumerInput {
            role: "backend",
            config: include_str!("../embedded-consumer/canic.toml"),
            prefix: "CANIC_BLOB_EMBEDDED",
            encode: embedded_init,
        }
    } else {
        ConsumerInput {
            role: "blob",
            config: include_str!("../consumer/canic.toml"),
            prefix: "CANIC_BLOB_DEDICATED",
            encode: dedicated_init,
        }
    };
    let build_id = std::env::var(format!("{prefix}_RELEASE_BUILD_ID"))
        .expect("complete managed release-build identity");
    let wasm = fs::read(
        std::env::var(format!("{prefix}_WASM_PATH")).expect("manifest-verified Wasm path"),
    )
    .expect("read exact consumer Wasm");
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
            pic.update_candid_as(service, operator, "blob_resume_current_instance", ())
                .expect("recovery transport");
        assert_eq!(result, Ok(()));
    };
    resume();
    let configuration = || {
        let result: Result<HostConfigurationView, HostFailure> = pic
            .query_candid_as(service, operator, "blob_configuration", ())
            .expect("configuration transport");
        result.expect("operator configuration")
    };
    let installed = configuration();
    assert_eq!(installed.configuration, installation(service).configuration);
    assert_eq!(installed.release, ic_blob_storage::LIBRARY_VERSION);
    assert!(!installed.fenced);
    let denied: Result<HostConfigurationView, HostFailure> = pic
        .query_candid_as(service, stranger, "blob_configuration", ())
        .expect("stranger query transport");
    assert_eq!(denied, Err(HostFailure::Denied));

    let scope = TenantScope {
        service,
        namespace: NAMESPACE,
        tenant,
    };
    let capacity = || {
        let result: Result<UploadCapacityResponse, UploadCapacityFailure> = pic
            .query_candid_as(service, tenant, "blob_upload_capacity", (scope,))
            .expect("tenant capacity transport");
        result
    };
    assert_eq!(capacity(), Err(UploadCapacityFailure::NotEnrolled));
    let request = TenantUpdateRequest {
        scope,
        expected: None,
        active: true,
    };
    let denied: Result<TenantEnrollmentResponse, TenantFailure> = pic
        .update_candid_as(service, stranger, "blob_update_tenant", (request,))
        .expect("stranger mutation transport");
    assert_eq!(denied, Err(TenantFailure::Denied));
    let wrong = TenantScope {
        service: principal(12),
        ..scope
    };
    let invalid: Result<TenantEnrollmentResponse, TenantFailure> = pic
        .query_candid_as(service, operator, "blob_tenant", (wrong,))
        .expect("wrong-service query transport");
    assert_eq!(invalid, Err(TenantFailure::Binding));

    // Discard the mutation result, then reconcile from authenticated current state.
    let _: Result<TenantEnrollmentResponse, TenantFailure> = pic
        .update_candid_as(service, operator, "blob_update_tenant", (request,))
        .expect("accepted mutation transport");
    let enrolled: Result<TenantEnrollmentResponse, TenantFailure> = pic
        .query_candid_as(service, tenant, "blob_tenant", (scope,))
        .expect("tenant reconciliation transport");
    let enrolled = enrolled.expect("retained enrollment after discarded result");
    assert!(enrolled.enrollment.expect("enrollment").active);
    let replay = TenantUpdateRequest {
        expected: enrolled.enrollment,
        ..request
    };
    let replayed: Result<TenantEnrollmentResponse, TenantFailure> = pic
        .update_candid_as(service, operator, "blob_update_tenant", (replay,))
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
    let denied: Result<UploadCapacityResponse, UploadCapacityFailure> = pic
        .query_candid_as(service, stranger, "blob_upload_capacity", (scope,))
        .expect("stranger capacity transport");
    assert_eq!(denied, Err(UploadCapacityFailure::Denied));

    // The tenant grants exact per-upload authority to two distinct browser actors.
    let expires_at_ns = pic
        .get_time()
        .as_nanos_since_unix_epoch()
        .checked_add(60_000_000_000)
        .expect("bounded permission deadline");
    let permissions = [principal(13), principal(14)]
        .into_iter()
        .enumerate()
        .map(|(index, uploader)| UploadAdmissionRequest {
            upload: ReferenceUpload {
                service,
                tenant,
                namespace: NAMESPACE,
                upload: index as u128 + 1,
                object: index as u128 + 1,
                incarnation: 1,
                first_reference: 1,
                root: [index as u8 + 1; 32],
                bytes: 1,
            },
            uploader,
            expires_at_ns,
        })
        .collect::<Vec<_>>();
    for permission in &permissions {
        let denied: Result<UploadAdmissionMutation, UploadAdmissionFailure> = pic
            .update_candid_as(
                service,
                permission.uploader,
                "blob_admit_upload",
                (*permission,),
            )
            .expect("uploader cannot grant tenant authority");
        assert_eq!(denied, Err(UploadAdmissionFailure::Denied));
        let granted: Result<UploadAdmissionMutation, UploadAdmissionFailure> = pic
            .update_candid_as(service, tenant, "blob_admit_upload", (*permission,))
            .expect("tenant permission transport");
        let granted = granted.expect("tenant grants uploader permission");
        assert_eq!(granted.admission.permission, *permission);
        assert!(!granted.replayed);
        let root = ProviderRootHash::try_from(permission.upload.root.as_slice())
            .expect("exact provider root bytes")
            .to_string();
        for actor in [permission.uploader, stranger] {
            let assessment: Result<UploadCertificateAssessmentResponse, UploadExposureFailure> =
                pic.query_candid_as(
                    service,
                    actor,
                    "blob_upload_certificate_assessment",
                    (root.clone(),),
                )
                .expect("certificate assessment transport");
            let expected = if actor == permission.uploader {
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
        let count: u64 = pic
            .update_candid(service, "application_increment", ())
            .expect("application mutation");
        assert_eq!(count, 42);
    }
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
    resume();
    resume();
    assert!(!configuration().fenced);
    assert_eq!(capacity().unwrap(), admitted_capacity);
    let restored: Result<TenantEnrollmentResponse, TenantFailure> = pic
        .query_candid_as(service, tenant, "blob_tenant", (scope,))
        .expect("restored tenant transport");
    assert_eq!(restored.unwrap().enrollment, enrolled.enrollment);
    for permission in permissions {
        let replayed: Result<UploadAdmissionMutation, UploadAdmissionFailure> = pic
            .update_candid_as(service, tenant, "blob_admit_upload", (permission,))
            .expect("restored uploader permission transport");
        let replayed = replayed.expect("retained exact permission after restoration");
        assert!(replayed.replayed);
        assert_eq!(replayed.admission.permission, permission);
    }
    if embedded {
        let count: u64 = pic
            .query_candid(service, "application_count", ())
            .expect("restored application counter");
        assert_eq!(count, 42);
    }
}

#[test]
#[ignore = "requires complete managed artifacts and a caller-owned PocketIC server"]
fn managed_composition_dedicated() {
    qualify(false);
}

#[test]
#[ignore = "requires complete managed artifacts and a caller-owned PocketIC server"]
fn managed_composition_embedded() {
    qualify(true);
}
