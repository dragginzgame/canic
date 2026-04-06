use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Duration;

use candid::Principal;
use canic::{Error, ids::CanisterRole};

use super::{INSTALL_CYCLES, Pic, PicInstallError, install_args, install_root_args, startup};

impl Pic {
    /// Install a root canister with the default root init arguments.
    pub fn create_and_install_root_canister(&self, wasm: Vec<u8>) -> Result<Principal, Error> {
        let init_bytes = install_root_args()?;

        Ok(self.create_and_install_with_args(wasm, init_bytes, INSTALL_CYCLES))
    }

    /// Install a canister with the given type and wasm bytes.
    ///
    /// Install failures are treated as fatal in tests.
    pub fn create_and_install_canister(
        &self,
        role: CanisterRole,
        wasm: Vec<u8>,
    ) -> Result<Principal, Error> {
        let init_bytes = install_args(role)?;

        Ok(self.create_and_install_with_args(wasm, init_bytes, INSTALL_CYCLES))
    }

    /// Install one arbitrary wasm module with caller-provided init bytes.
    ///
    /// This is the generic install path for downstreams that use `canic-testkit`
    /// without depending on Canic canister init payload conventions.
    #[must_use]
    pub fn create_and_install_with_args(
        &self,
        wasm: Vec<u8>,
        init_bytes: Vec<u8>,
        install_cycles: u128,
    ) -> Principal {
        self.try_create_and_install_with_args(wasm, init_bytes, install_cycles)
            .unwrap_or_else(|err| panic!("{err}"))
    }

    /// Install one arbitrary wasm module with caller-provided init bytes.
    pub fn try_create_and_install_with_args(
        &self,
        wasm: Vec<u8>,
        init_bytes: Vec<u8>,
        install_cycles: u128,
    ) -> Result<Principal, PicInstallError> {
        self.try_create_funded_and_install(wasm, init_bytes, install_cycles)
    }

    /// Wait until one canister reports `canic_ready`.
    pub fn wait_for_ready(&self, canister_id: Principal, tick_limit: usize, context: &str) {
        for _ in 0..tick_limit {
            self.tick();
            if self.fetch_ready(canister_id) {
                return;
            }
        }

        self.dump_canister_debug(canister_id, context);
        panic!("{context}: canister {canister_id} did not become ready after {tick_limit} ticks");
    }

    /// Wait until all provided canisters report `canic_ready`.
    pub fn wait_for_all_ready<I>(&self, canister_ids: I, tick_limit: usize, context: &str)
    where
        I: IntoIterator<Item = Principal>,
    {
        let canister_ids = canister_ids.into_iter().collect::<Vec<_>>();

        for _ in 0..tick_limit {
            self.tick();
            if canister_ids
                .iter()
                .copied()
                .all(|canister_id| self.fetch_ready(canister_id))
            {
                return;
            }
        }

        for canister_id in &canister_ids {
            self.dump_canister_debug(*canister_id, context);
        }
        panic!("{context}: canisters did not become ready after {tick_limit} ticks");
    }

    /// Wait out the PocketIC `install_code` cooldown window inside the same instance.
    pub fn wait_out_install_code_rate_limit(&self, cooldown: Duration) {
        self.advance_time(cooldown);
        self.tick_n(2);
    }

    /// Retry one install_code-like operation while PocketIC still reports rate limiting.
    pub fn retry_install_code_ok<T, F>(
        &self,
        retry_limit: usize,
        cooldown: Duration,
        mut op: F,
    ) -> Result<T, String>
    where
        F: FnMut() -> Result<T, String>,
    {
        let mut last_err = None;

        for _ in 0..retry_limit {
            match op() {
                Ok(value) => return Ok(value),
                Err(err) if is_install_code_rate_limited(&err) => {
                    last_err = Some(err);
                    self.wait_out_install_code_rate_limit(cooldown);
                }
                Err(err) => return Err(err),
            }
        }

        Err(last_err.unwrap_or_else(|| "install_code retry loop exhausted".to_string()))
    }

    /// Retry one install_code-like failure path while PocketIC still reports rate limiting.
    pub fn retry_install_code_err<F>(
        &self,
        retry_limit: usize,
        cooldown: Duration,
        first: Result<(), String>,
        mut op: F,
    ) -> Result<(), String>
    where
        F: FnMut() -> Result<(), String>,
    {
        match first {
            Ok(()) => return Ok(()),
            Err(err) if !is_install_code_rate_limited(&err) => return Err(err),
            Err(_) => {}
        }

        self.wait_out_install_code_rate_limit(cooldown);

        for _ in 1..retry_limit {
            match op() {
                Ok(()) => return Ok(()),
                Err(err) if is_install_code_rate_limited(&err) => {
                    self.wait_out_install_code_rate_limit(cooldown);
                }
                Err(err) => return Err(err),
            }
        }

        op()
    }

    // Install a canister after creating it and funding it with cycles.
    fn try_create_funded_and_install(
        &self,
        wasm: Vec<u8>,
        init_bytes: Vec<u8>,
        install_cycles: u128,
    ) -> Result<Principal, PicInstallError> {
        let canister_id = self.create_canister();
        self.add_cycles(canister_id, install_cycles);

        let install = catch_unwind(AssertUnwindSafe(|| {
            self.inner
                .install_canister(canister_id, wasm, init_bytes, None);
        }));
        if let Err(payload) = install {
            eprintln!("install_canister trapped for {canister_id}");
            if let Ok(status) = self.inner.canister_status(canister_id, None) {
                eprintln!("canister_status for {canister_id}: {status:?}");
            }
            if let Ok(logs) = self
                .inner
                .fetch_canister_logs(canister_id, Principal::anonymous())
            {
                for record in logs {
                    eprintln!("canister_log {canister_id}: {record:?}");
                }
            }
            return Err(PicInstallError::new(
                canister_id,
                startup::panic_payload_to_string(payload.as_ref()),
            ));
        }

        Ok(canister_id)
    }
}

fn is_install_code_rate_limited(message: &str) -> bool {
    message.contains("CanisterInstallCodeRateLimited")
}
