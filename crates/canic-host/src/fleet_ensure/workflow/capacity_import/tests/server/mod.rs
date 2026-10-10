//! Bounded HTTP query fixture: Root replies, Coordinator fails, every update is rejected.

use candid::Principal;
use canic_contracts::dto::{
    error::Error,
    pool_import::{PoolImportContext, PoolImportStatus},
    wire::projection::capacity_import::{StatusRequest, StatusResponse},
};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    thread,
    thread::JoinHandle,
    time::{Duration, Instant},
};

#[derive(serde::Deserialize)]
struct QueryEnvelope {
    content: QueryContent,
}

#[derive(serde::Deserialize)]
struct QueryContent {
    #[serde(with = "serde_bytes")]
    arg: Vec<u8>,
}

#[derive(serde::Serialize)]
struct Reply {
    #[serde(with = "serde_bytes")]
    arg: Vec<u8>,
}

#[derive(serde::Serialize)]
struct Response {
    status: &'static str,
    reply: Reply,
}

pub(super) struct QueryFailureServer {
    pub url: String,
    server: JoinHandle<()>,
}

impl QueryFailureServer {
    pub fn start(root: Principal, status: PoolImportStatus, context: PoolImportContext) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let server = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(10);
            // Each attempt reads progress, reads destination context, then fails
            // its Coordinator query. No update endpoint belongs to this fixture.
            for _ in 0..9 {
                let mut stream = loop {
                    match listener.accept() {
                        Ok((stream, _)) => break stream,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            assert!(Instant::now() < deadline, "missing query");
                            thread::sleep(Duration::from_millis(5));
                        }
                        Err(error) => panic!("query accept: {error}"),
                    }
                };
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                stream
                    .set_write_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let (path, body) = request(&stream);
                assert!(path.ends_with("/query"), "unexpected effect: {path}");
                let (code, reply) = if path.contains(&format!("/canister/{root}/")) {
                    ("200 OK", root_reply(&body, &status, &context))
                } else {
                    ("502 Bad Gateway", Vec::new())
                };
                write!(stream, "HTTP/1.1 {code}\r\nContent-Type: application/cbor\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", reply.len()).unwrap();
                stream.write_all(&reply).unwrap();
            }
        });
        Self { url, server }
    }

    pub fn finish(self) {
        self.server.join().unwrap();
    }
}

fn request(stream: &TcpStream) -> (String, Vec<u8>) {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    let path = line.split_whitespace().nth(1).unwrap().to_owned();
    let mut length = None;
    loop {
        line.clear();
        assert!(reader.read_line(&mut line).unwrap() > 0);
        if line == "\r\n" {
            break;
        }
        if let Some((name, value)) = line.split_once(':')
            && name.eq_ignore_ascii_case("content-length")
        {
            length = Some(value.trim().parse::<usize>().unwrap());
        }
    }
    let length = length.unwrap();
    assert!(length < 64 * 1024);
    let mut body = vec![0; length];
    reader.read_exact(&mut body).unwrap();
    (path, body)
}

fn root_reply(body: &[u8], status: &PoolImportStatus, context: &PoolImportContext) -> Vec<u8> {
    let envelope: QueryEnvelope = ciborium::de::from_reader(body).unwrap();
    let response = match candid::decode_one::<StatusRequest>(&envelope.content.arg).unwrap() {
        StatusRequest::PoolImport(_) => StatusResponse::PoolImport(Box::new(status.clone())),
        StatusRequest::PoolImportContext => {
            StatusResponse::PoolImportContext(Box::new(context.clone()))
        }
    };
    let response = Response {
        status: "replied",
        reply: Reply {
            arg: candid::encode_one(Ok::<_, Error>(response)).unwrap(),
        },
    };
    let mut bytes = Vec::new();
    ciborium::ser::into_writer(&response, &mut bytes).unwrap();
    bytes
}
