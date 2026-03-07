use super::{
    CreateCanisterParent, CreateCanisterRequest, CreateCanisterResponse, CyclesRequest,
    CyclesResponse, Request, Response, RootRequestMetadata, UpgradeCanisterRequest,
    UpgradeCanisterResponse,
};

impl From<RootRequestMetadata> for crate::dto::rpc::RootRequestMetadata {
    fn from(value: RootRequestMetadata) -> Self {
        Self {
            request_id: value.request_id,
            ttl_seconds: value.ttl_seconds,
        }
    }
}

impl From<crate::dto::rpc::RootRequestMetadata> for RootRequestMetadata {
    fn from(value: crate::dto::rpc::RootRequestMetadata) -> Self {
        Self {
            request_id: value.request_id,
            ttl_seconds: value.ttl_seconds,
        }
    }
}

impl From<CreateCanisterRequest> for crate::dto::rpc::CreateCanisterRequest {
    fn from(value: CreateCanisterRequest) -> Self {
        Self {
            canister_role: value.canister_role,
            parent: value.parent.into(),
            extra_arg: value.extra_arg,
            metadata: value.metadata.map(Into::into),
        }
    }
}

impl From<crate::dto::rpc::CreateCanisterRequest> for CreateCanisterRequest {
    fn from(value: crate::dto::rpc::CreateCanisterRequest) -> Self {
        Self {
            canister_role: value.canister_role,
            parent: value.parent.into(),
            extra_arg: value.extra_arg,
            metadata: value.metadata.map(Into::into),
        }
    }
}

impl From<CreateCanisterParent> for crate::dto::rpc::CreateCanisterParent {
    fn from(value: CreateCanisterParent) -> Self {
        match value {
            CreateCanisterParent::Root => Self::Root,
            CreateCanisterParent::ThisCanister => Self::ThisCanister,
            CreateCanisterParent::Parent => Self::Parent,
            CreateCanisterParent::Canister(pid) => Self::Canister(pid),
            CreateCanisterParent::Directory(role) => Self::Directory(role),
        }
    }
}

impl From<crate::dto::rpc::CreateCanisterParent> for CreateCanisterParent {
    fn from(value: crate::dto::rpc::CreateCanisterParent) -> Self {
        match value {
            crate::dto::rpc::CreateCanisterParent::Root => Self::Root,
            crate::dto::rpc::CreateCanisterParent::ThisCanister => Self::ThisCanister,
            crate::dto::rpc::CreateCanisterParent::Parent => Self::Parent,
            crate::dto::rpc::CreateCanisterParent::Canister(pid) => Self::Canister(pid),
            crate::dto::rpc::CreateCanisterParent::Directory(role) => Self::Directory(role),
        }
    }
}

impl From<UpgradeCanisterRequest> for crate::dto::rpc::UpgradeCanisterRequest {
    fn from(value: UpgradeCanisterRequest) -> Self {
        Self {
            canister_pid: value.canister_pid,
            metadata: value.metadata.map(Into::into),
        }
    }
}

impl From<crate::dto::rpc::UpgradeCanisterRequest> for UpgradeCanisterRequest {
    fn from(value: crate::dto::rpc::UpgradeCanisterRequest) -> Self {
        Self {
            canister_pid: value.canister_pid,
            metadata: value.metadata.map(Into::into),
        }
    }
}

impl From<CyclesRequest> for crate::dto::rpc::CyclesRequest {
    fn from(value: CyclesRequest) -> Self {
        Self {
            cycles: value.cycles,
            metadata: value.metadata.map(Into::into),
        }
    }
}

impl From<crate::dto::rpc::CyclesRequest> for CyclesRequest {
    fn from(value: crate::dto::rpc::CyclesRequest) -> Self {
        Self {
            cycles: value.cycles,
            metadata: value.metadata.map(Into::into),
        }
    }
}

impl From<Request> for crate::dto::rpc::Request {
    fn from(value: Request) -> Self {
        match value {
            Request::CreateCanister(req) => Self::CreateCanister(req.into()),
            Request::UpgradeCanister(req) => Self::UpgradeCanister(req.into()),
            Request::Cycles(req) => Self::Cycles(req.into()),
            Request::IssueDelegation(req) => Self::IssueDelegation(req),
            Request::IssueRoleAttestation(req) => Self::IssueRoleAttestation(req),
        }
    }
}

impl From<crate::dto::rpc::Request> for Request {
    fn from(value: crate::dto::rpc::Request) -> Self {
        match value {
            crate::dto::rpc::Request::CreateCanister(req) => Self::CreateCanister(req.into()),
            crate::dto::rpc::Request::UpgradeCanister(req) => Self::UpgradeCanister(req.into()),
            crate::dto::rpc::Request::Cycles(req) => Self::Cycles(req.into()),
            crate::dto::rpc::Request::IssueDelegation(req) => Self::IssueDelegation(req),
            crate::dto::rpc::Request::IssueRoleAttestation(req) => Self::IssueRoleAttestation(req),
        }
    }
}

impl From<CreateCanisterResponse> for crate::dto::rpc::CreateCanisterResponse {
    fn from(value: CreateCanisterResponse) -> Self {
        Self {
            new_canister_pid: value.new_canister_pid,
        }
    }
}

impl From<crate::dto::rpc::CreateCanisterResponse> for CreateCanisterResponse {
    fn from(value: crate::dto::rpc::CreateCanisterResponse) -> Self {
        Self {
            new_canister_pid: value.new_canister_pid,
        }
    }
}

impl From<UpgradeCanisterResponse> for crate::dto::rpc::UpgradeCanisterResponse {
    fn from(_value: UpgradeCanisterResponse) -> Self {
        Self {}
    }
}

impl From<crate::dto::rpc::UpgradeCanisterResponse> for UpgradeCanisterResponse {
    fn from(_value: crate::dto::rpc::UpgradeCanisterResponse) -> Self {
        Self {}
    }
}

impl From<CyclesResponse> for crate::dto::rpc::CyclesResponse {
    fn from(value: CyclesResponse) -> Self {
        Self {
            cycles_transferred: value.cycles_transferred,
        }
    }
}

impl From<crate::dto::rpc::CyclesResponse> for CyclesResponse {
    fn from(value: crate::dto::rpc::CyclesResponse) -> Self {
        Self {
            cycles_transferred: value.cycles_transferred,
        }
    }
}

impl From<Response> for crate::dto::rpc::Response {
    fn from(value: Response) -> Self {
        match value {
            Response::CreateCanister(res) => Self::CreateCanister(res.into()),
            Response::UpgradeCanister(res) => Self::UpgradeCanister(res.into()),
            Response::Cycles(res) => Self::Cycles(res.into()),
            Response::DelegationIssued(res) => Self::DelegationIssued(res),
            Response::RoleAttestationIssued(res) => Self::RoleAttestationIssued(res),
        }
    }
}

impl From<crate::dto::rpc::Response> for Response {
    fn from(value: crate::dto::rpc::Response) -> Self {
        match value {
            crate::dto::rpc::Response::CreateCanister(res) => Self::CreateCanister(res.into()),
            crate::dto::rpc::Response::UpgradeCanister(res) => Self::UpgradeCanister(res.into()),
            crate::dto::rpc::Response::Cycles(res) => Self::Cycles(res.into()),
            crate::dto::rpc::Response::DelegationIssued(res) => Self::DelegationIssued(res),
            crate::dto::rpc::Response::RoleAttestationIssued(res) => {
                Self::RoleAttestationIssued(res)
            }
        }
    }
}
