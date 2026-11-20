//! Native CLI refusal against an independent HTTP peer; no fixture or credentials.

use std::{
    io::{Read, Write},
    net::TcpListener,
    process::{Command, Output, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Copy)]
enum Reply {
    Busy(u16),
    Stall,
    Redirect,
    Oversized,
}

fn refuse(reply: Reply) -> (Output, Vec<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let (stop, stopped) = mpsc::channel();
    let peer = thread::spawn(move || {
        let mut requests = Vec::new();
        loop {
            if stopped.try_recv().is_ok() {
                break;
            }
            let (mut socket, _) = match listener.accept() {
                Ok(socket) => socket,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5));
                    continue;
                }
                Err(e) => panic!("accept failed: {e}"),
            };
            socket
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            socket
                .set_write_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut bytes = [0; 8192];
            let mut read = 0;
            // TCP may split the request line across reads. Keep capture bounded.
            while !bytes[..read].contains(&b'\n') {
                let count = socket.read(&mut bytes[read..]).unwrap();
                assert!(count > 0, "missing or oversized HTTP request line");
                read += count;
            }
            assert!(bytes[..read].starts_with(b"POST "));
            requests.push(
                String::from_utf8_lossy(&bytes[..read])
                    .lines()
                    .next()
                    .unwrap()
                    .to_owned(),
            );

            match reply {
                Reply::Busy(status) => write!(
                    socket, "HTTP/1.1 {status} Busy\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                ).unwrap(),
                Reply::Redirect => write!(
                    socket, "HTTP/1.1 307 Temporary Redirect\r\nLocation: http://{address}/redirected\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                ).unwrap(),
                Reply::Oversized => {
                    let body = vec![0; 2 * 1024 * 1024 + 1];
                    write!(socket, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).unwrap();
                    // Early rejection can close the socket before the final byte.
                    if let Err(error) = socket.write_all(&body) {
                        assert!(matches!(error.kind(), std::io::ErrorKind::BrokenPipe | std::io::ErrorKind::ConnectionReset), "{error}");
                    }
                }
                Reply::Stall => {
                    // Keep the accepted request unanswered until the CLI exits.
                    let _ = stopped.recv_timeout(Duration::from_secs(8));
                    break;
                }
            }
        }

        requests
    });
    let mut process = Command::new(env!("CARGO_BIN_EXE_canic"));
    process.args([
        "db",
        "inspect",
        "--network",
        "local",
        "--root-key",
        &include_bytes!("../../canic-host/src/network/mainnet-root.der")
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "--root-subnet",
        "tdb26-jop6k-aogll-7ltgs-eruif-6kk7m-qpktf-gdiqx-mxtrf-vb5e6-eqe",
        "--routing-canister",
        "aaaaa-aa",
        "--url",
        &format!("http://{address}"),
        "--canister",
        "aaaaa-aa",
        "--anonymous",
        "--timeout-secs",
        "1",
        "--json",
    ]);
    let mut child = process
        .env_remove("IC_TESTKIT_POCKET_IC_URL")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let started = Instant::now();
    let mut exceeded = false;
    while child.try_wait().unwrap().is_none() {
        if started.elapsed() > Duration::from_secs(5) {
            exceeded = true;
            child.kill().unwrap();
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    let _ = stop.send(());
    let requests = peer.join().unwrap();
    assert!(!exceeded, "CLI exceeded the test watchdog");
    assert!(!output.status.success());
    assert!(
        output.stdout.is_empty(),
        "no partial JSON or fixture fallback"
    );

    (output, requests)
}

#[test]
fn provider_refusals_report_one_failed_call() {
    for (reply, reason) in [
        (Reply::Busy(429), "429"),
        (Reply::Busy(503), "503"),
        (Reply::Redirect, ""),
        (Reply::Stall, "deadline exceeded"),
        (Reply::Oversized, "byte allowance"),
    ] {
        let (output, requests) = refuse(reply);
        assert_eq!(requests.len(), 1, "{requests:?}");
        assert!(!requests[0].contains("/redirected"));
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(error.contains(reason), "{error}");
        assert_eq!(
            error.matches("query call (db_inspect)").count(),
            1,
            "{error}"
        );
        assert!(error.contains("cycles unavailable"), "{error}");
    }
}
