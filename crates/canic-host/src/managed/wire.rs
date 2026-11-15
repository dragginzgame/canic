//! One selector for signed child methods and their retained response contracts.
use crate::{Plan, Result, network::principal, signed::Call, supplier};
use canic_management_protocol::{
    self as protocol,
    children::{self, Slot},
};
use serde::Serialize;

#[derive(Clone, Copy)]
pub(super) enum Method {
    Stage,
    Observe,
    Reconcile,
    StageUpgrade,
    ObserveUpgrade,
    ReconcileUpgrade,
}
impl Method {
    pub(super) fn id(self, plan: &Plan) -> u64 {
        self.contract(plan).1
    }
    fn contract(self, plan: &Plan) -> (&'static str, u64) {
        let scoped = plan.management().and_then(|owner| owner.slot).is_some();
        match (self, scoped) {
            (Self::Stage, false) => ("stage", protocol::STAGE),
            (Self::Stage, true) => ("stage_child", children::STAGE),
            (Self::Observe, false) => ("observe", protocol::STATUS),
            (Self::Observe, true) => ("observe_child", children::STATUS),
            (Self::Reconcile, false) => ("reconcile", protocol::RECONCILE),
            (Self::Reconcile, true) => ("reconcile_child", children::RECONCILE),
            (Self::StageUpgrade, false) => ("stage_upgrade", protocol::upgrade::STAGE),
            (Self::StageUpgrade, true) => ("stage_child_upgrade", children::STAGE_UPGRADE),
            (Self::ObserveUpgrade, false) => ("observe_upgrade", protocol::upgrade::OBSERVE),
            (Self::ObserveUpgrade, true) => ("observe_child_upgrade", children::OBSERVE_UPGRADE),
            (Self::ReconcileUpgrade, false) => ("reconcile_upgrade", protocol::upgrade::RECONCILE),
            (Self::ReconcileUpgrade, true) => {
                ("reconcile_child_upgrade", children::RECONCILE_UPGRADE)
            }
        }
    }
    pub(super) fn call<T: Serialize>(
        self,
        plan: &Plan,
        value: &T,
        limits: ic_wire::Limits,
    ) -> Result<Call<'static>> {
        let owner = plan
            .management()
            .ok_or_else(|| crate::Error::Invalid("missing child scope".into()))?;
        let root = principal(&owner.root)?;
        let (method, id) = self.contract(plan);
        let args = match owner.slot {
            Some(slot) if matches!(self, Self::Observe) => {
                ic_wire::native::encode_request(id, &slot, limits)
            }
            Some(slot) => {
                ic_wire::native::encode_request(id, &children::Request { slot, value }, limits)
            }
            None => ic_wire::native::encode_request(id, value, limits),
        }
        .map_err(supplier)?;
        Ok(Call {
            canister: root,
            effective: root,
            method,
            args,
        })
    }
}
pub(crate) fn observation(plan: &Plan) -> Result<Call<'static>> {
    match plan.action {
        crate::PlanAction::UpgradeChild { .. } => {
            Method::ObserveUpgrade.call(plan, &super::upgrade::approval(plan)?, protocol::LIMITS)
        }
        crate::PlanAction::CreateChild(_) => Method::Observe.call(plan, &(), protocol::LIMITS),
        _ => Err(crate::Error::Invalid(
            "expected Root-owned child observation".into(),
        )),
    }
}
// None is the historical First slot, but must retain its original wire contract.
pub(crate) fn same_slot(left: Option<Slot>, right: Option<Slot>) -> bool {
    left.unwrap_or(Slot::First) == right.unwrap_or(Slot::First)
}
