//! Build explicit ICRC selections and install them through the managed-App owner.

use super::artifacts::{
    CanicWasmBuildProfile, INTERNAL_TEST_RELEASE_BUILD_ID, InternalTestWasms,
    build_internal_test_wasm_canisters_with_features,
};
use canic::testing::{ManagedAppFixture, ManagedAppQualificationInput, install_managed_app};
use std::{collections::BTreeMap, path::Path, sync::OnceLock};

const PACKAGE: &str = "managed_lifecycle_probe";
const CONFIG: &str =
    include_str!("../../../../../canisters/test/managed_lifecycle_probe/canic.toml");
static BUILDS: OnceLock<BTreeMap<MountProfile, InternalTestWasms>> = OnceLock::new();

///
/// MountProfile
///
/// Fixture-owned feature selections; enabling the adapter and mounting it are distinct.
///

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum MountProfile {
    Default,
    FeatureOnly,
    Discovery,
    Consent,
}

impl MountProfile {
    /// Every maintained selection exercised by the installed adapter contract.
    pub const ALL: &[Self] = &[
        Self::Default,
        Self::FeatureOnly,
        Self::Discovery,
        Self::Consent,
    ];

    const fn feature(self) -> Option<&'static str> {
        match self {
            Self::Default => None,
            Self::FeatureOnly => Some("managed_lifecycle_probe/icrc"),
            Self::Discovery => Some("managed_lifecycle_probe/icrc-discovery"),
            Self::Consent => Some("managed_lifecycle_probe/icrc-consent"),
        }
    }
}

///
/// IcrcFixture
///
/// Exact inspected artifact and the managed-App fixture installed from those bytes.
///

pub struct IcrcFixture {
    pub application: ManagedAppFixture,
    pub wasm: Vec<u8>,
}

/// Install one selected profile with the ordinary framework lifecycle and authority.
///
/// # Panics
/// Panics if the registered fixture cannot be built or installed by its owner.
#[must_use]
pub fn install(profile: MountProfile) -> IcrcFixture {
    let builds = BUILDS.get_or_init(|| {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        MountProfile::ALL
            .iter()
            .copied()
            .map(|profile| {
                let features = profile
                    .feature()
                    .map(str::to_owned)
                    .into_iter()
                    .collect::<Vec<_>>();
                let wasms = build_internal_test_wasm_canisters_with_features(
                    &workspace,
                    &workspace.join("target/pic-icrc-wasm"),
                    &[PACKAGE],
                    CanicWasmBuildProfile::Fast,
                    &[],
                    &features,
                );
                (profile, wasms)
            })
            .collect()
    });
    let wasm = builds[&profile].wasm(PACKAGE);
    let application = install_managed_app(ManagedAppQualificationInput::new(
        CONFIG,
        "test",
        "test",
        INTERNAL_TEST_RELEASE_BUILD_ID.1,
        vec![ic_testkit::fake_principal(15)],
        wasm.clone(),
    ))
    .expect("install selected ICRC managed-App fixture");
    IcrcFixture { application, wasm }
}
