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
