use canic_cli::run_from_env;

// Run the operator CLI and report errors in a shell-friendly form.
fn main() {
    if let Err(err) = run_from_env() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
