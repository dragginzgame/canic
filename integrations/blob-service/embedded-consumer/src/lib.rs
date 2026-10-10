//! An application with its own endpoints, durable state and embedded blob management.

mod app;

use canic::api::public_status::{ApplicationMetricsSampler, PublicStatusApi};
use canic_blob_service::{dto::configuration::ServiceInstallationInput, lifecycle};

/// This application's nested init contract, delivered after the protected Canic payload.
#[derive(candid::CandidType, serde::Deserialize)]
struct InstallationInput {
    initial_count: u64,
    blob: ServiceInstallationInput,
}

// The artifact grants both namespaces in one shared physical allocation pool.
canic::memory::memory_allocation_pool!(
    authorities = [
        ("embedded-app", "embedded_app."),
        (
            canic_blob_service::MEMORY_AUTHORITY,
            canic_blob_service::MEMORY_KEY_PREFIX
        ),
    ],
    exclusions = [],
);
canic_blob_service::mount!();

canic::start!(
    argument_limits = lifecycle::ENVELOPE_LIMITS,
    lifecycle_participant(init = install, post_upgrade = restore),
);

fn install() {
    let (_, application): (canic::dto::abi::v1::CanisterInitPayload, Option<Vec<u8>>) =
        lifecycle::ENVELOPE_LIMITS
            .read()
            .expect("bounded application envelope");
    let (input,): (InstallationInput,) = lifecycle::CONFIGURATION_LIMITS
        .decode(
            application
                .as_deref()
                .expect("application installation arguments"),
        )
        .expect("bounded application configuration");
    app::install(input.initial_count);
    lifecycle::install(&input.blob);
    register_metrics();
}

fn restore() {
    app::restore();
    lifecycle::restore();
    register_metrics();
}

fn register_metrics() {
    PublicStatusApi::set_application_sampler(Some(ApplicationMetricsSampler::new(sample)));
}

fn sample() -> Result<Vec<canic::dto::public_status::PublicMetric>, canic::Error> {
    let mut rows = canic_blob_service::metrics::sample()?;
    rows.push(app::metric());
    Ok(rows)
}

#[canic::canic_query(public, on_access_denied = "reject")]
fn application_count() -> u64 {
    app::count()
}

#[canic::canic_update(public, on_access_denied = "reject")]
fn application_increment() -> u64 {
    app::increment()
}

#[expect(clippy::unused_async, reason = "Canic deferred lifecycle signature")]
async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
#[expect(clippy::unused_async, reason = "restoration runs synchronously above")]
async fn canic_upgrade() {}

canic::finish!();
