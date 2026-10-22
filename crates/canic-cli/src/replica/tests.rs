use super::*;

#[cfg(unix)]
use crate::test_support::TempDir;
#[cfg(unix)]
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    net::TcpListener,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

// Ensure replica start defaults to foreground mode while allowing background use.
#[test]
fn parses_replica_start_options() {
    let options = ReplicaOptions::parse_start([
        OsString::from("--background"),
        OsString::from(crate::cli::globals::INTERNAL_ICP_OPTION),
        OsString::from("/tmp/icp"),
    ])
    .expect("parse replica start");

    assert_eq!(options.icp, "/tmp/icp");
    assert_eq!(options.port, None);
    assert!(options.background);
    assert!(!options.debug);
    assert!(!options.json);
}

// Ensure replica start can require the project-local gateway port before launch.
#[test]
fn parses_replica_start_port() {
    let options = ReplicaOptions::parse_start([OsString::from("--port"), OsString::from("8001")])
        .expect("parse replica start");

    assert_eq!(options.port, Some(8001));
}

// Ensure invalid ports fail at CLI parsing before running ICP commands.
#[test]
fn rejects_invalid_replica_start_port() {
    let error = ReplicaOptions::parse_start([OsString::from("--port"), OsString::from("0")])
        .expect_err("port 0 should be invalid");

    std::assert_matches!(error, ReplicaCommandError::Usage(_));
}

// Ensure foreground mode is the default, matching ICP CLI.
#[test]
fn replica_start_defaults_to_foreground() {
    let options = ReplicaOptions::parse_start([]).expect("parse replica start");

    assert_eq!(options.icp, "icp");
    assert_eq!(options.port, None);
    assert!(!options.background);
    assert!(!options.debug);
    assert!(!options.json);
}

// Ensure replica lifecycle commands can enable ICP CLI debug logging.
#[test]
fn parses_replica_debug_options() {
    let start =
        ReplicaOptions::parse_start([OsString::from("--debug")]).expect("parse replica start");
    let status =
        ReplicaOptions::parse_status([OsString::from("--debug")]).expect("parse replica status");
    let stop = ReplicaOptions::parse_stop([OsString::from("--debug")]).expect("parse replica stop");

    assert!(start.debug);
    assert!(status.debug);
    assert!(stop.debug);
    assert!(!status.json);
}

// Ensure status uses the default ICP executable when no override is provided.
#[test]
fn parses_replica_status_options() {
    let options = ReplicaOptions::parse_status([]).expect("parse replica status");

    assert_eq!(options.icp, "icp");
    assert_eq!(options.port, None);
    assert!(!options.background);
    assert!(!options.debug);
    assert!(!options.json);
}

// Ensure replica status can expose the underlying ICP status payload as JSON.
#[test]
fn parses_replica_status_json_option() {
    let options =
        ReplicaOptions::parse_status([OsString::from("--json")]).expect("parse replica status");

    assert!(options.json);
}

// Ensure JSON can distinguish ICP CLI-managed status from direct HTTP fallback.
#[test]
fn replica_status_json_exposes_status_source() {
    let report = ReplicaStatusJsonReport {
        network: "local",
        running: true,
        configured_gateway_port: "8000".to_string(),
        status_source: ReplicaStatusSource::HttpStatus,
        icp_cli_running: false,
        local_gateway_reachable: true,
        icp_status: None,
    };

    let value = serde_json::to_value(report).expect("serialize replica status");

    assert_eq!(value["running"], true);
    assert_eq!(value["status_source"], "http_status");
    assert_eq!(value["icp_cli_running"], false);
    assert_eq!(value["local_gateway_reachable"], true);
    assert!(value["icp_status"].is_null());
}

// Ensure JSON can surface stale ICP status metadata without claiming the replica is usable.
#[test]
fn replica_status_json_exposes_stale_icp_status() {
    let report = ReplicaStatusJsonReport {
        network: "local",
        running: false,
        configured_gateway_port: "8001".to_string(),
        status_source: ReplicaStatusSource::IcpCliStale,
        icp_cli_running: true,
        local_gateway_reachable: false,
        icp_status: Some(serde_json::json!({
            "gateway_url": "http://localhost:8001/"
        })),
    };

    let value = serde_json::to_value(report).expect("serialize replica status");

    assert_eq!(value["running"], false);
    assert_eq!(value["status_source"], "icp_cli_stale");
    assert_eq!(value["icp_cli_running"], true);
    assert_eq!(value["local_gateway_reachable"], false);
    assert_eq!(value["icp_status"]["gateway_url"], "http://localhost:8001/");
}

// Ensure replica help exposes the native lifecycle commands.
#[test]
fn replica_usage_lists_commands() {
    let text = usage();

    assert!(text.contains("Manage the local ICP replica"));
    assert!(text.contains("start"));
    assert!(text.contains("status"));
    assert!(text.contains("stop"));
    assert!(text.contains("canic replica status"));
}

// Ensure leaf help documents command-specific options and examples.
#[test]
fn replica_leaf_usage_lists_options() {
    let text = start_usage();

    assert!(text.contains("--background"));
    assert!(text.contains("--port"));
    assert!(text.contains("--debug"));
    assert!(!text.contains("--json"));
    assert!(!text.contains("--icp"));
    assert!(text.contains("canic replica start --background"));
    assert!(text.contains("canic replica start --port 8001 --background"));

    let status = status_usage();
    assert!(status.contains("--json"));
}

// Ensure ICP's foreign-owner diagnostic is surfaced as an ownership problem.
#[test]
fn maps_foreign_local_replica_owner_error() {
    let error = replica_icp_error(IcpCommandError::Failed {
        command: "icp network start local --background".to_string(),
        stderr: "Error: port 8000 is in use by the local network of the project at '/workspaces/other-app'\n".to_string(),
    });

    std::assert_matches!(
        error,
        ReplicaCommandError::ForeignLocalReplicaOwner { ref network, ref project }
            if network == "local" && project == "/workspaces/other-app"
    );
    assert!(
        error
            .to_string()
            .contains("owned by ICP network `local` for project: /workspaces/other-app")
    );
}

// Ensure ICP's raw missing-project error is replaced with a Canic setup hint.
#[test]
fn maps_missing_project_manifest_error() {
    let error = replica_icp_error(IcpCommandError::Failed {
        command: "icp network start local".to_string(),
        stderr: "Error: failed to locate project directory\n\nCaused by:\n    project manifest not found in /home/adam/projects/toko/backend\n".to_string(),
    });

    std::assert_matches!(error, ReplicaCommandError::ProjectManifestMissing);
    assert!(error.to_string().contains("Create or repair icp.yaml"));
    assert!(error.to_string().contains("retry the replica command"));
}

// Ensure owner parsing keeps the ICP network/environment separate from the project path.
#[test]
fn parses_foreign_local_replica_owner() {
    let error = IcpCommandError::Failed {
        command: "icp network start local".to_string(),
        stderr: "Error: port 8000 is in use by the demo network of the project at '/home/adam/projects/toko'\n".to_string(),
    };
    let Some(IcpDiagnostic::ForeignLocalReplicaOwner { network, project }) = error.diagnostic()
    else {
        panic!("expected foreign replica owner diagnostic");
    };

    assert_eq!(
        (network, project),
        ("demo".to_string(), "/home/adam/projects/toko".to_string())
    );
}

// Ensure stop can distinguish ICP's project-scoped not-running diagnostic.
#[test]
fn detects_project_local_network_not_running() {
    let error = IcpCommandError::Failed {
        command: "icp network stop local".to_string(),
        stderr: "Error: network 'local' is not running\n".to_string(),
    };

    assert!(local_network_not_running(&error));

    let status_error = IcpCommandError::Failed {
        command: "icp network status local".to_string(),
        stderr: "Error: unable to access network 'local', is it running?\n\nCaused by:\n    the local network for this project is not running\n".to_string(),
    };

    assert!(local_network_not_running(&status_error));
}

#[cfg(unix)]
#[test]
fn stop_checks_the_selected_gateway_when_the_network_is_absent() {
    for reachable in [true, false] {
        let root = TempDir::new("canic-replica-stop-gateway");
        fs::create_dir_all(&root).unwrap();
        let selected_root = fs::canonicalize(&root).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let port = listener.local_addr().unwrap().port();
        fs::write(
            root.join("icp.yaml"),
            format!(
                "networks:\n  - name: local\n    mode: managed\n    gateway:\n      port: {port}\n"
            ),
        )
        .unwrap();
        let server = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(5);
            let stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(
                            Instant::now() < deadline,
                            "selected gateway was not queried"
                        );
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => panic!("accept selected gateway request: {error}"),
                }
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut request = BufReader::new(stream);
            let mut first_line = String::new();
            request.read_line(&mut first_line).unwrap();
            loop {
                let mut line = String::new();
                assert!(request.read_line(&mut line).unwrap() > 0);
                if line == "\r\n" {
                    break;
                }
            }
            let status = if reachable {
                "200 OK"
            } else {
                "503 Service Unavailable"
            };
            write!(
                request.get_mut(),
                "HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            )
            .unwrap();
            first_line
        });
        let executable = write_stop_icp(&root, 1, "Error: network 'local' is not running");
        let icp = IcpCli::new(executable.to_str().unwrap(), None);
        let result = stop_local_replica(&icp, &selected_root, true);
        assert_eq!(server.join().unwrap(), "GET /api/v2/status HTTP/1.1\r\n");
        if reachable {
            std::assert_matches!(
                result,
                Err(ReplicaCommandError::ForeignLocalReplicaReachable)
            );
        } else {
            result.unwrap();
        }
        assert_eq!(
            fs::read_to_string(root.join("calls")).unwrap(),
            "network\nstop\nlocal\n--debug\n"
        );
        assert_eq!(
            fs::read_to_string(root.join("selected-roots"))
                .unwrap()
                .trim(),
            selected_root.to_str().unwrap()
        );
        assert_eq!(
            fs::read_to_string(root.join("cwd")).unwrap().trim(),
            selected_root.to_str().unwrap()
        );
    }
}

#[cfg(unix)]
#[test]
fn stop_preserves_success_and_typed_command_failures() {
    for (case, (status, diagnostic)) in [
        (0, "Network stopped successfully"),
        (1, "Error: access denied"),
        (
            1,
            "Error: port 8000 is in use by the demo network of the project at '/other-app'",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let root = TempDir::new("canic-replica-stop-result");
        fs::create_dir_all(&root).unwrap();
        let executable = write_stop_icp(&root, status, diagnostic);
        let icp = IcpCli::new(executable.to_str().unwrap(), None);
        let result = stop_local_replica(&icp, &root, false);
        match case {
            0 => result.unwrap(),
            1 => std::assert_matches!(
                result,
                Err(ReplicaCommandError::Icp(IcpCommandError::Failed { .. }))
            ),
            _ => {
                std::assert_matches!(result, Err(ReplicaCommandError::ForeignLocalReplicaOwner { network, project }) if network == "demo" && project == "/other-app");
            }
        }
        assert_eq!(
            fs::read_to_string(root.join("calls")).unwrap(),
            "network\nstop\nlocal\n"
        );
    }
}

#[cfg(unix)]
fn write_stop_icp(root: &Path, status: u8, diagnostic: &str) -> PathBuf {
    let executable = root.join("icp-stop-fixture");
    fs::write(root.join("exit-status"), status.to_string()).unwrap();
    fs::write(root.join("diagnostic"), diagnostic).unwrap();
    fs::write(
        &executable,
        r#"#!/bin/sh
if [ "$1" = --version ]; then
    printf 'icp 1.5.0\n'
    exit 0
fi
pwd -P > cwd
while [ "$#" -gt 0 ]; do
    case "$1" in
        --project-root-override)
            shift
            printf '%s\n' "$1" >> selected-roots
            shift
            ;;
        --identity-password-file) shift 2 ;;
        *) printf '%s\n' "$1" >> calls; shift ;;
    esac
done
cat diagnostic >&2
exit "$(cat exit-status)"
"#,
    )
    .unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
    executable
}
