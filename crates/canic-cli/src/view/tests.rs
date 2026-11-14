use super::*;
use std::{
    io::{Read, Write},
    net::TcpStream,
    time::Duration,
};

#[test]
fn viewer_serves_only_the_report_over_loopback() {
    let server = Server::http((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let address = server.server_addr().to_ip().unwrap();
    assert!(address.ip().is_loopback());
    let worker = std::thread::spawn(move || {
        for _ in 0..5 {
            let request = server
                .recv_timeout(Duration::from_secs(5))
                .unwrap()
                .expect("request");
            respond(request, b"<h1>Canic snapshot</h1>", address.port()).unwrap();
        }
    });
    for (method, path, host, status, body) in [
        (
            "GET",
            "/",
            format!("localhost:{}", address.port()),
            "200",
            true,
        ),
        (
            "HEAD",
            "/",
            format!("127.0.0.1:{}", address.port()),
            "200",
            false,
        ),
        (
            "GET",
            "/.canic/signed.json",
            format!("localhost:{}", address.port()),
            "404",
            false,
        ),
        (
            "POST",
            "/",
            format!("localhost:{}", address.port()),
            "405",
            false,
        ),
        ("GET", "/", "external.example".into(), "421", false),
    ] {
        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        write!(stream, "{method} {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\nContent-Length: 0\r\n\r\n").unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        assert!(
            response.starts_with(&format!("HTTP/1.1 {status}")),
            "{response}"
        );
        assert_eq!(response.contains("<h1>Canic snapshot</h1>"), body);
        assert!(
            response
                .to_ascii_lowercase()
                .contains("cache-control: no-store")
        );
    }
    worker.join().unwrap();
}

#[test]
fn polling_viewer_serves_the_latest_page_without_waiting_for_a_refresh() {
    let viewer = Viewer::new(0, false, "test live viewer").unwrap();
    let address = viewer.server.server_addr().to_ip().unwrap();
    for content in ["original", "refresh failed: original", "updated"] {
        let client = std::thread::spawn(move || {
            let mut stream = TcpStream::connect(address).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            write!(
                stream,
                "GET / HTTP/1.1\r\nHost: localhost:{}\r\nConnection: close\r\n\r\n",
                address.port()
            )
            .unwrap();
            let mut response = String::new();
            stream.read_to_string(&mut response).unwrap();
            response
        });
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while !client.is_finished() {
            assert!(std::time::Instant::now() < deadline);
            viewer.poll(content.as_bytes()).unwrap();
            std::thread::sleep(Duration::from_millis(5));
        }
        let response = client.join().unwrap();
        assert!(response.starts_with("HTTP/1.1 200"));
        assert!(response.ends_with(content));
    }
    assert!(viewer.poll(&vec![0; MAX_REPORT_BYTES + 1]).is_err());
}
