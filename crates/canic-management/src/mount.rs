//! The endpoint roster is available to consumers on both targets.

/// Mount explicitly selected management methods using Platform's IC entrypoints.
///
/// Names are the existing exports: `status`, `observe`, `observe_custody`, `stage`, `reconcile`,
/// `stage_upgrade`, `upgrade_status`, `observe_upgrade`, `reconcile_upgrade`,
/// and (only with the supplier's `fault-injection` feature) `inject_fault`.
/// Child-scoped selections: `child_status`, `child_upgrade_status`, `observe_child`, `observe_children`,
/// `stage_child`, `reconcile_child`, `stage_child_upgrade`, `observe_child_upgrade`,
/// `reconcile_child_upgrade` and test-only `inject_child_fault`.
/// Store-bound selections: `observe_stored_children`, `stage_stored_child`,
/// `reconcile_stored_child`, `stage_stored_upgrade` and `reconcile_stored_upgrade`.
/// `store_publication` mounts a separate, controller-only bounded publication call.
/// `observe_store` reads initialized Store state through Root without claiming deployment.
/// `observe_store_custody` reads one release and all pins on its shared artifact.
/// `store_admission` forwards an exact Store close/observe/activate command, not an upgrade.
/// `store_removal` forwards one exact deletion/cancellation; the caller owns settlement custody.
/// `stage_store_maintenance`, `advance_store_maintenance` and `observe_store_maintenance`
/// retain and recover Root-owned closure/stopping.
/// `install_store_maintenance` supplies approved replacement bytes; `activate_store_maintenance`
/// explicitly opens admission only after replacement verification.
/// `store_maintenance_status` reports the same retained metadata as an uncertified public query.
/// `recover_store_maintenance` retains observed outcomes without issuing fresh maintenance effects.
/// `observe_root_custody` reads children, artifact references and a single Store operation.
/// `observe_root_history` includes all completed Store predecessors; older reads refuse omission.
/// `observe_store_attempt` reads an exact current or historical approval without effects.
/// Duplicate selections fail compilation. Unknown names have no expansion.
/// This mounts no lifecycle hooks, storage, timers or readiness policy.
///
/// ```ignore
/// canic_management::mount_endpoints! {
///     status, observe, observe_custody, stage, reconcile,
///     stage_upgrade, upgrade_status, observe_upgrade, reconcile_upgrade,
/// }
/// ```
#[macro_export]
macro_rules! mount_endpoints {
    (@select observe_root_history) => {
        $crate::mount_endpoints!(@method observe_root_history, "observe_root_history", update, true, custody::ROOT_HISTORY, LIMITS,
            [] () -> $crate::endpoint::protocol::custody::RootHistory,
            $crate::stored::root_history());
    };
    (@select observe_store_attempt) => {
        $crate::mount_endpoints!(@method observe_store_attempt, "observe_store_attempt", update, true, store_maintenance::OBSERVE_ATTEMPT, LIMITS,
            [] (input: $crate::endpoint::protocol::store_maintenance::Approval) -> Option<$crate::endpoint::protocol::store_maintenance::Status>,
            $crate::store_maintenance::observe_attempt(&input));
    };
    (@select observe_root_custody) => {
        $crate::mount_endpoints!(@method observe_root_custody, "observe_root_custody", update, true, custody::ROOT, LIMITS,
            [] () -> $crate::endpoint::protocol::custody::Root,
            $crate::stored::root_custody());
    };
    ($($name:ident),+ $(,)?) => { $($crate::mount_endpoints!(@select $name);)+ };
    (@select install_store_maintenance) => {
        $crate::mount_endpoints!(@method install_store_maintenance, "install_store_maintenance", update, true, store_maintenance::INSTALL, STORE_INSTALL_LIMITS,
            [async] (input: $crate::endpoint::protocol::store_maintenance::Install) -> Result<$crate::endpoint::protocol::store_maintenance::Status, $crate::endpoint::protocol::Refusal>,
            $crate::store_maintenance::install(input).await);
    };
    (@select activate_store_maintenance) => {
        $crate::mount_endpoints!(@method activate_store_maintenance, "activate_store_maintenance", update, true, store_maintenance::ACTIVATE, LIMITS,
            [async] (input: $crate::endpoint::protocol::store_maintenance::Approval) -> $crate::endpoint::protocol::store_maintenance::Status,
            $crate::store_maintenance::activate(input).await);
    };
    (@select stage_store_maintenance) => {
        $crate::mount_endpoints!(@method stage_store_maintenance, "stage_store_maintenance", update, true, store_maintenance::STAGE, LIMITS,
            [] (input: $crate::endpoint::protocol::store_maintenance::Approval) -> Result<$crate::endpoint::protocol::store_maintenance::Status, $crate::endpoint::protocol::Refusal>,
            $crate::store_maintenance::stage(input));
    };
    (@select advance_store_maintenance) => {
        $crate::mount_endpoints!(@method advance_store_maintenance, "advance_store_maintenance", update, true, store_maintenance::ADVANCE, LIMITS,
            [async] (input: $crate::endpoint::protocol::store_maintenance::Approval) -> $crate::endpoint::protocol::store_maintenance::Status,
            $crate::store_maintenance::advance(input).await);
    };
    (@select observe_store_maintenance) => {
        $crate::mount_endpoints!(@method observe_store_maintenance, "observe_store_maintenance", update, true, store_maintenance::OBSERVE, LIMITS,
            [] () -> Option<$crate::endpoint::protocol::store_maintenance::Status>,
            $crate::store_maintenance::observe());
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
    (@select store_removal) => {
        $crate::mount_endpoints!(@method store_removal, "store_removal", update, true, publication::REMOVE, publication::REMOVAL_LIMITS,
            [async] (input: $crate::endpoint::protocol::publication::RemovalRequest) -> $crate::endpoint::protocol::publication::RemovalReply,
            $crate::publication::remove(input).await);
    };
    (@select store_admission) => {
        $crate::mount_endpoints!(@method store_admission, "store_admission", update, true, publication::ADMISSION, LIMITS,
            [async] (input: $crate::endpoint::protocol::publication::AdmissionRequest) -> $crate::endpoint::protocol::publication::AdmissionReply,
            $crate::publication::admission(input).await);
    };
    (@select observe_store_custody) => {
        $crate::mount_endpoints!(@method observe_store_custody, "observe_store_custody", update, true, publication::CUSTODY, LIMITS,
            [async] (input: $crate::endpoint::protocol::publication::CustodyRequest) -> $crate::endpoint::protocol::publication::CustodyReply,
            $crate::publication::observe_custody(input).await);
    };
    (@select observe_store) => {
        $crate::mount_endpoints!(@method observe_store, "observe_store", update, true, publication::INFO, LIMITS,
            [async] (input: $crate::endpoint::protocol::publication::InfoRequest) -> $crate::endpoint::protocol::publication::InfoReply,
            $crate::publication::observe_info(input).await);
    };
    (@select store_publication) => {
        $crate::mount_endpoints!(@method store_publication, "store_publication", update, true, publication::EXECUTE, PUBLICATION_LIMITS,
            [async] (input: $crate::endpoint::protocol::publication::Request) -> $crate::endpoint::protocol::publication::Reply,
            $crate::publication::execute(input).await);
    };
    (@select stage_application_upgrade) => {
        $crate::mount_endpoints!(@method stage_application_upgrade, "stage_application_upgrade", update, true, application::STAGE_UPGRADE, application::LIMITS,
            [] (input: $crate::endpoint::protocol::children::Request<$crate::endpoint::protocol::application::Upgrade>) -> Result<$crate::endpoint::protocol::upgrade::Status, $crate::endpoint::protocol::Refusal>,
            $crate::application::stage_upgrade(input.slot, input.value));
    };
    (@select reconcile_application_upgrade) => {
        $crate::mount_endpoints!(@method reconcile_application_upgrade, "reconcile_application_upgrade", update, true, application::RECONCILE_UPGRADE, application::LIMITS,
            [async] (input: $crate::endpoint::protocol::children::Request<$crate::endpoint::protocol::application::Upgrade>) -> $crate::endpoint::protocol::upgrade::Status,
            $crate::application::reconcile_upgrade(input.slot, input.value).await);
    };
    (@select activate_application_upgrade) => {
        $crate::mount_endpoints!(@method activate_application_upgrade, "activate_application_upgrade", update, true, application::ACTIVATE, application::LIMITS,
            [async] (input: $crate::endpoint::protocol::children::Request<$crate::endpoint::protocol::application::Upgrade>) -> Result<$crate::endpoint::protocol::application::Maintenance, $crate::endpoint::protocol::Failure>,
            $crate::application::activate(input.slot, input.value).await);
    };
    (@select observe_application_children) => {
        $crate::mount_endpoints!(@method observe_application_children, "observe_application_children", update, true, application::CUSTODY, LIMITS,
            [] () -> $crate::endpoint::protocol::application::Custody,
            $crate::application::custody());
    };
    (@select stage_application_child) => {
        $crate::mount_endpoints!(@method stage_application_child, "stage_application_child", update, true, application::STAGE, application::LIMITS,
            [] (input: $crate::endpoint::protocol::children::Request<$crate::endpoint::protocol::application::Stage<$crate::endpoint::protocol::Approval>>) -> Result<$crate::endpoint::protocol::Status, $crate::endpoint::protocol::Refusal>,
            $crate::application::stage(input.slot, input.value));
    };
    (@select reconcile_application_child) => {
        $crate::mount_endpoints!(@method reconcile_application_child, "reconcile_application_child", update, true, application::RECONCILE, application::LIMITS,
            [async] (input: $crate::endpoint::protocol::children::Request<$crate::endpoint::protocol::application::Stage<$crate::endpoint::protocol::Approval>>) -> $crate::endpoint::protocol::Status,
            $crate::application::reconcile(input.slot, input.value).await);
    };
    (@select observe_stored_children) => {
        $crate::mount_endpoints!(@method observe_stored_children, "observe_stored_children", update, true, stored::CUSTODY, LIMITS,
            [] () -> $crate::endpoint::protocol::stored::Custody,
            $crate::stored::custody());
    };
    (@select stage_stored_child) => {
        $crate::mount_endpoints!(@method stage_stored_child, "stage_stored_child", update, true, stored::STAGE, STORED_LIMITS,
            [] (input: $crate::endpoint::protocol::children::Request<$crate::endpoint::protocol::stored::Stage<$crate::endpoint::protocol::Approval>>) -> Result<$crate::endpoint::protocol::Status, $crate::endpoint::protocol::Refusal>,
            $crate::stored::stage(input.slot, input.value));
    };
    (@select reconcile_stored_child) => {
        $crate::mount_endpoints!(@method reconcile_stored_child, "reconcile_stored_child", update, true, stored::RECONCILE, STORED_LIMITS,
            [async] (input: $crate::endpoint::protocol::children::Request<$crate::endpoint::protocol::stored::Stage<$crate::endpoint::protocol::Approval>>) -> $crate::endpoint::protocol::Status,
            $crate::stored::reconcile(input.slot, input.value).await);
    };
    (@select stage_stored_upgrade) => {
        $crate::mount_endpoints!(@method stage_stored_upgrade, "stage_stored_upgrade", update, true, stored::STAGE_UPGRADE, STORED_LIMITS,
            [] (input: $crate::endpoint::protocol::children::Request<$crate::endpoint::protocol::stored::Stage<$crate::endpoint::protocol::upgrade::Approval>>) -> Result<$crate::endpoint::protocol::upgrade::Status, $crate::endpoint::protocol::Refusal>,
            $crate::stored::stage_upgrade(input.slot, input.value));
    };
    (@select reconcile_stored_upgrade) => {
        $crate::mount_endpoints!(@method reconcile_stored_upgrade, "reconcile_stored_upgrade", update, true, stored::RECONCILE_UPGRADE, STORED_LIMITS,
            [async] (input: $crate::endpoint::protocol::children::Request<$crate::endpoint::protocol::stored::Stage<$crate::endpoint::protocol::upgrade::Approval>>) -> $crate::endpoint::protocol::upgrade::Status,
            $crate::stored::reconcile_upgrade(input.slot, input.value).await);
    };
    (@select observe_children) => {
        $crate::mount_endpoints!(@method observe_children, "observe_children", update, true, children::CUSTODY, LIMITS,
            [] () -> $crate::endpoint::protocol::children::Custody,
            $crate::children::custody());
    };
    (@select child_status) => {
        $crate::mount_endpoints!(@method child_status, "child_status", query, false, children::STATUS, LIMITS,
            [] (input: $crate::endpoint::protocol::children::ChildId) -> $crate::endpoint::protocol::Status,
            $crate::children::status(input));
    };
    (@select observe_child) => {
        $crate::mount_endpoints!(@method observe_child, "observe_child", update, true, children::STATUS, LIMITS,
            [] (input: $crate::endpoint::protocol::children::ChildId) -> $crate::endpoint::protocol::Status,
            $crate::children::status(input));
    };
    (@select stage_child) => {
        $crate::mount_endpoints!(@method stage_child, "stage_child", update, true, children::STAGE, STAGE_LIMITS,
            [] (input: $crate::endpoint::protocol::children::Request<$crate::endpoint::protocol::Stage>) -> Result<$crate::endpoint::protocol::Status, $crate::endpoint::protocol::Refusal>,
            $crate::children::stage(input.slot, input.value));
    };
    (@select reconcile_child) => {
        $crate::mount_endpoints!(@method reconcile_child, "reconcile_child", update, true, children::RECONCILE, LIMITS,
            [async] (input: $crate::endpoint::protocol::children::Request<$crate::endpoint::protocol::Approval>) -> $crate::endpoint::protocol::Status,
            $crate::children::reconcile(input.slot, input.value).await);
    };
    (@select stage_child_upgrade) => {
        $crate::mount_endpoints!(@method stage_child_upgrade, "stage_child_upgrade", update, true, children::STAGE_UPGRADE, STAGE_LIMITS,
            [] (input: $crate::endpoint::protocol::children::Request<$crate::endpoint::protocol::upgrade::Stage>) -> Result<$crate::endpoint::protocol::upgrade::Status, $crate::endpoint::protocol::Refusal>,
            $crate::children::stage_upgrade(input.slot, input.value));
    };
    (@select observe_child_upgrade) => {
        $crate::mount_endpoints!(@method observe_child_upgrade, "observe_child_upgrade", update, true, children::OBSERVE_UPGRADE, LIMITS,
            [] (input: $crate::endpoint::protocol::children::Request<$crate::endpoint::protocol::upgrade::Approval>) -> $crate::endpoint::protocol::upgrade::Status,
            $crate::children::observe_upgrade(input.slot, &input.value));
    };
    (@select child_upgrade_status) => {
        $crate::mount_endpoints!(@method child_upgrade_status, "child_upgrade_status", query, false, children::OBSERVE_UPGRADE, LIMITS,
            [] (input: $crate::endpoint::protocol::children::Request<$crate::endpoint::protocol::upgrade::Approval>) -> $crate::endpoint::protocol::upgrade::Status,
            $crate::children::observe_upgrade(input.slot, &input.value));
    };
    (@select reconcile_child_upgrade) => {
        $crate::mount_endpoints!(@method reconcile_child_upgrade, "reconcile_child_upgrade", update, true, children::RECONCILE_UPGRADE, LIMITS,
            [async] (input: $crate::endpoint::protocol::children::Request<$crate::endpoint::protocol::upgrade::Approval>) -> $crate::endpoint::protocol::upgrade::Status,
            $crate::children::reconcile_upgrade(input.slot, input.value).await);
    };
    (@select inject_child_fault) => {
        $crate::mount_endpoints!(@method inject_child_fault, "inject_child_fault", update, true, children::INJECT, LIMITS,
            [] (input: $crate::endpoint::protocol::children::Request<u8>) -> (),
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
    (@select stage) => {
        $crate::mount_endpoints!(@method stage, "stage", update, true, STAGE, STAGE_LIMITS,
            [] (input: $crate::endpoint::protocol::Stage)
                -> Result<$crate::endpoint::protocol::Status, $crate::endpoint::protocol::Refusal>,
            $crate::stage(input));
    };
    (@select reconcile) => {
        $crate::mount_endpoints!(@method reconcile, "reconcile", update, true, RECONCILE, LIMITS,
            [async] (expected: $crate::endpoint::protocol::Approval) -> $crate::endpoint::protocol::Status,
            $crate::reconcile(expected).await);
    };
    (@select stage_upgrade) => {
        $crate::mount_endpoints!(@method stage_upgrade, "stage_upgrade", update, true, upgrade::STAGE, STAGE_LIMITS,
            [] (input: $crate::endpoint::protocol::upgrade::Stage)
                -> Result<$crate::endpoint::protocol::upgrade::Status, $crate::endpoint::protocol::Refusal>,
            $crate::stage_upgrade(input));
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
    (@select reconcile_upgrade) => {
        $crate::mount_endpoints!(@method reconcile_upgrade, "reconcile_upgrade", update, true, upgrade::RECONCILE, LIMITS,
            [async] (expected: $crate::endpoint::protocol::upgrade::Approval) -> $crate::endpoint::protocol::upgrade::Status,
            $crate::reconcile_upgrade(expected).await);
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
            #[cfg(target_arch = "wasm32")]
            mod installed {
                use $crate::endpoint::{runtime, protocol, wire};
                type Reply = $reply;

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
                    runtime = runtime,
                    name = $export,
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
        }
    };
}
