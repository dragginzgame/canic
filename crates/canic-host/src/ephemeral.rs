//! Run-scoped simulator and deployment journal for installed fleet tests.
use crate::{Config, Error, FleetLifetime, Result, supplier};
use ic_testkit::{
    pocket_ic::PocketIcBuilder,
    server::{Deletion, Instance, Limits, PreparedServer, Server},
};
use std::{path::Path, time::Duration};

const TIMEOUT: Duration = Duration::from_secs(30);

/// An isolated simulator and private journal. Dropping the owner releases both.
pub struct EphemeralFleet {
    instance: Instance,
    server: Server,
    journal: tempfile::TempDir,
}

impl EphemeralFleet {
    /// Start a session for an explicitly ephemeral fleet in a validated declaration.
    /// # Errors
    /// Refuses missing declarations, invalid server assets and startup failures.
    pub async fn start(config: &Config, app: &str, fleet: &str, directory: &Path) -> Result<Self> {
        let (_, app) = config.select_app(Some(app))?;
        if app.fleets.get(fleet).and_then(|f| f.lifetime) != Some(FleetLifetime::Ephemeral) {
            return Err(Error::Invalid(
                "select a fleet with ephemeral lifetime".into(),
            ));
        }

        let journal = tempfile::tempdir().map_err(supplier)?;
        let server = Server::start(
            PreparedServer::check(directory).map_err(supplier)?,
            Limits {
                startup: TIMEOUT,
                lifetime: Duration::from_secs(120),
                stdout_bytes: 1024 * 1024,
                stderr_bytes: 1024 * 1024,
            },
        )
        .map_err(supplier)?;
        let topology = PocketIcBuilder::new()
            .with_nns_subnet()
            .with_application_subnet();
        let instance = server.instance(topology, TIMEOUT).await.map_err(supplier)?;

        Ok(Self {
            instance,
            server,
            journal,
        })
    }

    /// Access the owned simulator for installation, gateway setup and calls.
    pub fn instance(&mut self) -> &mut Instance {
        &mut self.instance
    }

    /// Absolute journal directory to bind in the run's in-memory configuration.
    #[must_use]
    pub fn journal(&self) -> &Path {
        self.journal.path()
    }

    /// Close the instance and reap its dedicated server before removing the journal.
    /// # Errors
    /// Reports incomplete instance shutdown, server failures or journal cleanup failures.
    pub async fn close(self) -> Result<()> {
        let Self {
            instance,
            mut server,
            journal,
        } = self;
        let deletion = instance.close(TIMEOUT).await;
        let stopped = server.stop().map_err(supplier)?;
        journal.close().map_err(supplier)?;
        if deletion != Deletion::Attempted || stopped.abandoned != 0 {
            return Err(Error::Invalid("incomplete ephemeral fleet shutdown".into()));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires a prepared Testkit server and loopback"]
    async fn unwinding_reaps_server_and_removes_run_journal() {
        let config: Config = toml::from_str("[apps.a.canisters.hub]\n[apps.a.fleets.test]\nlifetime = 'ephemeral'\nnetwork = { kind = 'Local' }").unwrap();
        let prepared = std::env::var_os("IC_TESTKIT_SERVER_DIRECTORY").unwrap();
        let run = EphemeralFleet::start(&config, "a", "test", Path::new(&prepared))
            .await
            .unwrap();
        let journal = run.journal().to_owned();
        std::fs::write(journal.join("operation"), b"pending").unwrap();
        let endpoint = url::Url::parse(run.server.endpoint()).unwrap();
        let address = endpoint.socket_addrs(|| None).unwrap()[0];
        assert!(std::net::TcpStream::connect(address).is_ok());

        let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            let _owner = run;
            panic!("simulate an interrupted test");
        }));
        assert!(failed.is_err());
        assert!(!journal.exists());
        assert!(std::net::TcpStream::connect(address).is_err());
    }
}
