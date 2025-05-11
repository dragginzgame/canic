///
/// endpoints_root
///

#[macro_export]
macro_rules! endpoints_root {
    () => {
        // app
        // modify app-level state
        // @todo eventually this will cascade down from an orchestrator canister
        #[::mimic::ic::update]
        async fn app(cmd: ::actor::state::core::app_state::AppCommand) -> Result<(), ActorError> {
            ::actor::interface::state::core::app_state::command_api(cmd)?;
            ::actor::interface::cascade::app_state_cascade_api().await?;

            Ok(())
        }

        // response
        #[::mimic::ic::update]
        async fn response(
            request: ::actor::interface::request::Request,
        ) -> Result<::actor::interface::response::Response, ActorError> {
            let response = ::actor::interface::response::response(request).await?;

            Ok(response)
        }
    };
}
