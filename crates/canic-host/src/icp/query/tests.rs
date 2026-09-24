//! Typed retry, failure precedence and actual HTTP request qualification.

use super::*;
use ic_agent::agent::agent_error::HttpErrorPayload;
use std::{
    cell::Cell,
    collections::VecDeque,
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    thread,
    time::Instant,
};

fn http_error(status: u16) -> IcpQueryError {
    IcpQueryError::Agent(Box::new(AgentError::HttpError(HttpErrorPayload {
        status,
        content_type: None,
        content: Vec::new(),
    })))
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

#[test]
fn read_query_retries_transients_and_preserves_the_last_typed_failure() {
    let calls = Cell::new(0);
    let mut results = VecDeque::from([Err(http_error(502)), Err(http_error(503)), Ok(vec![1, 2])]);
    let value = runtime()
        .block_on(retry_query(|| {
            calls.set(calls.get() + 1);
            std::future::ready(results.pop_front().unwrap())
        }))
        .unwrap();
    assert_eq!(value, vec![1, 2]);
    assert_eq!(calls.get(), ATTEMPTS);
    calls.set(0);
    let error = runtime()
        .block_on(retry_query(|| {
            calls.set(calls.get() + 1);
            std::future::ready(Err::<(), _>(http_error(504)))
        }))
        .unwrap_err();
    assert_eq!(calls.get(), ATTEMPTS);
    assert!(
        matches!(error, IcpQueryError::Agent(error) if matches!(*error, AgentError::HttpError(HttpErrorPayload { status: 504, .. })))
    );
}

#[test]
fn read_query_never_retries_authentication_integrity_or_decode_failures() {
    for error in [
        http_error(400),
        http_error(401),
        http_error(403),
        http_error(404),
        IcpQueryError::Agent(Box::new(AgentError::CertificateVerificationFailed())),
        IcpQueryError::Agent(Box::new(AgentError::QuerySignatureVerificationFailed)),
        IcpQueryError::Agent(Box::new(AgentError::ResponseSizeExceededLimit())),
        IcpQueryError::Agent(Box::new(AgentError::MessageError(
            "502 is only text".into(),
        ))),
        IcpQueryError::Decode(candid::decode_one::<u64>(&[0]).unwrap_err()),
    ] {
        assert!(!transient(&error));
        let mut error = Some(error);
        let calls = Cell::new(0);
        assert!(
            runtime()
                .block_on(retry_query(|| {
                    calls.set(calls.get() + 1);
                    std::future::ready(Err::<(), _>(error.take().unwrap()))
                }))
                .is_err()
        );
        assert_eq!(calls.get(), 1);
    }
}

#[test]
fn authenticated_query_http_502_retries_only_the_same_read_request() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let target = Principal::from_slice(&[1]);
    let argument = candid::encode_one([7_u8; 32]).unwrap();
    let expected_argument = argument.clone();
    let server = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(8);
        for _ in 0..ATTEMPTS {
            let mut connection = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(Instant::now() < deadline, "missing retry request");
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => panic!("accept failed: {error}"),
                }
            };
            let (head, body) = read_request(&mut connection);
            assert!(head.starts_with("POST "));
            assert!(head.contains(&format!("/canister/{target}/query ")));
            assert!(
                body.windows(expected_argument.len())
                    .any(|bytes| bytes == expected_argument)
            );
            assert!(body.windows(11).any(|bytes| bytes == b"read_status"));
            connection
                .write_all(
                    b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .unwrap();
        }
    });
    // This HTTP fault server owns only the query endpoint. Signature verification
    // is enabled in production and qualified by the real PocketIC journey; its
    // certificate read_state requests are outside this transport fault test.
    let agent = Agent::builder()
        .with_url(url)
        .with_verify_query_signatures(false)
        .build()
        .unwrap();
    let icp = IcpCli::new("unused-icp", Some("local".into()));
    let result = query_bytes(&icp, &agent, target, "read_status", &argument);
    server.join().unwrap();
    assert!(
        matches!(result, Err(IcpQueryError::Agent(error)) if matches!(*error, AgentError::HttpError(HttpErrorPayload { status: 502, .. })))
    );
    assert_eq!(icp.remote_call_count(), u64::try_from(ATTEMPTS).unwrap());
}

#[test]
fn query_context_clones_reuse_connections_across_fresh_agents() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let server = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(8);
        let mut connection = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(Instant::now() < deadline, "missing query connection");
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("accept failed: {error}"),
            }
        };
        // A second connection cannot satisfy this fixture: both requests must
        // arrive on the first socket, driven by the retained runtime.
        for method in ["first_read", "second_read"] {
            let (head, body) = read_request(&mut connection);
            assert!(head.starts_with("POST "));
            assert!(
                body.windows(method.len())
                    .any(|bytes| bytes == method.as_bytes())
            );
            connection.write_all(
                b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: keep-alive\r\n\r\n",
            ).unwrap();
        }
    });
    let icp = IcpCli::new("unused-icp", None);
    let clone = icp.clone();
    for (context, method) in [(&icp, "first_read"), (&clone, "second_read")] {
        let agent = Agent::builder()
            .with_url(&url)
            .with_http_client(context.query_transport().unwrap().client.clone())
            .with_verify_query_signatures(false)
            .build()
            .unwrap();
        let result = query_bytes(context, &agent, Principal::anonymous(), method, &[]);
        assert!(matches!(result, Err(IcpQueryError::Agent(error))
            if matches!(*error, AgentError::HttpError(HttpErrorPayload { status: 401, .. }))));
    }
    server.join().unwrap();
    assert_eq!(icp.remote_call_count(), 2);
    // Contexts can be dropped by async consumers without trying to block their runtime.
    runtime().block_on(async move {
        drop(clone);
        drop(icp);
    });
}

#[test]
fn query_runtime_drives_concurrent_contexts_without_serializing_reads() {
    let icp = IcpCli::new("unused-icp", None);
    let barrier = Arc::new(tokio::sync::Barrier::new(4));
    thread::scope(|scope| {
        let handles = (0..4)
            .map(|_| {
                let context = icp.clone();
                let barrier = Arc::clone(&barrier);
                scope.spawn(move || {
                    context.query_transport().unwrap().block_on(async {
                        tokio::time::timeout(Duration::from_secs(3), barrier.wait())
                            .await
                            .unwrap();
                    });
                })
            })
            .collect::<Vec<_>>();
        for handle in handles {
            handle.join().unwrap();
        }
    });
}

#[cfg(unix)]
#[test]
fn pooled_transport_rechecks_signer_and_network_authority() {
    use ic_agent::{Identity as _, identity::BasicIdentity};
    use std::{fs, os::unix::fs::PermissionsExt as _};

    let root = crate::test_support::temp_dir("pooled-query-authority");
    fs::create_dir_all(&root).unwrap();
    let executable = root.join("icp");
    fs::write(
        &executable,
        crate::test_support::tool_script(
            r#"#!/bin/sh
set -eu
case " $* " in
  *" --version "*) echo 'icp @ICP_VERSION@';;
  *" network status "*) echo network >> calls; cat network.json;;
  *" identity export "*) echo export >> calls; cat identity.pem;;
  *" identity principal "*) echo principal >> calls; cat principal;;
  *) exit 2;;
esac
"#,
        ),
    )
    .unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let icp = IcpCli::new(executable.to_str().unwrap(), Some("local".into()))
        .with_cwd(&root)
        .with_identity(Some("test-signer"));
    let transport = icp.query_transport().unwrap();
    for seed in [1, 2] {
        let pem = synthetic_identity_pem(seed);
        let signer = BasicIdentity::from_pem(&pem).unwrap().sender().unwrap();
        fs::write(root.join("identity.pem"), pem).unwrap();
        fs::write(root.join("principal"), signer.to_text()).unwrap();
        fs::write(
            root.join("network.json"),
            serde_json::json!({
                "api_url": format!("http://127.0.0.1:{seed}"),
                "root_key": format!("{seed:02x}"),
            })
            .to_string(),
        )
        .unwrap();
        let agent = icp
            .build_authenticated_agent(
                Agent::builder()
                    .with_http_client(transport.client.clone())
                    .with_max_response_body_size(RESPONSE_BYTES),
            )
            .unwrap();
        assert_eq!(agent.get_principal().unwrap(), signer);
        assert_eq!(agent.read_root_key(), [seed]);
        // A changed Principal under the same selected name must fail admission
        // before any network request, even after pooled setup succeeded.
        fs::write(root.join("principal"), Principal::anonymous().to_text()).unwrap();
        let result: Result<(), _> =
            icp.query_candid_readonly(Principal::anonymous(), "status", &());
        assert!(matches!(result, Err(IcpQueryError::Authority(error))
            if matches!(*error, IcpManagementCallError::ExportedIdentityConflict { .. })));
    }
    assert_eq!(icp.remote_call_count(), 0);
    let calls = fs::read_to_string(root.join("calls")).unwrap();
    for kind in ["network", "export", "principal"] {
        assert_eq!(calls.lines().filter(|line| *line == kind).count(), 4);
    }
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
fn synthetic_identity_pem(seed: u8) -> String {
    // Public deterministic fixture seed in an RFC 8410 PKCS#8 container. Build
    // the PEM locally so no stored credential or external key tool is required.
    let mut der = vec![
        0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04,
        0x20,
    ];
    der.extend([seed; 32]);
    let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut body = String::new();
    for chunk in der.as_chunks::<3>().0 {
        let value = (u32::from(chunk[0]) << 16) | (u32::from(chunk[1]) << 8) | u32::from(chunk[2]);
        for shift in [18, 12, 6, 0] {
            body.push(char::from(alphabet[((value >> shift) & 63) as usize]));
        }
    }
    format!("-----BEGIN PRIVATE KEY-----\n{body}\n-----END PRIVATE KEY-----\n")
}

fn read_request(connection: &mut TcpStream) -> (String, Vec<u8>) {
    connection
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    connection
        .set_write_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let mut reader = BufReader::new(connection);
    let mut head = String::new();
    let mut length = None;
    loop {
        let mut line = String::new();
        assert!(reader.read_line(&mut line).unwrap() > 0);
        if line == "\r\n" {
            break;
        }
        if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
            length = Some(value.trim().parse().unwrap());
        }
        head.push_str(&line);
    }
    let mut body = vec![0; length.unwrap()];
    reader.read_exact(&mut body).unwrap();
    (head, body)
}
