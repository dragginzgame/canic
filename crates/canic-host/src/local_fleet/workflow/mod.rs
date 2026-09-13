//! Persistent local instance orchestration; Fleet convergence remains a separate existing owner.

use crate::local_fleet::{
    LocalFleetError,
    model::{
        LocalAllocationInput, LocalAllocationRecord, LocalCheckpoint, LocalFleetConfig,
        LocalFleetRecord,
    },
    ops::{self, runtime},
    policy,
    view::LocalFleetView,
};
use ic_testkit::{pic::PocketIcManagedServer, pocket_ic::PocketIc};
use std::{
    fs::File,
    path::{Path, PathBuf},
};

/// Exclusive public developer harness. Drop flushes the instance before terminating its server.
pub struct LocalFleetSession {
    directory: PathBuf,
    record: LocalFleetRecord,
    gateway: String,
    pic: Option<PocketIc>,
    server: Option<PocketIcManagedServer>,
    lock: Option<File>,
    ready: bool,
}

impl LocalFleetSession {
    /// Open only this workspace's exact named current-release local environment.
    pub fn open(root: &Path, config: &LocalFleetConfig) -> Result<Self, LocalFleetError> {
        policy::validate(config)?;
        let (directory, lock) = ops::lock_directory(root, &config.name)?;
        ops::validate_tree(&directory)?;
        ops::reset::require_complete(&directory)?;
        let previous = ops::read_record(&directory)?;
        if previous.as_ref().is_some_and(|record| {
            record.configuration != *config || record.canic_version != env!("CARGO_PKG_VERSION")
        }) {
            return Err(LocalFleetError::Identity);
        }
        if previous.is_none() && directory.join("preparation.json").exists() {
            return Err(LocalFleetError::Identity);
        }
        if previous
            .as_ref()
            .is_some_and(|record| record.checkpoint != LocalCheckpoint::Saved)
        {
            return Err(LocalFleetError::UncleanCheckpoint);
        }
        let initial = previous.is_none();
        let record = if let Some(record) = previous {
            record
        } else {
            let record = ops::initial_record(config)?;
            ops::write_record(&directory, &record)?;
            record
        };
        if ops::binary_sha256(&config.server_binary)? != config.server_binary_sha256 {
            return Err(LocalFleetError::Identity);
        }
        let instance_directory = ops::instance_directory(&directory, &record.session_id)?;
        if initial && instance_directory.exists() {
            return Err(LocalFleetError::Identity);
        }
        if !initial && !instance_directory.is_dir() {
            return Err(LocalFleetError::UncleanCheckpoint);
        }
        let record = ops::checkpoint(&directory, &record, LocalCheckpoint::Running)?;
        let server = runtime::server(config)?;
        let mut session = Self {
            directory,
            record,
            gateway: String::new(),
            pic: None,
            server: Some(server),
            lock: Some(lock),
            ready: false,
        };
        let pic = runtime::instance(
            config,
            &instance_directory,
            session.server.as_ref().ok_or(LocalFleetError::Session)?,
        )?;
        session.pic = Some(pic);
        let pic = session.pic.as_ref().ok_or(LocalFleetError::Session)?;
        session.record = runtime::guarded(|| {
            if initial {
                ops::admit_initial_instance(&session.record, pic)
            } else {
                ops::instance_record(config, Some(session.record.clone()), pic)
            }
        })??;
        session.record = ops::checkpoint(
            &session.directory,
            &session.record,
            LocalCheckpoint::Running,
        )?;
        session.gateway = runtime::gateway(
            session.pic.as_mut().ok_or(LocalFleetError::Session)?,
            config.gateway_port,
        )?;
        session.ready = true;
        Ok(session)
    }

    /// Discard one explicitly selected local session; retries cannot reset a later replacement.
    pub fn reset(root: &Path, name: &str, expected_session: &str) -> Result<(), LocalFleetError> {
        let (directory, _lock) = ops::lock_directory(root, name)?;
        if ops::reset::begin(&directory, expected_session)? {
            ops::reset::remove_instance(&directory)?;
            ops::reset::finish(&directory, expected_session)?;
        }
        Ok(())
    }

    /// Reserve and create one new local identity, with exact same-session retry.
    pub fn allocate(
        &mut self,
        input: &LocalAllocationInput,
    ) -> Result<LocalAllocationRecord, LocalFleetError> {
        self.require_active()?;
        policy::validate_allocation(input, &self.record.configuration)?;
        let pic = self.pic.as_ref().ok_or(LocalFleetError::Session)?;
        let now = runtime::guarded(|| pic.get_time().as_nanos_since_unix_epoch())?;
        let (next, intent) = ops::reserve(&self.directory, &self.record, input, now)?;
        self.record = next;
        let id = runtime::create(pic, &intent, &self.record.configuration)?;
        let (next, allocation) =
            ops::complete_allocation(&self.directory, &self.record, &intent, id)?;
        self.record = next;
        Ok(allocation)
    }

    /// Generate current desired state through the exact owned gateway and enrolled local trust.
    pub fn generate_fleet(
        &self,
        request: &crate::fleet_ensure::FleetGenerateRequest<'_>,
    ) -> Result<crate::fleet_ensure::GeneratedDesiredFleet, LocalFleetError> {
        self.require_active()?;
        ops::prepare::enroll(
            &self.directory,
            request.root,
            request.environment,
            &self.record,
        )?;
        crate::fleet_ensure::generate_local_fleet(request, &self.replica_target())
            .map_err(|error| LocalFleetError::Preparation(error.to_string()))
    }

    /// Prepare an explicitly fresh, release-bound local Fleet for ordinary Ensure convergence.
    pub fn prepare_fleet(
        &mut self,
        workspace: &Path,
        source: &crate::fleet_ensure::model::DesiredFleet,
    ) -> Result<crate::fleet_ensure::LoadedDesiredFleet, LocalFleetError> {
        self.require_active()?;
        let mut preparation =
            ops::prepare::begin(&self.directory, workspace, &self.record, source)?;
        ops::bindings::stage(workspace, source)?;
        for configured in &source.canisters {
            let input = ops::prepare::allocation_input(&self.record, source, &configured.name)?;
            self.allocate(&input)?;
        }
        preparation = ops::prepare::bind(&self.directory, &self.record, source, &preparation)?;
        let prepared = preparation
            .prepared
            .as_ref()
            .ok_or(LocalFleetError::Identity)?;
        let pic = self.pic.as_ref().ok_or(LocalFleetError::Session)?;
        for configured in &prepared.canisters {
            let target = ops::prepare::controllers(prepared, &configured.name)?;
            runtime::controllers(
                pic,
                target.canister_id,
                target.operator,
                &target.controllers,
            )?;
        }
        for target in ops::prepare::root_installations(&preparation)? {
            let install_needed = runtime::root_needs_install(pic, &target)?;
            preparation = ops::prepare::install_intent(
                &self.directory,
                &preparation,
                &target,
                install_needed,
            )?;
            if install_needed {
                runtime::install_root(pic, &target)?;
            }
            runtime::verify_root_authority(pic, &target)?;
            preparation = ops::prepare::verified_root(&self.directory, &preparation, &target.name)?;
        }
        self.record = ops::prepare::finish(&self.directory, &self.record, &preparation)?;
        ops::prepare::loaded(&preparation)
    }

    /// Converge only the exact prepared local input through the existing Fleet journal and adapter.
    pub fn converge_fleet(
        &mut self,
        workspace: &Path,
        source: &crate::fleet_ensure::model::DesiredFleet,
        icp_executable: &str,
    ) -> Result<crate::fleet_ensure::FleetEnsureReport, LocalFleetError> {
        let loaded = self.prepare_fleet(workspace, source)?;
        let mut platform = crate::fleet_ensure::IcpEnsurePlatform::new(
            loaded.desired.clone(),
            icp_executable,
            workspace,
        )
        .with_local_replica(self.replica_target());
        let now = runtime::guarded(|| {
            self.pic
                .as_ref()
                .ok_or(LocalFleetError::Session)
                .map(|pic| pic.get_time().as_nanos_since_unix_epoch())
        })??;
        let report = crate::fleet_ensure::plan(
            workspace,
            &loaded.desired,
            &loaded.sha256,
            &loaded.desired.fleet,
            now,
            &mut platform,
        )
        .map_err(|error| LocalFleetError::Ensure(Box::new(error)))?;
        crate::fleet_ensure::apply(
            workspace,
            &loaded.desired,
            &loaded.sha256,
            &loaded.desired.fleet,
            &report.plan.plan_sha256,
            &mut platform,
        )
        .map_err(|error| LocalFleetError::Ensure(Box::new(error)))
    }

    /// Bind the maintained ICP transport to this exact loopback gateway and trust key.
    #[must_use]
    pub fn replica_target(&self) -> crate::icp::LocalReplicaTarget {
        crate::icp::LocalReplicaTarget {
            environment: ops::environment_name(&self.record),
            url: self.gateway.clone(),
            root_key: self.record.root_key_der_hex.clone(),
        }
    }

    /// Return local discovery and observed native cycles without querying external networks.
    pub fn status(&self) -> Result<LocalFleetView, LocalFleetError> {
        self.require_active()?;
        let pic = self.pic.as_ref().ok_or(LocalFleetError::Session)?;
        runtime::guarded(|| ops::status(&self.record, pic, &self.gateway))
    }

    /// Resolve current Fleet roles through the maintained terminal inventory and observed subnets.
    pub fn discover(
        &self,
        workspace: &Path,
        fleet: &str,
    ) -> Result<crate::local_fleet::view::LocalFleetDiscoveryView, LocalFleetError> {
        ops::prepare::require_workspace(&self.directory, workspace)?;
        ops::discovery::resolve(workspace, fleet, self.status()?)
    }

    /// Flush and reopen the same state while retaining the same exclusive owner and gateway port.
    pub fn restart(&mut self, expected_session: &str) -> Result<(), LocalFleetError> {
        self.require_session(expected_session)?;
        self.require_active()?;
        self.flush()?;
        self.ready = false;
        self.record = ops::checkpoint(&self.directory, &self.record, LocalCheckpoint::Running)?;
        let server = self.server.as_ref().ok_or(LocalFleetError::Session)?;
        self.pic = Some(runtime::instance(
            &self.record.configuration,
            &ops::instance_directory(&self.directory, &self.record.session_id)?,
            server,
        )?);
        let pic = self.pic.as_ref().ok_or(LocalFleetError::Session)?;
        self.record = runtime::guarded(|| {
            ops::instance_record(&self.record.configuration, Some(self.record.clone()), pic)
        })??;
        self.record = ops::checkpoint(&self.directory, &self.record, LocalCheckpoint::Running)?;
        self.gateway = runtime::gateway(
            self.pic.as_mut().ok_or(LocalFleetError::Session)?,
            self.record.configuration.gateway_port,
        )?;
        self.ready = true;
        Ok(())
    }

    /// Pause the owned gateway, advance at most one simulated day, and resume on the same port.
    pub fn advance_time(
        &mut self,
        expected_session: &str,
        seconds: u32,
    ) -> Result<(), LocalFleetError> {
        self.require_session(expected_session)?;
        self.require_active()?;
        if seconds == 0 || seconds > 86_400 {
            return Err(LocalFleetError::Configuration);
        }
        let pic = self.pic.as_mut().ok_or(LocalFleetError::Session)?;
        runtime::stop_gateway(pic)?;
        runtime::advance(pic, seconds)?;
        self.gateway = runtime::gateway(pic, self.record.configuration.gateway_port)?;
        Ok(())
    }

    /// Shut down synchronously. No saved PID is signalled and no unrelated state is removed.
    pub fn shutdown(&mut self, expected_session: &str) -> Result<(), LocalFleetError> {
        self.require_session(expected_session)?;
        self.flush()?;
        self.ready = false;
        drop(self.server.take());
        drop(self.lock.take());
        Ok(())
    }

    const fn require_active(&self) -> Result<(), LocalFleetError> {
        if self.ready && self.pic.is_some() && self.server.is_some() {
            Ok(())
        } else {
            Err(LocalFleetError::Session)
        }
    }

    fn require_session(&self, expected: &str) -> Result<(), LocalFleetError> {
        if self.record.session_id == expected {
            Ok(())
        } else {
            Err(LocalFleetError::Session)
        }
    }

    fn flush(&mut self) -> Result<(), LocalFleetError> {
        if let Some(pic) = self.pic.take() {
            ops::checkpoint::save(
                pic,
                &mut self.server,
                std::time::Duration::from_secs(u64::from(
                    self.record.configuration.request_timeout_secs,
                )),
            )?;
            if self.ready {
                self.record =
                    ops::checkpoint(&self.directory, &self.record, LocalCheckpoint::Saved)?;
            }
        }
        Ok(())
    }
}

impl Drop for LocalFleetSession {
    fn drop(&mut self) {
        let _ = self.flush();
        drop(self.server.take());
    }
}
