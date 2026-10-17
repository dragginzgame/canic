//! Exact Blob installation decoding for dedicated and nested application inputs.
//! The adapter owns limits; these native cases perform no installation effects.

use candid::{CandidType, Principal};
use canic::endpoint::ArgumentDecodeError;
use canic_blob_service::{
    dto::configuration::{
        ServiceBillingInput, ServiceConfigurationInput, ServiceFundingInput,
        ServiceInstallationInput, ServiceReadInput, ServiceResourceInput,
    },
    lifecycle::{CONFIGURATION_LIMITS, REQUEST_LIMITS},
};

#[derive(CandidType, Debug, serde::Deserialize, PartialEq)]
struct EmbeddedInput {
    initial_count: u64,
    blob: ServiceInstallationInput,
}

#[derive(CandidType)]
struct UnmodeledInstallationInput {
    configuration: ServiceConfigurationInput,
    project: String,
    completion_verifier: Principal,
    unmodeled: Principal,
}

#[derive(CandidType)]
struct UnmodeledEmbeddedInput {
    initial_count: u64,
    blob: UnmodeledInstallationInput,
}

fn installation() -> ServiceInstallationInput {
    let principal = |byte| Principal::from_slice(&[byte; 29]);
    ServiceInstallationInput {
        configuration: ServiceConfigurationInput {
            service: principal(1),
            operator: principal(7),
            payment_account: principal(8),
            namespace: 7,
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
        project: "decoder-test".into(),
        completion_verifier: principal(11),
    }
}

fn unmodeled(input: &ServiceInstallationInput) -> UnmodeledInstallationInput {
    UnmodeledInstallationInput {
        configuration: input.configuration,
        project: input.project.clone(),
        completion_verifier: input.completion_verifier,
        unmodeled: Principal::anonymous(),
    }
}

#[test]
fn dedicated_configuration_is_exact_while_ordinary_requests_keep_bounded_skipping() {
    let input = installation();
    let current = candid::encode_one(&input).unwrap();
    assert_eq!(
        CONFIGURATION_LIMITS.decode::<(ServiceInstallationInput,)>(&current),
        Ok((input.clone(),))
    );
    let extra = candid::encode_args((&input, "small")).unwrap();
    let field = candid::encode_one(unmodeled(&input)).unwrap();
    for bytes in [field, extra.clone()] {
        assert!(bytes.len() < CONFIGURATION_LIMITS.max_bytes);
        assert_eq!(
            CONFIGURATION_LIMITS.decode::<(ServiceInstallationInput,)>(&bytes),
            Err(ArgumentDecodeError::InvalidCandid)
        );
    }
    assert_eq!(
        REQUEST_LIMITS.decode::<(ServiceInstallationInput,)>(&extra),
        Ok((input,))
    );
}

#[test]
fn embedded_configuration_refuses_unknown_nested_fields_and_extra_arguments() {
    let input = EmbeddedInput {
        initial_count: 41,
        blob: installation(),
    };
    let current = candid::encode_one(&input).unwrap();
    assert_eq!(
        CONFIGURATION_LIMITS
            .decode::<(EmbeddedInput,)>(&current)
            .unwrap()
            .0,
        input
    );
    let field = candid::encode_one(UnmodeledEmbeddedInput {
        initial_count: input.initial_count,
        blob: unmodeled(&input.blob),
    })
    .unwrap();
    let extra = candid::encode_args((&input, "small")).unwrap();
    for bytes in [field, extra] {
        assert!(bytes.len() < CONFIGURATION_LIMITS.max_bytes);
        assert_eq!(
            CONFIGURATION_LIMITS.decode::<(EmbeddedInput,)>(&bytes),
            Err(ArgumentDecodeError::InvalidCandid)
        );
    }
}
