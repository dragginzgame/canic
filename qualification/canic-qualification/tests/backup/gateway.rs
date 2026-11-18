//! Local transport fault: discard the selected reply after the replica begins returning it.
use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::{
    io::{self, AsyncReadExt},
    net::{TcpListener, TcpStream},
    task::JoinHandle,
};

pub(super) struct Gateway {
    pub endpoint: String,
    pub requests: Arc<AtomicUsize>,
    pub lose_next_reply: Arc<AtomicBool>,
    worker: JoinHandle<()>,
}

impl Gateway {
    pub async fn start(port: u16) -> Self {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let lose_next_reply = Arc::new(AtomicBool::new(false));
        let requests = Arc::new(AtomicUsize::new(0));
        let drop_next = lose_next_reply.clone();
        let observed = requests.clone();
        let upstream = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
        let worker = tokio::spawn(async move {
            loop {
                let (mut host, _) = listener.accept().await.unwrap();
                observed.fetch_add(1, Ordering::SeqCst);
                let mut replica = TcpStream::connect(upstream).await.unwrap();
                tokio::time::timeout(Duration::from_secs(35), async {
                    if drop_next.swap(false, Ordering::SeqCst) {
                        let (mut read, mut write) = replica.split();
                        tokio::select! {
                            result = read.read_u8() => { result.unwrap(); }
                            result = io::copy(&mut host, &mut write) => {
                                result.unwrap();
                                read.read_u8().await.unwrap();
                            }
                        }
                    } else {
                        io::copy_bidirectional(&mut host, &mut replica)
                            .await
                            .unwrap();
                    }
                })
                .await
                .unwrap();
            }
        });
        Self {
            endpoint,
            requests,
            lose_next_reply,
            worker,
        }
    }
}

impl Drop for Gateway {
    fn drop(&mut self) {
        self.worker.abort();
    }
}
