//! An application with its own endpoints, durable state and embedded blob management.

mod app;
mod funding;
mod protocol;

use canic::api::public_status::{ApplicationMetricsSampler, PublicStatusApi};
use canic_blob_service::{dto::configuration::ServiceInstallationInput, lifecycle};

/// This application's nested init contract, delivered after the protected Canic payload.
#[derive(serde::Deserialize, serde::Serialize)]
struct InstallationInput {
    initial_count: u64,
    blob: Vec<u8>,
}

// The artifact grants both namespaces in one shared physical allocation pool.
canic::memory::memory_allocation_pool!(
    provider = memory_pool,
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
    memory = memory(),
    argument_limits = lifecycle::ENVELOPE_LIMITS,
    lifecycle_participant(init = install, post_upgrade = restore),
);

fn memory() -> canic::memory::MemoryBootstrap {
    let mut input = canic::memory::framework();
    input.pool = memory_pool().unwrap();
    input
        .requests
        .extend(canic_blob_service::memory_requests().unwrap());
    input.requests.extend(app::memory_requests());
    input
}

fn install() {
    let (_, application): (canic::dto::abi::v1::CanisterInitPayload, Option<Vec<u8>>) =
        canic::endpoint::read_initialization(lifecycle::ENVELOPE_LIMITS)
            .expect("bounded application envelope");
    let input: InstallationInput = canic::__internal::ic_wire::native::decode(
        application
            .as_deref()
            .expect("application installation arguments"),
        canic::__internal::ic_wire::Limits {
            bytes: 16_384,
            allocation: 262_144,
            nodes: 16_384,
            depth: 32,
        },
    )
    .expect("bounded application configuration");
    app::install(input.initial_count);
    let blob: ServiceInstallationInput =
        canic_blob_service::__private::ic_blob_storage_contracts::wire::decode_request(
            "blob_install",
            &input.blob,
            canic_blob_service::__private::ic_blob_storage_contracts::wire::request_limit(
                "blob_install",
            )
            .expect("Blob installation method"),
        )
        .expect("bounded Blob installation");
    lifecycle::install(&blob);
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

#[canic::query(public, on_access_denied = "reject", wire(id = protocol::COUNT_ID, request = protocol::LIMITS, response = protocol::LIMITS))]
fn application_count() -> u64 {
    app::count()
}

#[canic::update(public, on_access_denied = "reject", wire(id = protocol::INCREMENT_ID, request = protocol::LIMITS, response = protocol::LIMITS))]
fn application_increment() -> u64 {
    app::increment()
}

#[expect(clippy::unused_async, reason = "Canic deferred lifecycle signature")]
async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
#[expect(clippy::unused_async, reason = "restoration runs synchronously above")]
async fn canic_upgrade() {}

canic::finish!();
