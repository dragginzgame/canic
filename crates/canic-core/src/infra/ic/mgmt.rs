//! Infra-scoped IC helpers.
//!
//! These wrappers provide low-level IC management canister calls and common
//! ICC call patterns without layering concerns.

use crate::{
    cdk::{
        self,
        candid::{Principal, encode_args, utils::ArgumentEncoder},
        mgmt::{
            CanisterInstallMode, CanisterSettings, CanisterStatusArgs, CanisterStatusResult,
            CreateCanisterArgs, DeleteCanisterArgs, DepositCyclesArgs, InstallCodeArgs,
            UninstallCodeArgs, UpdateSettingsArgs, WasmModule,
        },
        types::Cycles,
    },
    infra::{InfraError, ic::IcInfraError, ic::call::Call},
};
use thiserror::Error as ThisError;

///
/// MgmtInfraError
///

#[derive(Debug, ThisError)]
pub enum MgmtInfraError {
    #[error("raw_rand returned {len} bytes")]
    RawRandInvalidLength { len: usize },
}

///
/// MgmtInfra
///

pub struct MgmtInfra;

impl MgmtInfra {
    /// Create a canister with explicit controllers and an initial cycle balance.
    pub async fn create_canister(
        controllers: Vec<Principal>,
        cycles: Cycles,
    ) -> Result<Principal, InfraError> {
        let settings = Some(CanisterSettings {
            controllers: Some(controllers),
            ..Default::default()
        });

        let args = CreateCanisterArgs { settings };

        let pid = cdk::mgmt::create_canister_with_extra_cycles(&args, cycles.to_u128())
            .await
            .map_err(IcInfraError::from)?
            .canister_id;

        Ok(pid)
    }

    // ────────────────────────────── CANISTER STATUS ──────────────────────────────

    /// Query the management canister for a canister's status.
    pub async fn canister_status(
        canister_pid: Principal,
    ) -> Result<CanisterStatusResult, InfraError> {
        let args = CanisterStatusArgs {
            canister_id: canister_pid,
        };

        let status = cdk::mgmt::canister_status(&args)
            .await
            .map_err(IcInfraError::from)?;

        Ok(status)
    }

    // ──────────────────────────────── CYCLES API ─────────────────────────────────

    /// Returns the local canister's cycle balance (cheap).
    #[must_use]
    pub fn canister_cycle_balance() -> Cycles {
        cdk::api::canister_cycle_balance().into()
    }

    /// Deposits cycles into a canister.
    pub async fn deposit_cycles(canister_pid: Principal, cycles: u128) -> Result<(), InfraError> {
        let args = DepositCyclesArgs {
            canister_id: canister_pid,
        };

        cdk::mgmt::deposit_cycles(&args, cycles)
            .await
            .map_err(IcInfraError::from)?;

        Ok(())
    }

    /// Gets a canister's cycle balance (expensive: calls mgmt canister).
    pub async fn get_cycles(canister_pid: Principal) -> Result<Cycles, InfraError> {
        let status = Self::canister_status(canister_pid).await?;
        Ok(status.cycles.into())
    }

    // ──────────────────────────────── RANDOMNESS ────────────────────────────────

    /// Query the management canister for raw randomness.
    pub async fn raw_rand() -> Result<[u8; 32], InfraError> {
        let response = Call::unbounded_wait(Principal::management_canister(), "raw_rand")
            .execute()
            .await?;

        let bytes: Vec<u8> = response.candid()?;
        let len = bytes.len();

        let seed: [u8; 32] = bytes
            .try_into()
            .map_err(|_| MgmtInfraError::RawRandInvalidLength { len })
            .map_err(IcInfraError::from)?;

        Ok(seed)
    }

    // ────────────────────────────── INSTALL / UNINSTALL ──────────────────────────

    /// Installs or upgrades a canister with the given wasm + args.
    pub async fn install_code<T: ArgumentEncoder>(
        mode: CanisterInstallMode,
        canister_pid: Principal,
        wasm: &[u8],
        args: T,
    ) -> Result<(), InfraError> {
        let arg = encode_args(args).map_err(IcInfraError::from)?;

        let install_args = InstallCodeArgs {
            mode,
            canister_id: canister_pid,
            wasm_module: WasmModule::from(wasm),
            arg,
        };

        cdk::mgmt::install_code(&install_args)
            .await
            .map_err(IcInfraError::from)?;

        Ok(())
    }

    /// Upgrades a canister to the provided wasm.
    pub async fn upgrade_canister(canister_pid: Principal, wasm: &[u8]) -> Result<(), InfraError> {
        Self::install_code(CanisterInstallMode::Upgrade(None), canister_pid, wasm, ()).await
    }

    /// Uninstalls code from a canister.
    pub async fn uninstall_code(canister_pid: Principal) -> Result<(), InfraError> {
        let args = UninstallCodeArgs {
            canister_id: canister_pid,
        };

        cdk::mgmt::uninstall_code(&args)
            .await
            .map_err(IcInfraError::from)?;

        Ok(())
    }

    /// Deletes a canister (code + controllers) via the management canister.
    pub async fn delete_canister(canister_pid: Principal) -> Result<(), InfraError> {
        let args = DeleteCanisterArgs {
            canister_id: canister_pid,
        };

        cdk::mgmt::delete_canister(&args)
            .await
            .map_err(IcInfraError::from)?;

        Ok(())
    }

    // ─────────────────────────────── SETTINGS API ────────────────────────────────

    /// Updates canister settings via the management canister.
    pub async fn update_settings(args: &UpdateSettingsArgs) -> Result<(), InfraError> {
        cdk::mgmt::update_settings(args)
            .await
            .map_err(IcInfraError::from)?;

        Ok(())
    }
}
