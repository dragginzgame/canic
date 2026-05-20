use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Duration;

use candid::Principal;

use super::{Pic, PicInstallError, startup};

impl Pic {
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
