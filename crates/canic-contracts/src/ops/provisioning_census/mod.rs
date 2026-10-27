//! Lossless census projection; journal, retry and custody rules stay with their owners.
use crate::{
    dto::{
        component_provisioning::RootComponentProvisioningFailure,
        provisioning_census::{DependencyFailure, Entry, Failure, Page},
        root::{RootProvisioningReleaseEntry, RootProvisioningReleaseResponse},
    },
    values::provisioning_failure::ProvisioningDependencyFailure,
};
impl From<RootProvisioningReleaseResponse> for Page {
    fn from(value: RootProvisioningReleaseResponse) -> Self {
        Self {
            root: value.root.into(),
            active_provisioning: value.active_provisioning,
            active_directory_synchronization: value.active_directory_synchronization,
            entry: value.entry.map(Into::into),
            next_after: value.next_after,
        }
    }
}
impl From<Page> for RootProvisioningReleaseResponse {
    fn from(value: Page) -> Self {
        Self {
            root: value.root.into(),
            active_provisioning: value.active_provisioning,
            active_directory_synchronization: value.active_directory_synchronization,
            entry: value.entry.map(Into::into),
            next_after: value.next_after,
        }
    }
}
impl From<RootProvisioningReleaseEntry> for Entry {
    fn from(value: RootProvisioningReleaseEntry) -> Self {
        Self {
            key: value.key,
            plan_hash: value.plan_hash,
            phase: value.phase,
            delivery_in_flight: value.delivery_in_flight.map(Into::into),
            last_failure: value.last_failure.map(Into::into),
        }
    }
}
impl From<Entry> for RootProvisioningReleaseEntry {
    fn from(value: Entry) -> Self {
        Self {
            key: value.key,
            plan_hash: value.plan_hash,
            phase: value.phase,
            delivery_in_flight: value.delivery_in_flight.map(Into::into),
            last_failure: value.last_failure.map(Into::into),
        }
    }
}
impl From<RootComponentProvisioningFailure> for Failure {
    fn from(value: RootComponentProvisioningFailure) -> Self {
        Self {
            dependency: value.dependency.map(Into::into),
            stage: value.stage,
            target: value.target.into(),
            operation_id: value.operation_id,
            diagnostic_code: value.diagnostic_code,
            retry_category: value.retry_category,
            failed_at_ns: value.failed_at_ns,
            consecutive_failures: value.consecutive_failures,
            retry_at_ns: value.retry_at_ns,
        }
    }
}
impl From<Failure> for RootComponentProvisioningFailure {
    fn from(value: Failure) -> Self {
        Self {
            dependency: value.dependency.map(Into::into),
            stage: value.stage,
            target: value.target.into(),
            operation_id: value.operation_id,
            diagnostic_code: value.diagnostic_code,
            retry_category: value.retry_category,
            failed_at_ns: value.failed_at_ns,
            consecutive_failures: value.consecutive_failures,
            retry_at_ns: value.retry_at_ns,
        }
    }
}
impl From<ProvisioningDependencyFailure> for DependencyFailure {
    fn from(value: ProvisioningDependencyFailure) -> Self {
        Self {
            target: value.target.into(),
            method: value.method,
            step: value.step,
            rejection_code: value.rejection_code,
            rejection_detail: value.rejection_detail,
        }
    }
}
impl From<DependencyFailure> for ProvisioningDependencyFailure {
    fn from(value: DependencyFailure) -> Self {
        Self {
            target: value.target.into(),
            method: value.method,
            step: value.step,
            rejection_code: value.rejection_code,
            rejection_detail: value.rejection_detail,
        }
    }
}
