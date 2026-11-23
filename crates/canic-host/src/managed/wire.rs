//! One selector for signed child methods and their retained response contracts.
use crate::{Plan, Result, network::principal, signed::Call, supplier};
use canic_management_protocol::{
    self as protocol,
    children::{self, ChildId},
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
    ActivateUpgrade,
}
impl Method {
    pub(super) fn id(self, plan: &Plan) -> u64 {
        self.contract(plan).1
    }

    fn contract(self, plan: &Plan) -> (&'static str, u64) {
        if plan.version == 8 && matches!(self, Self::Observe | Self::ObserveUpgrade) {
            return (
                "observe_role_children",
                protocol::application::CHILD_EVIDENCE,
            );
        }
        if plan.management().is_some_and(|m| m.arguments.is_some()) {
            use protocol::application;
            return match self {
                Self::Stage => ("stage_application_child", application::STAGE),
                Self::Reconcile => ("reconcile_application_child", application::RECONCILE),
                Self::Observe | Self::ObserveUpgrade => {
                    ("observe_application_children", application::CUSTODY)
                }
                Self::StageUpgrade => ("stage_application_upgrade", application::STAGE_UPGRADE),
                Self::ReconcileUpgrade => (
                    "reconcile_application_upgrade",
                    application::RECONCILE_UPGRADE,
                ),
                Self::ActivateUpgrade => ("activate_application_upgrade", application::ACTIVATE),
            };
        }
        if plan.management().is_some_and(|m| m.source.is_some()) {
            use protocol::stored;
            return match self {
                Self::ActivateUpgrade => {
                    unreachable!("activation requires application maintenance")
                }
                Self::Stage => ("stage_stored_child", stored::STAGE),
                Self::Reconcile => ("reconcile_stored_child", stored::RECONCILE),
                Self::StageUpgrade => ("stage_stored_upgrade", stored::STAGE_UPGRADE),
                Self::ReconcileUpgrade => ("reconcile_stored_upgrade", stored::RECONCILE_UPGRADE),
                Self::Observe | Self::ObserveUpgrade => {
                    match plan
                        .management()
                        .and_then(|m| m.source.as_ref())
                        .and_then(|s| s.maintenance.as_ref())
                        .map(|a| a.sequence)
                    {
                        Some(2..) => ("observe_root_history", protocol::custody::ROOT_HISTORY),
                        Some(_) => ("observe_root_custody", protocol::custody::ROOT),
                        None => ("observe_stored_children", stored::CUSTODY),
                    }
                }
            };
        }
        let scoped = plan.management().and_then(|owner| owner.slot).is_some();
        match (self, scoped) {
            (Self::ActivateUpgrade, _) => {
                unreachable!("activation requires application maintenance")
            }
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
        let args = if owner.source.is_some() {
            super::stored::arguments(plan, id)?
        } else {
            match owner.slot {
                Some(slot) if matches!(self, Self::Observe) => ic_wire::encode(
                    &slot,
                    ic_wire::Encoding::Frame {
                        method: id,
                        direction: ic_wire::Direction::Request,
                    },
                    limits,
                ),
                Some(slot) => ic_wire::encode(
                    &children::Request { slot, value },
                    ic_wire::Encoding::Frame {
                        method: id,
                        direction: ic_wire::Direction::Request,
                    },
                    limits,
                ),
                None => ic_wire::encode(
                    value,
                    ic_wire::Encoding::Frame {
                        method: id,
                        direction: ic_wire::Direction::Request,
                    },
                    limits,
                ),
            }
            .map_err(supplier)?
        };
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
pub(crate) fn same_slot(left: Option<ChildId>, right: Option<ChildId>) -> bool {
    left.unwrap_or(ChildId::FIRST) == right.unwrap_or(ChildId::FIRST)
}
