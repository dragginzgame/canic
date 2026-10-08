use super::{
    LOCAL_HTTP_TIMEOUT, get_http_status, http_request, local_query_with_endpoint,
    local_replica_endpoint_with_port,
};
use crate::replica_query::ReplicaQueryError;
use reqwest::Method;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::mpsc,
    thread::{self, JoinHandle},
    time::Duration,
};

#[test]
fn local_replica_endpoint_defaults_to_icp_cli_port() {
    assert_eq!(
        local_replica_endpoint_with_port(None, None),
        "http://127.0.0.1:8000"
    );
    assert_eq!(
        local_replica_endpoint_with_port(None, Some(8001)),
        "http://127.0.0.1:8001"
    );
    assert_eq!(
        local_replica_endpoint_with_port(Some("http://127.0.0.1:9000/"), Some(8001)),
        "http://127.0.0.1:9000"
    );
}

#[test]
fn status_reads_chunked_http_on_the_selected_origin() {
    let (endpoint, server) = serve(|stream| {
        stream.write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n2\r\nab\r\n1\r\nc\r\n0\r\n\r\n").unwrap();
    });
    assert_eq!(
        get_http_status(&format!("{endpoint}/ignored?query=ignored#fragment")).unwrap(),
        b"abc"
    );
    let request = server.join().unwrap();
    assert!(
        request
            .headers
            .starts_with("GET /api/v2/status HTTP/1.1\r\n")
    );
    assert!(
        !request
            .headers
            .to_ascii_lowercase()
            .contains("authorization:")
    );
    assert!(request.body.is_empty());
}

#[test]
fn query_posts_anonymous_cbor_and_decodes_a_chunked_reply() {
    // The canonical codec golden reply contains the four DIDL bytes.
    let reply = b"\xa2\x66status\x67replied\x65reply\xa1\x63arg\x44DIDL";
    let (endpoint, server) = serve(move |stream| {
        stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/cbor\r\nTransfer-Encoding: chunked\r\n\r\n").unwrap();
        write!(stream, "{:x}\r\n", reply.len()).unwrap();
        stream.write_all(reply).unwrap();
        stream.write_all(b"\r\n0\r\n\r\n").unwrap();
    });
    let canister = candid::Principal::from_slice(&[1, 2, 3]);
    let arg = [0x44, 0x49, 0x44, 0x4c];
    assert_eq!(
        local_query_with_endpoint(&canister.to_text(), "canic_public_status", &arg, endpoint)
            .unwrap(),
        arg
    );
    let request = server.join().unwrap();
    assert!(request.headers.starts_with(&format!(
        "POST /api/v2/canister/{canister}/query HTTP/1.1\r\n"
    )));
    assert!(
        request
            .headers
            .to_ascii_lowercase()
            .contains("content-type: application/cbor\r\n")
    );
    assert!(
        !request
            .headers
            .to_ascii_lowercase()
            .contains("authorization:")
    );
    let envelope: ciborium::value::Value =
        ciborium::de::from_reader(request.body.as_slice()).unwrap();
    let content = field(&envelope, "content");
    assert_eq!(
        field(content, "sender"),
        &ciborium::value::Value::Bytes(candid::Principal::anonymous().as_slice().to_vec())
    );
    assert_eq!(
        field(content, "canister_id"),
        &ciborium::value::Value::Bytes(canister.as_slice().to_vec())
    );
    assert_eq!(
        field(content, "arg"),
        &ciborium::value::Value::Bytes(arg.to_vec())
    );
    assert_eq!(
        field(content, "method_name"),
        &ciborium::value::Value::Text("canic_public_status".into())
    );
}

#[test]
fn http_refusal_and_redirect_do_not_become_success_or_a_second_request() {
    for response in [
        b"HTTP/1.1 503 Unavailable\r\nContent-Length: 0\r\n\r\n".as_slice(),
        b"HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/\r\nContent-Length: 0\r\n\r\n"
            .as_slice(),
    ] {
        let (endpoint, server) = serve(move |stream| stream.write_all(response).unwrap());
        assert!(matches!(
            get_http_status(&endpoint),
            Err(ReplicaQueryError::Query(_))
        ));
        assert!(
            server
                .join()
                .unwrap()
                .headers
                .starts_with("GET /api/v2/status ")
        );
    }
}

#[test]
fn credentials_and_other_schemes_are_refused_before_http() {
    for endpoint in [
        "http://user:password@127.0.0.1:1",
        "https://127.0.0.1:1",
        "file:///tmp/status",
    ] {
        assert!(matches!(
            get_http_status(endpoint),
            Err(ReplicaQueryError::Query(_))
        ));
    }
}

#[test]
fn stalled_response_retains_a_typed_timeout() {
    let (release, wait) = mpsc::channel();
    let (endpoint, server) = serve(move |stream| {
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\n\r\n")
            .unwrap();
        let _ = wait.recv_timeout(Duration::from_secs(5));
    });
    let result = http_request(
        &endpoint,
        "/api/v2/status",
        Method::GET,
        None,
        Duration::from_secs(1),
    );
    release.send(()).unwrap();
    let request = server.join().unwrap();
    assert!(request.headers.starts_with("GET /api/v2/status "));
    let Err(ReplicaQueryError::Io(error)) = result else {
        panic!("expected typed transport failure")
    };
    assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
    assert!(
        error
            .get_ref()
            .unwrap()
            .downcast_ref::<reqwest::Error>()
            .unwrap()
            .is_timeout()
    );
}

#[test]
fn connection_refusal_retains_the_native_kind_and_http_cause() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    let Err(ReplicaQueryError::Io(error)) = http_request(
        &endpoint,
        "/api/v2/status",
        Method::GET,
        None,
        LOCAL_HTTP_TIMEOUT,
    ) else {
        panic!("expected typed transport failure")
    };
    assert_eq!(error.kind(), std::io::ErrorKind::ConnectionRefused);
    assert!(error.get_ref().unwrap().is::<reqwest::Error>());
}

/// Exact bytes observed by a local HTTP fixture, without canister effects.
struct ObservedRequest {
    headers: String,
    body: Vec<u8>,
}

fn serve(
    reply: impl FnOnce(&mut TcpStream) + Send + 'static,
) -> (String, JoinHandle<ObservedRequest>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut header = Vec::new();
        while !header.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            stream.read_exact(&mut byte).unwrap();
            header.push(byte[0]);
        }
        let headers = String::from_utf8(header).unwrap();
        let length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().unwrap())
            })
            .unwrap_or(0);
        let mut body = vec![0; length];
        stream.read_exact(&mut body).unwrap();
        reply(&mut stream);
        ObservedRequest { headers, body }
    });
    (endpoint, server)
}

fn field<'a>(value: &'a ciborium::value::Value, name: &str) -> &'a ciborium::value::Value {
    value
        .as_map()
        .unwrap()
        .iter()
        .find(|(key, _)| key.as_text() == Some(name))
        .map(|(_, value)| value)
        .unwrap()
}
