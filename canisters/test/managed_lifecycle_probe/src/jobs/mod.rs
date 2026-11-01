//! Application-owned Jobs composition inside the existing Canic lifecycle probe.
//! This is private qualification, not a public mount API or a paid-lease migration.
mod dto;
mod endpoints;
mod model;
mod ops;
mod protocol;
mod view;
mod workflow;

pub const KEY: &str = "managed_probe.jobs.v1";
pub fn install() {
    workflow::install();
}
pub fn restore() {
    workflow::restore();
}
