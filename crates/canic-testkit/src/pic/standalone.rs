use super::{
    Pic, PicSerialGuard, StandaloneCanisterFixtureError, try_acquire_pic_serial_guard, try_pic,
};

const STANDALONE_INSTALL_CYCLES: u128 = 1_000_000_000_000;

///
/// StandaloneCanisterFixture
///

pub struct StandaloneCanisterFixture {
    pic: Pic,
    canister_id: candid::Principal,
    _serial_guard: PicSerialGuard,
}

impl StandaloneCanisterFixture {
    /// Borrow the PocketIC instance that owns this standalone fixture.
    #[must_use]
    pub const fn pic(&self) -> &Pic {
        &self.pic
    }

    /// Mutably borrow the PocketIC instance that owns this standalone fixture.
    #[must_use]
    pub const fn pic_mut(&mut self) -> &mut Pic {
        &mut self.pic
    }

    /// Read the installed canister id for this standalone fixture.
    #[must_use]
    pub const fn canister_id(&self) -> candid::Principal {
        self.canister_id
    }

    /// Consume the fixture and return the owned PocketIC instance and canister id.
    #[must_use]
    pub fn into_parts(self) -> (Pic, candid::Principal) {
        (self.pic, self.canister_id)
    }
}

// Install one already-built wasm module into a fresh PocketIC instance with
// caller-provided init args and no Canic-specific bootstrap assumptions.
#[must_use]
pub fn install_prebuilt_canister(wasm: Vec<u8>, init_bytes: Vec<u8>) -> StandaloneCanisterFixture {
    try_install_prebuilt_canister(wasm, init_bytes)
        .unwrap_or_else(|err| panic!("failed to install prebuilt canister fixture: {err}"))
}

// Install one already-built wasm module into a fresh PocketIC instance with
// caller-provided init args and no Canic-specific bootstrap assumptions.
pub fn try_install_prebuilt_canister(
    wasm: Vec<u8>,
    init_bytes: Vec<u8>,
) -> Result<StandaloneCanisterFixture, StandaloneCanisterFixtureError> {
    try_install_prebuilt_canister_with_cycles(wasm, init_bytes, STANDALONE_INSTALL_CYCLES)
}

// Install one already-built wasm module into a fresh PocketIC instance with
// caller-provided init args and explicit install cycles.
#[must_use]
pub fn install_prebuilt_canister_with_cycles(
    wasm: Vec<u8>,
    init_bytes: Vec<u8>,
    install_cycles: u128,
) -> StandaloneCanisterFixture {
    try_install_prebuilt_canister_with_cycles(wasm, init_bytes, install_cycles)
        .unwrap_or_else(|err| panic!("failed to install prebuilt canister fixture: {err}"))
}

// Install one already-built wasm module into a fresh PocketIC instance with
// caller-provided init args and explicit install cycles.
pub fn try_install_prebuilt_canister_with_cycles(
    wasm: Vec<u8>,
    init_bytes: Vec<u8>,
    install_cycles: u128,
) -> Result<StandaloneCanisterFixture, StandaloneCanisterFixtureError> {
    let serial_guard =
        try_acquire_pic_serial_guard().map_err(StandaloneCanisterFixtureError::SerialGuard)?;
    let pic = try_pic().map_err(StandaloneCanisterFixtureError::Start)?;
    let canister_id = pic
        .try_create_and_install_with_args(wasm, init_bytes, install_cycles)
        .map_err(StandaloneCanisterFixtureError::Install)?;

    Ok(StandaloneCanisterFixture {
        pic,
        canister_id,
        _serial_guard: serial_guard,
    })
}
