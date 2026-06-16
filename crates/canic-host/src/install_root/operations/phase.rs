pub(in crate::install_root) trait InstallPhaseOperation {
    fn phase(&self) -> &'static str;
    fn attempted_action(&self) -> &'static str;
    fn evidence(&self) -> Vec<String>;
    fn execute(&self) -> Result<(), Box<dyn std::error::Error>>;
}
