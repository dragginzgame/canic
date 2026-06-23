use canic_cli::{cli_error_exit_code, render_cli_error, run_from_env};

// Run the operator CLI and report errors in a shell-friendly form.
fn main() {
    if let Err(err) = run_from_env() {
        eprintln!("{}", render_cli_error(&err));
        std::process::exit(cli_error_exit_code(&err));
    }
}
