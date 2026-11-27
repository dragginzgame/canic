//! The endpoint roster is available to consumers on both targets.

/// Mount explicitly selected management methods using Platform's IC entrypoints.
///
/// Root uses `stage_child_creation` and `reconcile_child_creation` for Store-backed creation,
/// and `stage_child_upgrade` and `reconcile_child_upgrade` for code upgrades.
/// `publish_store_artifact` mounts a separate, controller-only bounded publication call.
/// `observe_store` reads initialized Store state through Root without claiming deployment.
/// Its custody variant reads one release and all pins on its shared artifact.
/// `change_store_admission` forwards an exact Store close or activate command, not an upgrade.
/// `remove_store_artifact` forwards one exact deletion/cancellation; the caller owns settlement custody.
/// `maintain_store` takes an explicit Stage, Advance, Install or Activate command.
/// `store_maintenance_status` reports the same retained metadata as an uncertified public query.
/// `recover_store_maintenance` retains observed outcomes without issuing fresh maintenance effects.
/// `observe_root_custody` reads all service children, artifact sources, lifecycle inputs and Store history.
/// `observe_child_deployments` reads retained deployment evidence on placement Roots.
/// `activate_child_upgrade` activates an approved Blob application upgrade.
/// Pass an approval to `observe_store_maintenance` to read its exact retained outcome.
/// Duplicate selections fail compilation. Unknown names have no expansion.
/// This mounts no lifecycle hooks, storage, timers or readiness policy.
///
/// ```ignore
/// canic_management::mount_endpoints! {
///     stage_child_creation, reconcile_child_creation,
///     stage_child_upgrade, reconcile_child_upgrade,
/// }
/// ```
#[macro_export]
macro_rules! mount_endpoints {
    ($($name:ident),+ $(,)?) => { $($crate::mount_endpoints!(@select $name);)+ };
    (@select maintain_store) => {
        $crate::mount_endpoints!(@method maintain_store, "maintain_store", update, true, store_maintenance::MAINTAIN, STORE_INSTALL_LIMITS,
            [async] (input: $crate::endpoint::protocol::store_maintenance::Request) -> Result<$crate::endpoint::protocol::store_maintenance::Status, $crate::endpoint::protocol::Refusal>,
            $crate::store_maintenance::maintain(input).await);
    };
    (@select observe_store_maintenance) => {
        $crate::mount_endpoints!(@method observe_store_maintenance, "observe_store_maintenance", update, true, store_maintenance::READ_ATTEMPT, LIMITS,
            [] (approval: Option<$crate::endpoint::protocol::store_maintenance::Approval>) -> Option<$crate::endpoint::protocol::store_maintenance::Status>,
            match approval { Some(approval) => $crate::store_maintenance::observe_attempt(&approval), None => $crate::store_maintenance::observe() });
    };
    (@select recover_store_maintenance) => {
        $crate::mount_endpoints!(@method recover_store_maintenance, "recover_store_maintenance", update, true, store_maintenance::RECOVER, LIMITS,
            [async] (input: $crate::endpoint::protocol::store_maintenance::Approval) -> $crate::endpoint::protocol::store_maintenance::Status,
            $crate::store_maintenance::recover(input).await);
    };
    (@select store_maintenance_status) => {
        $crate::mount_endpoints!(@method store_maintenance_status, "store_maintenance_status", query, false, store_maintenance::STATUS, LIMITS,
            [] () -> Option<$crate::endpoint::protocol::store_maintenance::Status>,
            $crate::store_maintenance::observe());
    };
    (@select remove_store_artifact) => {
        $crate::mount_endpoints!(@method remove_store_artifact, "remove_store_artifact", update, true, publication::REMOVE, publication::REMOVAL_LIMITS,
            [async] (input: $crate::endpoint::protocol::publication::RemovalRequest) -> $crate::endpoint::protocol::publication::RemovalReply,
            $crate::publication::remove(input).await);
    };
    (@select change_store_admission) => {
        $crate::mount_endpoints!(@method change_store_admission, "change_store_admission", update, true, publication::ADMISSION, LIMITS,
            [async] (input: $crate::endpoint::protocol::publication::AdmissionRequest) -> $crate::endpoint::protocol::publication::AdmissionReply,
            $crate::publication::admission(input).await);
    };
    (@select observe_store) => {
        $crate::mount_endpoints!(@method observe_store, "observe_store", update, true, publication::OBSERVATION, LIMITS,
            [async] (input: $crate::endpoint::protocol::publication::ObservationRequest) -> $crate::endpoint::protocol::publication::ObservationReply,
            $crate::publication::observe(input).await);
    };
    (@select publish_store_artifact) => {
        $crate::mount_endpoints!(@method publish_store_artifact, "publish_store_artifact", update, true, publication::EXECUTE, PUBLICATION_LIMITS,
            [async] (input: $crate::endpoint::protocol::publication::Request) -> $crate::endpoint::protocol::publication::Reply,
            $crate::publication::execute(input).await);
    };
    (@select activate_child_upgrade) => {
        $crate::mount_endpoints!(@method activate_child_upgrade, "activate_child_upgrade", update, true, application::ACTIVATE, application::LIMITS,
            [async] (input: $crate::endpoint::protocol::children::Request<$crate::endpoint::protocol::application::Upgrade>) -> Result<$crate::endpoint::protocol::application::Maintenance, $crate::endpoint::protocol::Failure>,
            $crate::application::activate(input.slot, input.value).await);
    };
    (@select observe_child_deployments) => {
        $crate::mount_endpoints!(@method observe_child_deployments, "observe_child_deployments", update, true, application::CHILD_EVIDENCE, LIMITS,
            [] () -> $crate::endpoint::protocol::application::Custody,
            $crate::application::child_evidence());
    };
    (@select observe_root_custody) => {
        $crate::mount_endpoints!(@method observe_root_custody, "observe_root_custody", update, true, application::CUSTODY, LIMITS,
            [] () -> $crate::endpoint::protocol::application::Custody,
            $crate::application::custody());
    };
    (@select observe_stored_children) => {
        $crate::mount_endpoints!(@method observe_stored_children, "observe_stored_children", update, true, stored::CUSTODY, LIMITS,
            [] () -> $crate::endpoint::protocol::stored::Custody,
            $crate::stored::custody());
    };
    (@select stage_child_creation) => {
        $crate::mount_endpoints!(@method stage_child_creation, "stage_child_creation", update, true, application::STAGE_CHILD_CREATION, STORED_LIMITS,
            [] (input: $crate::endpoint::protocol::children::Request<$crate::endpoint::protocol::application::Creation>) -> Result<$crate::endpoint::protocol::Status, $crate::endpoint::protocol::Refusal>,
            $crate::application::stage_child_creation(input.slot, input.value));
    };
    (@select reconcile_child_creation) => {
        $crate::mount_endpoints!(@method reconcile_child_creation, "reconcile_child_creation", update, true, application::RECONCILE_CHILD_CREATION, STORED_LIMITS,
            [async] (input: $crate::endpoint::protocol::children::AllocationRequest<$crate::endpoint::protocol::application::Creation>) -> $crate::endpoint::protocol::Status,
            $crate::application::reconcile_child_creation(input.slot, input.value).await);
    };
    (@select stage_child_upgrade) => {
        $crate::mount_endpoints!(@method stage_child_upgrade, "stage_child_upgrade", update, true, application::STAGE_CHILD_UPGRADE, STORED_LIMITS,
            [] (input: $crate::endpoint::protocol::children::Request<$crate::endpoint::protocol::application::Replacement>) -> Result<$crate::endpoint::protocol::upgrade::Status, $crate::endpoint::protocol::Refusal>,
            $crate::application::stage_child_upgrade(input.slot, input.value));
    };
    (@select reconcile_child_upgrade) => {
        $crate::mount_endpoints!(@method reconcile_child_upgrade, "reconcile_child_upgrade", update, true, application::RECONCILE_CHILD_UPGRADE, STORED_LIMITS,
            [async] (input: $crate::endpoint::protocol::children::Request<$crate::endpoint::protocol::application::Replacement>) -> $crate::endpoint::protocol::upgrade::Status,
            $crate::application::reconcile_child_upgrade(input.slot, input.value).await);
    };
    (@select observe_children) => {
        $crate::mount_endpoints!(@method observe_children, "observe_children", update, true, children::CUSTODY, LIMITS,
            [] () -> $crate::endpoint::protocol::children::Custody,
            $crate::children::custody());
    };
    (@select child_status) => {
        $crate::mount_endpoints!(@method child_status, "child_status", query, false, children::READ_STATUS, LIMITS,
            [] (input: $crate::endpoint::protocol::children::StatusRequest) -> $crate::endpoint::protocol::children::ObservedStatus,
            {
                use $crate::endpoint::protocol::children::{StatusRequest, ObservedStatus};
                match input {
                    StatusRequest::Creation(slot) => ObservedStatus::Creation($crate::children::status(slot)),
                    StatusRequest::Upgrade(request) => ObservedStatus::Upgrade(match request.value {
                        Some(approval) => $crate::children::observe_upgrade(request.slot, &approval),
                        None => $crate::children::upgrade_status(request.slot),
                    }),
                }
            });
    };
    (@select observe_child_status) => {
        $crate::mount_endpoints!(@method observe_child_status, "observe_child_status", update, true, children::OBSERVE_STATUS, LIMITS,
            [] (input: $crate::endpoint::protocol::children::ObserveStatus) -> $crate::endpoint::protocol::children::ObservedStatus,
            {
                use $crate::endpoint::protocol::children::{ObserveStatus, ObservedStatus};
                match input {
                    ObserveStatus::Creation(slot) => ObservedStatus::Creation($crate::children::status(slot)),
                    ObserveStatus::Upgrade(request) => ObservedStatus::Upgrade($crate::children::observe_upgrade(request.slot, &request.value)),
                }
            });
    };
    (@select inject_child_fault) => {
        $crate::mount_endpoints!(@method inject_child_fault, "inject_child_fault", update, true, children::INJECT, LIMITS,
            [] (input: $crate::endpoint::protocol::children::AllocationRequest<u8>) -> (),
            $crate::children::inject_fault(input.slot, input.value));
    };
    (@select observe_custody) => {
        $crate::mount_endpoints!(@method observe_custody, "observe_custody", update, true, custody::OBSERVE, LIMITS,
            [] () -> $crate::endpoint::protocol::custody::Custody,
            $crate::custody());
    };
    (@select status) => {
        $crate::mount_endpoints!(@method status, "status", query, false, STATUS, LIMITS,
            [] () -> $crate::endpoint::protocol::Status,
            $crate::status());
    };
    (@select observe) => {
        $crate::mount_endpoints!(@method observe, "observe", update, true, STATUS, LIMITS,
            [] () -> $crate::endpoint::protocol::Status,
            $crate::status());
    };
    (@select upgrade_status) => {
        $crate::mount_endpoints!(@method upgrade_status, "upgrade_status", query, false, upgrade::STATUS, LIMITS,
            [] () -> $crate::endpoint::protocol::upgrade::Status,
            $crate::upgrade_status());
    };
    (@select observe_upgrade) => {
        $crate::mount_endpoints!(@method observe_upgrade, "observe_upgrade", update, true, upgrade::OBSERVE, LIMITS,
            [] (expected: $crate::endpoint::protocol::upgrade::Approval) -> $crate::endpoint::protocol::upgrade::Status,
            $crate::observe_upgrade(&expected));
    };
    (@select inject_fault) => {
        $crate::mount_endpoints!(@method inject_fault, "inject_fault", update, true, INJECT, LIMITS,
            [] (value: u8) -> (),
            $crate::inject_fault(value));
    };
    // One expansion for this actor's fixed endpoint surface, not a public generic
    // endpoint framework. The response alias keeps Platform's owned encoder
    // signature uniform, including the unit fault acknowledgement.
    (@method $name:ident, $export:literal, $kind:ident, $private:literal, $($id:ident)::+, $($limits:ident)::+,
        [$($asynchronous:ident)?] ($($input:ident: $request:ty)?) -> $reply:ty,
        $body:expr) => {
        mod $name {
            use $crate::endpoint::{runtime, protocol, wire};
            type Reply = $reply;
            $crate::endpoint::inspect!(wire, $export, $kind, protocol::$($id)::+, ($($request)?), $reply,
                protocol::$($limits)::+, $crate::endpoint::declared_response_limits(protocol::$($id)::+));

            fn admit() -> Result<(), String> {
                $crate::endpoint::admit(protocol::$($limits)::+.bytes, $private, protocol::$($id)::+)
            }

            fn decode(bytes: &[u8]) -> Result<($($request)?), wire::Error> {
                wire::decode(bytes, wire::Encoding::Frame {
                    method: protocol::$($id)::+, direction: wire::Direction::Request
                }, protocol::$($limits)::+)
            }

            #[allow(clippy::needless_pass_by_value)]
            fn encode(value: Reply) -> Result<Vec<u8>, wire::Error> {
                wire::encode(&value, wire::Encoding::Frame {
                    method: protocol::$($id)::+, direction: wire::Direction::Response
                }, $crate::endpoint::response_limits(protocol::$($id)::+))
            }
            // Platform requires a literal physical ceiling; the guard owns the
            // tighter protocol-specific limit and runs before argument copying.
            const _: () = assert!(protocol::$($limits)::+.bytes <= 2_097_152);

            #[runtime::$kind(
                name = $export,
                runtime = runtime,
                guard = admit,
                decode = decode,
                encode = encode,
                max_bytes = 2097152,
                on_error = $crate::endpoint::fail,
            )]
            $($asynchronous)? fn $name(_input: ($($request)?)) -> Reply {
                $(let $input: $request = _input;)?

                $body
            }
        }
    };
}
