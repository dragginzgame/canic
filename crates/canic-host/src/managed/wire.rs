//! One selector for signed child methods and their retained response contracts.
use crate::{Plan, Result, network::principal, signed::Call};
use canic_management_protocol::{
    self as protocol,
    children::{self, ChildId},
};

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
                "observe_child_deployments",
                protocol::application::CHILD_EVIDENCE,
            );
        }
        if plan.management().is_some_and(|m| m.source.is_some()) {
            use protocol::application;
            return match self {
                Self::Stage => ("stage_child_creation", application::STAGE_CHILD_CREATION),
                Self::Reconcile => (
                    "reconcile_child_creation",
                    application::RECONCILE_CHILD_CREATION,
                ),
                Self::StageUpgrade => ("stage_child_upgrade", application::STAGE_CHILD_UPGRADE),
                Self::ReconcileUpgrade => (
                    "reconcile_child_upgrade",
                    application::RECONCILE_CHILD_UPGRADE,
                ),
                Self::Observe | Self::ObserveUpgrade => {
                    ("observe_root_custody", application::CUSTODY)
                }
                Self::ActivateUpgrade => ("activate_application_upgrade", application::ACTIVATE),
            };
        }
        let scoped = plan.management().and_then(|owner| owner.slot).is_some();
        match (self, scoped) {
            (Self::ActivateUpgrade, _) => {
                unreachable!("activation requires application maintenance")
            }
            (Self::Stage, false) => ("stage", protocol::STAGE),
            (Self::Stage, true) => ("stage_inline_child_creation", children::STAGE),
            (Self::Observe, false) => ("observe", protocol::STATUS),
            (Self::Observe, true) => ("observe_child", children::STATUS),
            (Self::Reconcile, false) => ("reconcile", protocol::RECONCILE),
            (Self::Reconcile, true) => ("reconcile_inline_child_creation", children::RECONCILE),
            (Self::StageUpgrade, false) => ("stage_upgrade", protocol::upgrade::STAGE),
            (Self::StageUpgrade, true) => ("stage_inline_child_upgrade", children::STAGE_UPGRADE),
            (Self::ObserveUpgrade, false) => ("observe_upgrade", protocol::upgrade::OBSERVE),
            (Self::ObserveUpgrade, true) => ("observe_child_upgrade", children::OBSERVE_UPGRADE),
            (Self::ReconcileUpgrade, false) => ("reconcile_upgrade", protocol::upgrade::RECONCILE),
            (Self::ReconcileUpgrade, true) => (
                "reconcile_inline_child_upgrade",
                children::RECONCILE_UPGRADE,
            ),
        }
    }

    pub(super) fn call(self, plan: &Plan) -> Result<Call<'static>> {
        let owner = plan
            .management()
            .ok_or_else(|| crate::Error::Invalid("missing child scope".into()))?;
        if owner.source.is_none() {
            return Err(crate::Error::Blocked(
                "inline child approvals require their original Root and Host".into(),
            ));
        }
        let root = principal(&owner.root)?;
        let (method, id) = self.contract(plan);
        let args = super::stored::arguments(plan, id)?;
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
        crate::PlanAction::UpgradeChild { .. } => Method::ObserveUpgrade.call(plan),
        crate::PlanAction::CreateChild(_) => Method::Observe.call(plan),
        _ => Err(crate::Error::Invalid(
            "expected Root-owned child observation".into(),
        )),
    }
}

// None is the historical First slot, but must retain its original wire contract.
pub(crate) fn same_slot(left: Option<ChildId>, right: Option<ChildId>) -> bool {
    left.unwrap_or(ChildId::FIRST) == right.unwrap_or(ChildId::FIRST)
}
