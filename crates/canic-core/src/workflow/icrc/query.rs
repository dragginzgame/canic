use crate::{
    cdk::spec::standards::icrc::icrc21::{ConsentMessageRequest, ConsentMessageResponse},
    config::Config,
    dispatch::icrc21::Icrc21Dispatcher,
    domain::icrc::icrc10::Icrc10Registry,
    ops::runtime::env::EnvOps,
};

///
/// Icrc10Query
///

pub struct Icrc10Query;

impl Icrc10Query {
    #[must_use]
    pub fn supported_standards() -> Vec<(String, String)> {
        let global_standards = Config::try_get().and_then(|cfg| cfg.standards.clone());
        let canister_standards = Config::try_get().and_then(|cfg| {
            let subnet_role = EnvOps::subnet_role().ok()?;
            let canister_role = EnvOps::canister_role().ok()?;

            cfg.subnets
                .get(&subnet_role)?
                .canisters
                .get(&canister_role)
                .map(|canister_cfg| canister_cfg.standards.clone())
        });

        let icrc21_enabled = global_standards
            .as_ref()
            .is_some_and(|standards| standards.icrc21)
            && canister_standards
                .as_ref()
                .is_some_and(|standards| standards.icrc21);
        let icrc103_enabled = global_standards
            .as_ref()
            .is_some_and(|standards| standards.icrc103);

        Icrc10Registry::supported_standards(icrc21_enabled, icrc103_enabled)
    }
}

///
/// Icrc21Query
///

pub struct Icrc21Query;

impl Icrc21Query {
    #[must_use]
    pub fn consent_message(req: ConsentMessageRequest) -> ConsentMessageResponse {
        Icrc21Dispatcher::consent_message(req)
    }
}
