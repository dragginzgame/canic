//! Module: blob_service::workflow
//!
//! Responsibility: delegate endpoint operations to shared service workflows.
//! Does not own: blob storage schemas, tenant policy or a second provider implementation.
//! Boundary: platform continuity is checked before mutation and after async completion.

use crate::ops;

use ic_blob_storage::ops::service::operator::OperatorStores;
use ic_blob_storage_contracts::dto::funding::outcome::{
    FundingOutcomeFailure, FundingOutcomeRequest, FundingOutcomeResponse,
};
use ic_blob_storage_contracts::dto::funding::{
    FundingHistoryFailure, FundingHistoryPage, FundingHistoryRequest,
};
use ic_blob_storage_contracts::dto::operator::{
    LocalServiceStatus, LocalStatusFailure, OperatorScope,
};
use ic_blob_storage_contracts::dto::upload::history::{
    UploadHistoryFailure, UploadHistoryPage, UploadHistoryRequest,
};
use ic_blob_storage_contracts::dto::{
    configuration::{HostConfigurationView, HostFailure},
    reference::{
        ReferenceCommand, ReferenceFailure, ReferenceMutationResponse, ReferenceReceiptLookup,
    },
    tenant::{TenantEnrollmentResponse, TenantFailure, TenantScope, TenantUpdateRequest},
    upload::{
        admission::{
            UploadAdmissionFailure, UploadAdmissionMutation, UploadAdmissionRequest,
            UploadAdmissionResponse, UploadRevocationResponse,
        },
        manifest::{
            UploadManifestFailure, UploadManifestMutation, UploadManifestRequest,
            UploadManifestResponse,
        },
    },
};
use ic_blob_storage_contracts::upload::binding::UploadContext;

pub async fn before_update() {
    if let Some(request) = ops::begin_update() {
        if let Ok(proof) = ic_blob_storage::ops::service::recovery::prove_current_instance(
            request.installation_version,
        )
        .await
        {
            // Refusal retains the fence. The original endpoint workflow then
            // reports its own typed fenced outcome without changing a journal.
            let _ = ops::resume_continuous_active(proof);
        }
        ops::observe_version();
    }
}
pub async fn resume_current_instance(
    context: UploadContext,
) -> Result<(), ic_blob_storage_contracts::dto::recovery::CurrentInstanceRecoveryFailure> {
    let outcome = ic_blob_storage::workflow::installation::recovery::resume_current_instance(
        &ops::RecoveryHost,
        context,
    )
    .await
    .map_err(ic_blob_storage::ops::service::recovery::failure);
    ops::observe_version();
    outcome
}
pub fn certificate_assessment(
    context: UploadContext,
    root: &str,
    now: u64,
) -> Result<
    ic_blob_storage_contracts::dto::upload::certificate::UploadCertificateAssessmentResponse,
    ic_blob_storage_contracts::dto::upload::exposure::UploadExposureFailure,
> {
    ops::with_installation(|installation| {
        let stores = installation.stores();
        let permission = ic_blob_storage::workflow::uploads::certificate::resolve(
            &stores.uploads,
            context,
            root,
            now,
        )?;
        ic_blob_storage::workflow::uploads::certificate::inspect(
            &stores.uploads,
            context,
            root,
            installation.certificate_evidence(permission, now, true),
            now,
        )
    })
}

pub fn certificate(
    context: UploadContext,
    root: &str,
    now: u64,
) -> Result<
    ic_blob_storage_contracts::dto::upload::certificate::CaffeineUploadCertificateResponse,
    ic_blob_storage::workflow::uploads::certificate::UploadCertificateFailure,
> {
    use ic_blob_storage::workflow::uploads::certificate::{self, UploadCertificateFailure};
    ops::with_certificate(|installation| {
        let permission = certificate::resolve(&installation.stores().uploads, context, root, now)
            .map_err(UploadCertificateFailure::Exposure)?;
        // Synchronous replicated update: exposure and the plain reply commit together.
        let evidence = installation.certificate_evidence(permission, now, true);
        certificate::issue(
            &mut installation.stores_mut().uploads,
            context,
            root,
            evidence,
            now,
        )
    })
}
pub fn reference_capacity(
    context: UploadContext,
    input: ic_blob_storage_contracts::dto::reference::capacity::ReferenceCapacityRequest,
) -> Result<
    ic_blob_storage_contracts::dto::reference::capacity::ReferenceCapacityResponse,
    ic_blob_storage_contracts::dto::reference::capacity::ReferenceCapacityFailure,
> {
    ops::read(|stores| {
        ic_blob_storage::workflow::references::capacity::inspect(&stores.uploads, context, input)
    })
}
pub fn upload_capacity(
    context: UploadContext,
    input: ic_blob_storage_contracts::dto::tenant::TenantScope,
) -> Result<
    ic_blob_storage_contracts::dto::upload::capacity::UploadCapacityResponse,
    ic_blob_storage_contracts::dto::upload::capacity::UploadCapacityFailure,
> {
    ops::read(|stores| {
        ic_blob_storage::workflow::uploads::capacity::inspect(&stores.uploads, context, input)
    })
}
pub fn revoke_gateway(
    context: UploadContext,
    input: ic_blob_storage_contracts::dto::gateway::GatewayRevocationRequest,
) -> Result<
    ic_blob_storage_contracts::dto::gateway::GatewayRevocationResponse,
    ic_blob_storage_contracts::dto::gateway::GatewayRevocationFailure,
> {
    ops::mutate(|stores| {
        ic_blob_storage::workflow::gateways::revocation::revoke(
            &mut stores.gateways,
            context,
            input,
        )
    })
}
pub fn history(
    context: UploadContext,
    input: UploadHistoryRequest,
) -> Result<UploadHistoryPage, UploadHistoryFailure> {
    use ic_blob_storage::model::catalog::admission::read::UploadPageLimits;
    use std::num::NonZeroUsize;
    ops::read(|stores| {
        ic_blob_storage::workflow::uploads::history::inspect(
            &stores.uploads,
            context,
            input,
            UploadPageLimits {
                max_scan: NonZeroUsize::new(64).expect("fixed scan bound"),
                max_results: NonZeroUsize::new(32).expect("fixed reply bound"),
            },
        )
    })
}
pub fn install(input: &ic_blob_storage_contracts::dto::configuration::ServiceInstallationInput) {
    ops::install(input);
}
pub fn funding_history(
    context: UploadContext,
    input: FundingHistoryRequest,
) -> Result<FundingHistoryPage, FundingHistoryFailure> {
    ops::read(|stores| {
        ic_blob_storage::workflow::funding::history::inspect(
            &stores.funding,
            context,
            input,
            std::num::NonZeroUsize::new(32).expect("fixed funding page bound"),
        )
    })
}
pub fn restore() {
    ops::restore();
}
pub fn funding_outcome(
    context: UploadContext,
    input: FundingOutcomeRequest,
) -> Result<Option<FundingOutcomeResponse>, FundingOutcomeFailure> {
    ops::read(|stores| {
        ic_blob_storage::workflow::funding::outcome::inspect(&stores.funding, context, input)
    })
}
pub fn funding_assessment(
    context: UploadContext,
    input: ic_blob_storage_contracts::dto::funding::assessment::FundingPreparationRequest,
) -> Result<
    ic_blob_storage_contracts::dto::funding::assessment::FundingPreparationResponse,
    ic_blob_storage_contracts::dto::funding::assessment::FundingPreparationFailure,
> {
    ops::read(|stores| {
        ic_blob_storage::workflow::funding::assessment::inspect(&stores.funding, context, input)
    })
}
pub fn local_status(
    context: UploadContext,
    input: OperatorScope,
) -> Result<LocalServiceStatus, LocalStatusFailure> {
    ops::read(|stores| {
        ic_blob_storage::workflow::operator::inspect(OperatorStores::from(stores), context, input)
    })
}
pub fn configuration(actor: candid::Principal) -> Result<HostConfigurationView, HostFailure> {
    ops::with_installation(|installation| {
        ic_blob_storage::workflow::installation::inspect(
            installation,
            UploadContext {
                service: ic_cdk::api::canister_self(),
                actor,
            },
        )
    })
}
pub fn update_tenant(
    context: UploadContext,
    input: TenantUpdateRequest,
) -> Result<TenantEnrollmentResponse, TenantFailure> {
    ops::mutate(|stores| {
        ic_blob_storage::workflow::tenants::update(&mut stores.uploads, context, input)
    })
}
pub fn tenant(
    context: UploadContext,
    input: TenantScope,
) -> Result<TenantEnrollmentResponse, TenantFailure> {
    ops::read(|stores| ic_blob_storage::workflow::tenants::inspect(&stores.uploads, context, input))
}
pub fn admit(
    context: UploadContext,
    input: UploadAdmissionRequest,
    now: u64,
) -> Result<UploadAdmissionMutation, UploadAdmissionFailure> {
    ops::mutate(|stores| {
        ic_blob_storage::workflow::uploads::admission::admit(
            &mut stores.uploads,
            context,
            input,
            now,
        )
    })
}
pub fn admission(
    context: UploadContext,
    input: UploadAdmissionRequest,
) -> Result<UploadAdmissionResponse, UploadAdmissionFailure> {
    ops::read(|stores| {
        ic_blob_storage::workflow::uploads::admission::inspect(&stores.uploads, context, input)
    })
}
pub fn upload_status(
    context: UploadContext,
    input: ic_blob_storage_contracts::dto::reference::ReferenceUpload,
) -> Result<
    ic_blob_storage_contracts::dto::upload::UploadStatusResponse,
    ic_blob_storage_contracts::dto::upload::UploadStatusFailure,
> {
    ops::read(|stores| ic_blob_storage::workflow::uploads::inspect(&stores.uploads, context, input))
}
pub fn revoke(
    context: UploadContext,
    input: UploadAdmissionRequest,
) -> Result<UploadRevocationResponse, UploadAdmissionFailure> {
    ops::mutate(|stores| {
        ic_blob_storage::workflow::uploads::admission::revoke(&mut stores.uploads, context, input)
    })
}
pub fn prepare(
    context: UploadContext,
    input: &UploadManifestRequest,
    now: u64,
) -> Result<UploadManifestMutation, UploadManifestFailure> {
    ops::mutate(|stores| {
        ic_blob_storage::workflow::uploads::manifests::prepare(
            &mut stores.uploads,
            context,
            input,
            now,
        )
    })
}
pub fn manifest(
    context: UploadContext,
    input: UploadAdmissionRequest,
) -> Result<UploadManifestResponse, UploadManifestFailure> {
    ops::read(|stores| {
        ic_blob_storage::workflow::uploads::manifests::inspect(&stores.uploads, context, input)
    })
}
pub fn reference(
    context: UploadContext,
    input: ReferenceCommand,
) -> Result<ReferenceMutationResponse, ReferenceFailure> {
    ops::mutate(|stores| {
        ic_blob_storage::workflow::references::apply(&mut stores.uploads, context, input)
    })
}
pub fn receipt(
    context: UploadContext,
    input: ReferenceCommand,
) -> Result<ReferenceReceiptLookup, ReferenceFailure> {
    ops::read(|stores| {
        ic_blob_storage::workflow::references::receipt(&stores.uploads, context, input)
    })
}

pub async fn sync_gateways(
    context: UploadContext,
    input: OperatorScope,
) -> Result<
    ic_blob_storage_contracts::dto::gateway::sync::GatewaySyncResponse,
    ic_blob_storage_contracts::dto::gateway::sync::GatewaySyncFailure,
> {
    let outcome = ic_blob_storage::workflow::gateways::sync::refresh(
        &ops::gateways::GatewayHost,
        context,
        input,
        30.try_into().unwrap(),
        ops::gateways::limits(),
    )
    .await;
    ops::observe_version();
    outcome
}
pub fn cancel_gateway_sync(
    context: UploadContext,
    input: ic_blob_storage_contracts::dto::gateway::sync::GatewaySyncCancellation,
) -> Result<(), ic_blob_storage_contracts::dto::gateway::sync::GatewaySyncFailure> {
    ops::mutate(|stores| {
        ic_blob_storage::workflow::gateways::sync::cancel(&mut stores.gateways, context, input)
    })
}

pub fn discover(
    context: UploadContext,
    input: ic_blob_storage_contracts::dto::upload::discovery::UploadDiscoveryRequest,
) -> Result<
    ic_blob_storage_contracts::dto::upload::discovery::UploadDiscoveryResponse,
    ic_blob_storage_contracts::dto::upload::discovery::UploadDiscoveryFailure,
> {
    ops::read(|stores| {
        ic_blob_storage::workflow::uploads::discovery::inspect(&stores.uploads, context, input)
    })
}

pub fn download(
    context: UploadContext,
    input: ic_blob_storage_contracts::dto::download::DownloadRequest,
) -> Result<
    ic_blob_storage_contracts::dto::download::DownloadResponse,
    ic_blob_storage_contracts::dto::download::DownloadFailure,
> {
    ops::with_download(|uploads, scope| {
        ic_blob_storage::workflow::reads::download::handle(uploads, context, scope, input)
    })
}

pub fn reference_status(
    context: UploadContext,
    input: ic_blob_storage_contracts::dto::reference::status::ReferenceStatusRequest,
) -> Result<
    ic_blob_storage_contracts::dto::reference::status::ReferenceStatusResponse,
    ReferenceFailure,
> {
    ops::read(|stores| {
        ic_blob_storage::workflow::references::status::inspect(&stores.uploads, context, input)
    })
}

pub async fn inspect_account(
    context: UploadContext,
    input: ic_blob_storage_contracts::dto::account::AccountInspectionRequest,
) -> Result<
    ic_blob_storage_contracts::dto::account::AccountInspectionResponse,
    ic_blob_storage_contracts::dto::account::AccountInspectionFailure,
> {
    use ic_blob_storage::ops::caffeine::query::transport::replicated::account::ReplicatedAccountQuery;
    use ic_blob_storage_contracts::dto::account::AccountInspectionFailure;
    let transport = ReplicatedAccountQuery::new(
        input.scope.service,
        input.scope.cashier,
        input.scope.payment_account,
        30.try_into().unwrap(),
    )
    .map_err(|_| AccountInspectionFailure::Invalid)?;
    let outcome = ic_blob_storage::workflow::account::inspect(
        &ops::account::AccountHost,
        &transport,
        context,
        input,
        ops::account::limits(),
    )
    .await;
    ops::observe_version();
    outcome
}

pub fn attest(
    context: UploadContext,
    input: &ic_blob_storage_contracts::dto::upload::completion::UploadAttestationRequest,
) -> Result<
    ic_blob_storage_contracts::dto::upload::completion::UploadAttestationMutation,
    ic_blob_storage_contracts::dto::upload::completion::UploadAttestationFailure,
> {
    ops::with_completion(|stores, authority| {
        ic_blob_storage::workflow::uploads::completion::attest(
            &mut stores.uploads,
            authority,
            context,
            input,
            ic_cdk::api::time(),
        )
    })
}
pub fn attestation(
    context: UploadContext,
    input: UploadAdmissionRequest,
) -> Result<
    ic_blob_storage_contracts::dto::upload::completion::UploadAttestationResponse,
    ic_blob_storage_contracts::dto::upload::completion::UploadAttestationFailure,
> {
    ops::with_completion(|stores, authority| {
        ic_blob_storage::workflow::uploads::completion::inspect(
            &stores.uploads,
            authority,
            context,
            input,
        )
    })
}

pub fn verification_manifest(
    context: UploadContext,
    input: UploadAdmissionRequest,
) -> Result<UploadManifestResponse, UploadManifestFailure> {
    ops::with_completion(|stores, authority| {
        ic_blob_storage::workflow::uploads::completion::manifest(
            &stores.uploads,
            authority,
            context,
            input,
        )
    })
}

pub fn verification_plan(
    context: UploadContext,
    input: UploadAdmissionRequest,
) -> Result<
    ic_blob_storage_contracts::dto::upload::completion::UploadVerificationPlan,
    ic_blob_storage_contracts::dto::upload::completion::UploadAttestationFailure,
> {
    ops::with_verification(|uploads, authority, scope| {
        ic_blob_storage::workflow::uploads::completion::verification_plan(
            uploads, authority, scope, context, input,
        )
    })
}
