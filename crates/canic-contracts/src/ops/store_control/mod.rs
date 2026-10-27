//! Lossless native boundary conversions; stored grant and cycle records remain unchanged.
use crate::dto::{fixture_provisioning as fixture, store_control as native, template};
impl From<fixture::FixtureGrantRequest> for native::FixtureGrantRequest {
    fn from(value: fixture::FixtureGrantRequest) -> Self {
        Self {
            expected_revision: value.expected_revision,
            binding: value.binding.into(),
            enabled: value.enabled,
        }
    }
}
impl From<native::FixtureGrantRequest> for fixture::FixtureGrantRequest {
    fn from(value: native::FixtureGrantRequest) -> Self {
        Self {
            expected_revision: value.expected_revision,
            binding: value.binding.into(),
            enabled: value.enabled,
        }
    }
}
impl From<template::WasmStoreDeletionCycleReclamationResponse> for native::ReclamationResponse {
    fn from(value: template::WasmStoreDeletionCycleReclamationResponse) -> Self {
        Self {
            destination: value.destination.into(),
            cycles_before: value.cycles_before,
            retained_cycles_target: value.retained_cycles_target,
            cycles_transferred: value.cycles_transferred,
            cycles_after: value.cycles_after,
        }
    }
}
impl From<native::ReclamationResponse> for template::WasmStoreDeletionCycleReclamationResponse {
    fn from(value: native::ReclamationResponse) -> Self {
        Self {
            destination: value.destination.into(),
            cycles_before: value.cycles_before,
            retained_cycles_target: value.retained_cycles_target,
            cycles_transferred: value.cycles_transferred,
            cycles_after: value.cycles_after,
        }
    }
}
