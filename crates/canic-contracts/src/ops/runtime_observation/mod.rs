//! Lossless runtime and fixture projections at the transport boundary.
use crate::dto::{fixture_provisioning, runtime, runtime_observation as wire};
impl From<runtime::CanicRuntimeStatus> for wire::CanicRuntimeStatus {
    fn from(value: runtime::CanicRuntimeStatus) -> Self {
        Self {
            schema_version: value.schema_version,
            observed_at_ns: value.observed_at_ns,
            canister_id: value.canister_id.into(),
            role: value.role,
            root: value.root.map(Into::into),
            build_network: value.build_network,
            build: value.build,
            features: value.features,
            topology: value.topology.map(Into::into),
            timers: value.timers,
            timer_inventory: value.timer_inventory,
            state: value.state,
            auth: value.auth,
            receipt_capacity: value.receipt_capacity,
            recent_failures: value.recent_failures,
            visibility: value.visibility,
            readiness: value.readiness.into(),
            status: value.status,
        }
    }
}
impl From<wire::CanicRuntimeStatus> for runtime::CanicRuntimeStatus {
    fn from(value: wire::CanicRuntimeStatus) -> Self {
        Self {
            schema_version: value.schema_version,
            observed_at_ns: value.observed_at_ns,
            canister_id: value.canister_id.into(),
            role: value.role,
            root: value.root.map(Into::into),
            build_network: value.build_network,
            build: value.build,
            features: value.features,
            topology: value.topology.map(Into::into),
            timers: value.timers,
            timer_inventory: value.timer_inventory,
            state: value.state,
            auth: value.auth,
            receipt_capacity: value.receipt_capacity,
            recent_failures: value.recent_failures,
            visibility: value.visibility,
            readiness: value.readiness.into(),
            status: value.status,
        }
    }
}
impl From<runtime::CanicReadinessStatus> for wire::CanicReadinessStatus {
    fn from(value: runtime::CanicReadinessStatus) -> Self {
        Self {
            fixture: value.fixture.map(Into::into),
            schema_version: value.schema_version,
            role: value.role,
            status: value.status,
            observed_at_ns: value.observed_at_ns,
            checks: value.checks,
            blockers: value.blockers,
            warnings: value.warnings,
        }
    }
}
impl From<wire::CanicReadinessStatus> for runtime::CanicReadinessStatus {
    fn from(value: wire::CanicReadinessStatus) -> Self {
        Self {
            fixture: value.fixture.map(Into::into),
            schema_version: value.schema_version,
            role: value.role,
            status: value.status,
            observed_at_ns: value.observed_at_ns,
            checks: value.checks,
            blockers: value.blockers,
            warnings: value.warnings,
        }
    }
}
impl From<runtime::RuntimeTopologyStatus> for wire::RuntimeTopologyStatus {
    fn from(value: runtime::RuntimeTopologyStatus) -> Self {
        Self {
            root: value.root.map(Into::into),
            parent: value.parent.map(Into::into),
            subnet: value.subnet.map(Into::into),
            source: value.source,
        }
    }
}
impl From<wire::RuntimeTopologyStatus> for runtime::RuntimeTopologyStatus {
    fn from(value: wire::RuntimeTopologyStatus) -> Self {
        Self {
            root: value.root.map(Into::into),
            parent: value.parent.map(Into::into),
            subnet: value.subnet.map(Into::into),
            source: value.source,
        }
    }
}
impl From<fixture_provisioning::FixtureTargetBinding> for wire::FixtureTargetBinding {
    fn from(value: fixture_provisioning::FixtureTargetBinding) -> Self {
        Self {
            target: value.target.into(),
            installation: value.installation,
            release_build_id: value.release_build_id,
            content_id: value.content_id,
        }
    }
}
impl From<wire::FixtureTargetBinding> for fixture_provisioning::FixtureTargetBinding {
    fn from(value: wire::FixtureTargetBinding) -> Self {
        Self {
            target: value.target.into(),
            installation: value.installation,
            release_build_id: value.release_build_id,
            content_id: value.content_id,
        }
    }
}
impl From<fixture_provisioning::FixtureImportReceipt> for wire::FixtureImportReceipt {
    fn from(value: fixture_provisioning::FixtureImportReceipt) -> Self {
        Self {
            binding: value.binding.into(),
            completion_summary: value.completion_summary,
        }
    }
}
impl From<wire::FixtureImportReceipt> for fixture_provisioning::FixtureImportReceipt {
    fn from(value: wire::FixtureImportReceipt) -> Self {
        Self {
            binding: value.binding.into(),
            completion_summary: value.completion_summary,
        }
    }
}
impl From<fixture_provisioning::FixtureImportProgress> for wire::FixtureImportProgress {
    fn from(value: fixture_provisioning::FixtureImportProgress) -> Self {
        Self {
            binding: value.binding.into(),
            next_chunk: value.next_chunk,
            receipt: value.receipt.map(|value| Box::new((*value).into())),
        }
    }
}
impl From<wire::FixtureImportProgress> for fixture_provisioning::FixtureImportProgress {
    fn from(value: wire::FixtureImportProgress) -> Self {
        Self {
            binding: value.binding.into(),
            next_chunk: value.next_chunk,
            receipt: value.receipt.map(|value| Box::new((*value).into())),
        }
    }
}
impl From<fixture_provisioning::FixtureProvisioningStatus> for wire::FixtureProvisioningStatus {
    fn from(value: fixture_provisioning::FixtureProvisioningStatus) -> Self {
        match value {
            fixture_provisioning::FixtureProvisioningStatus::NotRequired => Self::NotRequired,
            fixture_provisioning::FixtureProvisioningStatus::AwaitingImporter => {
                Self::AwaitingImporter
            }
            fixture_provisioning::FixtureProvisioningStatus::Pending(value) => {
                Self::Pending(value.map(|v| Box::new((*v).into())))
            }
            fixture_provisioning::FixtureProvisioningStatus::Complete(value) => {
                Self::Complete(Box::new((*value).into()))
            }
            fixture_provisioning::FixtureProvisioningStatus::Failed(value) => Self::Failed(value),
        }
    }
}
impl From<wire::FixtureProvisioningStatus> for fixture_provisioning::FixtureProvisioningStatus {
    fn from(value: wire::FixtureProvisioningStatus) -> Self {
        match value {
            wire::FixtureProvisioningStatus::NotRequired => Self::NotRequired,
            wire::FixtureProvisioningStatus::AwaitingImporter => Self::AwaitingImporter,
            wire::FixtureProvisioningStatus::Pending(value) => {
                Self::Pending(value.map(|v| Box::new((*v).into())))
            }
            wire::FixtureProvisioningStatus::Complete(value) => {
                Self::Complete(Box::new((*value).into()))
            }
            wire::FixtureProvisioningStatus::Failed(value) => Self::Failed(value),
        }
    }
}
