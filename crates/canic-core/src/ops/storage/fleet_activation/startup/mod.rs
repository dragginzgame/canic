//! Module: ops::storage::fleet_activation::startup
//!
//! Durable application initialization owned by the existing activation receipt.
//!
//! Publication and timer orchestration remain in workflow; reads borrow the activation cache.

use super::{FleetActivation, FleetActivationOps, FleetActivationOpsError, replace_record};
use crate::{
    model::caller_authority::{
        CallerAdmissionError, CallerChangeRecord, CallerPublicationRecord, CallerReceiptPhase,
    },
    ops::caller_authority::CallerAuthorityOps,
    storage::stable::fleet_activation::{ApplicationStartupRecord, FleetActivationStateRecord},
    view::fleet_activation::{ApplicationStartupWork, FleetActivationView},
};

impl FleetActivationOps {
    /// Retain the original completed opening publication without changing it on retry.
    pub(crate) fn release_application_startup(
        release: CallerPublicationRecord,
    ) -> Result<(), FleetActivationOpsError> {
        let mut record = FleetActivation::get().ok_or(FleetActivationOpsError::NotInitialized)?;
        validate_release(&record, &release)
            .map_err(|_| FleetActivationOpsError::EvidenceMismatch)?;
        let activation = record
            .component_runtime
            .as_mut()
            .and_then(|runtime| runtime.activation.as_mut())
            .ok_or(FleetActivationOpsError::NotActive)?;
        match &activation.startup.release {
            Some(original) if original != &release => {
                return Err(FleetActivationOpsError::EvidenceMismatch);
            }
            Some(_) => return Ok(()),
            None => activation.startup.release = Some(release),
        }
        replace_record(record)
    }

    /// Validate restored startup evidence even when a retired receiver remains closed.
    pub(crate) fn restore_application_startup() -> Result<(), CallerAdmissionError> {
        FleetActivation::with(|record| {
            let record = record
                .as_ref()
                .ok_or(CallerAdmissionError::AuthorityUnavailable)?;
            let Some(startup) = startup(record) else {
                return Ok(());
            };
            match &startup.release {
                Some(release) => validate_release(record, release)?,
                None if startup.initialized => return Err(CallerAdmissionError::AuthorityConflict),
                None => {}
            }
            if startup.initialized && startup.arguments.is_some() {
                return Err(CallerAdmissionError::AuthorityConflict);
            }
            Ok(())
        })
    }

    /// Application admission is independent of access expressions and runtime Active.
    pub(crate) fn require_application_started() -> Result<(), CallerAdmissionError> {
        FleetActivation::with(|record| {
            let record = record
                .as_ref()
                .ok_or(CallerAdmissionError::AuthorityUnavailable)?;
            let startup = startup(record).ok_or(CallerAdmissionError::Fenced)?;
            let release = startup
                .release
                .as_ref()
                .ok_or(CallerAdmissionError::Fenced)?;
            if !startup.initialized {
                return Err(CallerAdmissionError::Fenced);
            }
            require_open(record, release)
        })
    }

    /// Copy arguments only for actual hook execution, never for endpoint admission.
    pub(crate) fn application_startup_work()
    -> Result<Option<ApplicationStartupWork>, CallerAdmissionError> {
        FleetActivation::with(|record| {
            let record = record
                .as_ref()
                .ok_or(CallerAdmissionError::AuthorityUnavailable)?;
            let Some(startup) = startup(record) else {
                return Ok(None);
            };
            let Some(release) = &startup.release else {
                return Ok(None);
            };
            if startup.initialized {
                return Ok(None);
            }
            match require_open(record, release) {
                Ok(()) => {}
                Err(CallerAdmissionError::Fenced) => return Ok(None),
                Err(error) => return Err(error),
            }
            Ok(Some(ApplicationStartupWork {
                release: release.clone(),
                arguments: startup.arguments.clone(),
            }))
        })
    }

    /// A completed hook clears arguments only under its original retained release.
    pub(crate) fn complete_application_startup(
        release: &CallerPublicationRecord,
    ) -> Result<(), FleetActivationOpsError> {
        let mut record = FleetActivation::get().ok_or(FleetActivationOpsError::NotInitialized)?;
        validate_release(&record, release)
            .map_err(|_| FleetActivationOpsError::EvidenceMismatch)?;
        let activation = record
            .component_runtime
            .as_mut()
            .and_then(|runtime| runtime.activation.as_mut())
            .ok_or(FleetActivationOpsError::NotActive)?;
        if activation.startup.release.as_ref() != Some(release) {
            return Err(FleetActivationOpsError::EvidenceMismatch);
        }
        activation.startup.arguments = None;
        activation.startup.initialized = true;
        replace_record(record)
    }
}

fn startup(record: &FleetActivationView) -> Option<&ApplicationStartupRecord> {
    record
        .component_runtime
        .as_ref()?
        .activation
        .as_ref()
        .map(|activation| &activation.startup)
}

fn require_open(
    record: &FleetActivationView,
    release: &CallerPublicationRecord,
) -> Result<(), CallerAdmissionError> {
    validate_installation(record, release)?;
    let receiver =
        CallerAuthorityOps::receiver().ok_or(CallerAdmissionError::AuthorityUnavailable)?;
    if receiver.authority != release.authority || receiver.generation < release.generation {
        return Err(CallerAdmissionError::AuthorityConflict);
    }
    if !receiver.open || receiver.retired {
        return Err(CallerAdmissionError::Fenced);
    }
    Ok(())
}

fn validate_release(
    record: &FleetActivationView,
    release: &CallerPublicationRecord,
) -> Result<(), CallerAdmissionError> {
    validate_installation(record, release)?;
    let receipt = CallerAuthorityOps::receipt(release.operation_id)
        .ok_or(CallerAdmissionError::AuthorityUnavailable)?;
    let receiver =
        CallerAuthorityOps::receiver().ok_or(CallerAdmissionError::AuthorityUnavailable)?;
    let exact_receipt =
        receipt.publication == *release && receipt.phase == CallerReceiptPhase::Complete;
    let current_authority =
        receiver.authority == release.authority && receiver.generation >= release.generation;
    if !exact_receipt || !current_authority || release.change != CallerChangeRecord::OpenReceiver {
        return Err(CallerAdmissionError::AuthorityConflict);
    }
    Ok(())
}

#[derive(Eq, PartialEq)]
struct StartupInstallationAuthority<'a> {
    binding: &'a crate::ids::ManagedCanisterBinding,
    own_installation: [u8; 32],
    component_installation: [u8; 32],
}

fn validate_installation(
    record: &FleetActivationView,
    release: &CallerPublicationRecord,
) -> Result<(), CallerAdmissionError> {
    let FleetActivationStateRecord::Active { identity, .. } = &record.state else {
        return Err(CallerAdmissionError::Fenced);
    };
    let runtime = record
        .component_runtime
        .as_ref()
        .ok_or(CallerAdmissionError::AuthorityConflict)?;
    let receiver = &release.authority.receiver;
    let installed = StartupInstallationAuthority {
        binding: &runtime.binding,
        own_installation: identity.operation_id,
        component_installation: runtime.component_install_id,
    };
    let released = StartupInstallationAuthority {
        binding: &receiver.binding,
        own_installation: receiver.install_id,
        component_installation: receiver.component_install_id,
    };
    let installation_matches = installed == released;
    let root_matches = release.authority.issuer.install_id == runtime.root_install_id
        && release.authority.issuer.root == receiver.component().fleet_subnet_root
        && release.authority.issuer.registry == receiver.component().authority;
    if !installation_matches || !root_matches || release.generation == 0 {
        return Err(CallerAdmissionError::AuthorityConflict);
    }
    Ok(())
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::{
        config::caller_authority::CompiledCallerPolicy,
        storage::stable::caller_authority::CallerReceiverStore,
    };
    use canic_contracts::ids::{CallerInstallation, CallerReceiverAuthority, CallerRootAuthority};

    pub(in crate::ops::storage::fleet_activation) fn assert_startup_release() {
        CallerReceiverStore::reset();
        let record = FleetActivation::get().unwrap();
        let FleetActivationStateRecord::Active { identity, .. } = &record.state else {
            panic!("active fixture")
        };
        let runtime = record.component_runtime.as_ref().unwrap();
        let receiver = CallerInstallation {
            binding: runtime.binding.clone(),
            install_id: identity.operation_id,
            component_install_id: runtime.component_install_id,
        };
        let policy = CompiledCallerPolicy::compile(receiver.role().clone(), None).unwrap();
        let authority = CallerReceiverAuthority {
            issuer: CallerRootAuthority {
                registry: receiver.component().authority.clone(),
                root: receiver.component().fleet_subnet_root,
                install_id: runtime.root_install_id,
            },
            receiver,
            policy_digest: policy.digest,
        };
        CallerAuthorityOps::initialize(authority.clone()).unwrap();
        assert_eq!(
            FleetActivationOps::application_startup_work().unwrap(),
            None
        );
        assert_eq!(
            FleetActivationOps::require_application_started(),
            Err(CallerAdmissionError::Fenced)
        );
        let release = CallerAuthorityOps::publication(
            authority.clone(),
            [71; 32],
            0,
            CallerChangeRecord::OpenReceiver,
        )
        .unwrap();
        assert_eq!(
            FleetActivationOps::release_application_startup(release.clone()),
            Err(FleetActivationOpsError::EvidenceMismatch)
        );
        crate::workflow::caller_authority::prepare(release.clone(), &policy).unwrap();
        CallerAuthorityOps::commit(&release).unwrap();
        assert_eq!(
            FleetActivationOps::release_application_startup(release.clone()),
            Err(FleetActivationOpsError::EvidenceMismatch)
        );
        CallerAuthorityOps::complete(&release).unwrap();
        FleetActivationOps::release_application_startup(release.clone()).unwrap();
        super::super::codec::forget_projection();
        FleetActivationOps::restore_application_startup().unwrap();
        let work = FleetActivationOps::application_startup_work()
            .unwrap()
            .unwrap();
        assert_eq!(work.arguments, Some(vec![45, 46]));
        assert_eq!(work.release, release);
        assert_eq!(
            FleetActivationOps::require_application_started(),
            Err(CallerAdmissionError::Fenced)
        );
        let mut changed = release.clone();
        changed.authority.issuer.install_id = [99; 32];
        assert_eq!(
            FleetActivationOps::complete_application_startup(&changed),
            Err(FleetActivationOpsError::EvidenceMismatch)
        );
        FleetActivationOps::complete_application_startup(&release).unwrap();
        FleetActivationOps::require_application_started().unwrap();
        super::super::codec::forget_projection();
        FleetActivationOps::restore_application_startup().unwrap();
        assert_eq!(
            FleetActivationOps::application_startup_work().unwrap(),
            None
        );
        let retire = CallerAuthorityOps::publication(
            authority,
            [72; 32],
            1,
            CallerChangeRecord::RetireReceiver,
        )
        .unwrap();
        crate::workflow::caller_authority::prepare(retire.clone(), &policy).unwrap();
        assert_eq!(
            FleetActivationOps::require_application_started(),
            Err(CallerAdmissionError::Fenced)
        );
        CallerAuthorityOps::commit(&retire).unwrap();
        CallerAuthorityOps::complete(&retire).unwrap();
        FleetActivationOps::release_application_startup(release).unwrap();
        super::super::codec::forget_projection();
        FleetActivationOps::restore_application_startup().unwrap();
        assert_eq!(
            FleetActivationOps::application_startup_work().unwrap(),
            None
        );
        assert_eq!(
            FleetActivationOps::require_application_started(),
            Err(CallerAdmissionError::Fenced)
        );
        CallerReceiverStore::reset();
    }
}
