use crate::{
    Error, ThisError,
    ids::CanisterRole,
    log::Topic,
    ops::{
        command::{
            CommandOpsError,
            response::{CreateCanisterResponse, CyclesResponse, Response, UpgradeCanisterResponse},
        },
        prelude::*,
        storage::{env::EnvOps, topology::SubnetCanisterChildrenOps},
    },
};
use candid::encode_one;

///
/// RequestOpsError
/// Errors produced during request dispatch or response handling
///

#[derive(Debug, ThisError)]
pub enum RequestOpsError {
    #[error("canister type {0} not found")]
    CanisterRoleNotFound(CanisterRole),

    #[error("child canister {0} not found")]
    ChildNotFound(Principal),

    #[error("canister {0} is not a child of caller {1}")]
    NotChildOfCaller(Principal, Principal),

    #[error("canister {0}'s parent was not found")]
    ParentNotFound(Principal),

    #[error("invalid response type")]
    InvalidResponseType,

    #[error("create_canister: missing new pid")]
    MissingNewCanisterPid,

    #[error("cannot find the root canister")]
    RootNotFound,
}

impl From<RequestOpsError> for Error {
    fn from(err: RequestOpsError) -> Self {
        CommandOpsError::from(err).into()
    }
}

///
/// Request
/// Root-directed orchestration commands.
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub enum Request {
    CreateCanister(CreateCanisterRequest),
    UpgradeCanister(UpgradeCanisterRequest),
    Cycles(CyclesRequest),
}

///
/// CreateCanisterRequest
/// Payload for [`Request::CreateCanister`]
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct CreateCanisterRequest {
    pub canister_role: CanisterRole,
    pub parent: CreateCanisterParent,
    pub extra_arg: Option<Vec<u8>>,
}

///
/// CreateCanisterParent
/// Parent-location choices for a new canister
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub enum CreateCanisterParent {
    Root,
    /// Use the requesting canister as parent.
    ThisCanister,
    /// Use the requesting canister's parent (creates a sibling).
    Parent,
    Canister(Principal),
    Directory(CanisterRole),
}

///
/// UpgradeCanisterRequest
/// Payload for [`Request::UpgradeCanister`]
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct UpgradeCanisterRequest {
    pub canister_pid: Principal,
    pub canister_role: CanisterRole,
}

///
/// CyclesRequest
/// Payload for [`Request::Cycles`]
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct CyclesRequest {
    pub cycles: u128,
}

/// Send a request to the root canister and decode its response.
async fn request(request: Request) -> Result<Response, Error> {
    let root_pid = EnvOps::try_get_root_pid().map_err(|_| RequestOpsError::RootNotFound)?;

    let call_response = Call::unbounded_wait(root_pid, "canic_response")
        .with_arg(&request)
        .await?;

    call_response.candid::<Result<Response, Error>>()?
}

/// Ask root to create and install a canister of the given type.
pub async fn create_canister_request<A>(
    canister_role: &CanisterRole,
    parent: CreateCanisterParent,
    extra: Option<A>,
) -> Result<CreateCanisterResponse, Error>
where
    A: CandidType + Send + Sync,
{
    let encoded = extra.map(|v| encode_one(v)).transpose()?;
    let role = canister_role.clone();
    let parent_desc = format!("{:?}", &parent);
    let caller_ty = EnvOps::try_get_canister_role()
        .map_or_else(|_| "unknown".to_string(), |role| role.to_string());

    // build request
    let q = Request::CreateCanister(CreateCanisterRequest {
        canister_role: canister_role.clone(),
        parent,
        extra_arg: encoded,
    });

    match request(q).await {
        Ok(Response::CreateCanister(res)) => Ok(res),
        Ok(_) => {
            log!(
                Topic::CanisterLifecycle,
                Warn,
                "create_canister_request: invalid response type (caller={caller_ty}, role={role}, parent={parent_desc})"
            );

            Err(RequestOpsError::InvalidResponseType.into())
        }
        Err(err) => {
            log!(
                Topic::CanisterLifecycle,
                Warn,
                "create_canister_request failed (caller={caller_ty}, role={role}, parent={parent_desc}): {err}"
            );

            Err(err)
        }
    }
}

/// Ask root to upgrade a child canister to its latest registered WASM.
pub async fn upgrade_canister_request(
    canister_pid: Principal,
) -> Result<UpgradeCanisterResponse, Error> {
    // check this is a valid child
    let canister = SubnetCanisterChildrenOps::find_by_pid(&canister_pid)
        .ok_or(RequestOpsError::ChildNotFound(canister_pid))?;

    // send the request
    let q = Request::UpgradeCanister(UpgradeCanisterRequest {
        canister_pid: canister.pid,
        canister_role: canister.role,
    });

    match request(q).await? {
        Response::UpgradeCanister(res) => Ok(res),
        _ => Err(RequestOpsError::InvalidResponseType.into()),
    }
}

/// Request a cycle transfer from root to the current canister.
pub async fn cycles_request(cycles: u128) -> Result<CyclesResponse, Error> {
    OpsError::deny_root()?;

    let q = Request::Cycles(CyclesRequest { cycles });

    match request(q).await? {
        Response::Cycles(res) => Ok(res),
        _ => Err(RequestOpsError::InvalidResponseType.into()),
    }
}
