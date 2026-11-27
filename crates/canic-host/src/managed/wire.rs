//! One selector for signed child methods and their retained response contracts.
use crate::{Plan, Result, network::principal, signed::Call};
use canic_management_protocol as protocol;

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
        if plan
            .management()
            .is_some_and(|owner| owner.child_evidence_only)
            && matches!(self, Self::Observe | Self::ObserveUpgrade)
        {
            return (
                "observe_child_deployments",
                protocol::application::CHILD_EVIDENCE,
            );
        }
        use protocol::application;
        match self {
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
            Self::Observe | Self::ObserveUpgrade => ("observe_root_custody", application::CUSTODY),
            Self::ActivateUpgrade => ("activate_child_upgrade", application::ACTIVATE),
        }
    }

    pub(super) fn call(self, plan: &Plan) -> Result<Call<'static>> {
        let owner = plan
            .management()
            .ok_or_else(|| crate::Error::Invalid("missing child scope".into()))?;
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
