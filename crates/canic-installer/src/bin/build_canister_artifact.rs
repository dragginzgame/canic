use canic_installer::canister_build::{
    CanisterBuildProfile, build_current_workspace_canister_artifact,
};
use canic_installer::release_set::{dfx_root, workspace_root};
use std::{env, fs, time::Instant};

// Run the public visible-canister build entrypoint and print the `.wasm.gz` path.
fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

// Build one visible Canic canister artifact for the current workspace.
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let canister_name = std::env::args()
        .nth(1)
        .ok_or_else(|| "usage: canic-build-canister-artifact <canister_name>".to_string())?;
    let profile = CanisterBuildProfile::current();
    print_build_context_once(profile)?;
    eprintln!(
        "Canic build start: canister={canister_name} profile={}",
        profile.target_dir_name()
    );

    let started_at = Instant::now();
    let output = build_current_workspace_canister_artifact(&canister_name, profile)?;
    let elapsed = started_at.elapsed().as_secs_f64();

    println!("{}", output.wasm_gz_path.display());
    eprintln!("Canic build done: canister={canister_name} elapsed={elapsed:.2}s");
    eprintln!();
    Ok(())
}

fn print_build_context_once(
    profile: CanisterBuildProfile,
) -> Result<(), Box<dyn std::error::Error>> {
    let workspace_root = workspace_root()?;
    let dfx_root = dfx_root()?;
    let marker_dir = dfx_root.join(".dfx");
    fs::create_dir_all(&marker_dir)?;

    let requested_profile = env::var("CANIC_WASM_PROFILE").unwrap_or_else(|_| "unset".to_string());
    let network = env::var("DFX_NETWORK").unwrap_or_else(|_| "local".to_string());
    let marker_file = marker_dir.join(format!(
        ".canic-build-context-{}",
        dfx_ancestor_process_id()
            .or_else(parent_process_id)
            .unwrap_or_else(std::process::id)
    ));

    if !marker_file.exists() {
        fs::write(&marker_file, [])?;
        eprintln!(
            "Canic build context: profile={} requested_profile={} DFX_NETWORK={} CANIC_WORKSPACE_ROOT={} CANIC_DFX_ROOT={}",
            profile.target_dir_name(),
            requested_profile,
            network,
            workspace_root.display(),
            dfx_root.display()
        );
    }

    Ok(())
}

fn parent_process_id() -> Option<u32> {
    let stat = fs::read_to_string("/proc/self/stat").ok()?;
    parse_parent_process_id(&stat)
}

fn dfx_ancestor_process_id() -> Option<u32> {
    let mut pid = parent_process_id()?;
    loop {
        if process_comm(pid).as_deref() == Some("dfx") {
            return Some(pid);
        }

        let parent = process_parent_id(pid)?;
        if parent == 0 || parent == pid {
            return None;
        }
        pid = parent;
    }
}

fn process_parent_id(pid: u32) -> Option<u32> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    parse_parent_process_id(&stat)
}

fn process_comm(pid: u32) -> Option<String> {
    fs::read_to_string(format!("/proc/{pid}/comm"))
        .ok()
        .map(|comm| comm.trim().to_string())
}

fn parse_parent_process_id(stat: &str) -> Option<u32> {
    let (_, suffix) = stat.rsplit_once(") ")?;
    let mut parts = suffix.split_whitespace();
    let _state = parts.next()?;
    parts.next()?.parse::<u32>().ok()
}

#[cfg(test)]
mod tests {
    use super::parse_parent_process_id;

    #[test]
    fn parse_parent_process_id_accepts_proc_stat_shape() {
        let stat = "12345 (build_canister_ar) S 67890 0 0 0";
        assert_eq!(parse_parent_process_id(stat), Some(67890));
    }
}
