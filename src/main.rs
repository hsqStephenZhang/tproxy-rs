use clap::Parser;
use std::{
    collections::HashSet,
    io,
    net::{IpAddr, SocketAddr},
    str::FromStr,
    sync::Arc,
};
use tokio::{net::TcpListener, sync::Mutex};
use tracing::*;
use unix_udp_sock::UdpSocket;

use mark::set_mark;
use socket::{new_tcp_listener, new_tcp_stream, new_udp_listener, new_udp_packet};
use tracing_subscriber::EnvFilter;

pub mod mark;
pub mod socket;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(long, default_value = "7893")]
    tproxy_port: u16,

    #[clap(long, default_value = "8964")]
    tproxy_remote_mark: u32,

    #[clap(long, default_value = "false")]
    ipv6: bool,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let args = Args::parse();

    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("debug"))
        .unwrap();

    tracing_subscriber::fmt().with_env_filter(filter).init();

    // avoid infinite loop in iptables
    set_mark(args.tproxy_remote_mark);
    let addr = if args.ipv6 {
        SocketAddr::new(IpAddr::from_str("::").unwrap(), args.tproxy_port)
    } else {
        SocketAddr::new(IpAddr::from_str("0.0.0.0").unwrap(), args.tproxy_port)
    };

    let listener = new_tcp_listener(addr.clone()).unwrap();
    let f1 = tokio::spawn(handle_tcp_stream(listener));

    let udplistener = new_udp_listener(addr.clone()).unwrap();
    let f2 = tokio::spawn(handle_udp_packet(udplistener));

    let _ = tokio::join!(f1, f2);
    Ok(())
}

pub struct Detector {
    set: HashSet<SocketAddr>,
}

impl Detector {
    pub fn new() -> Self {
        Self {
            set: HashSet::new(),
        }
    }

    pub fn insert(&mut self, addr: SocketAddr) -> bool {
        self.set.insert(addr)
    }

    pub fn remove(&mut self, addr: &SocketAddr) -> bool {
        self.set.remove(addr)
    }

    pub fn contains(&self, addr: &SocketAddr) -> bool {
        self.set.contains(addr)
    }
}

pub struct DetectGuard(SocketAddr, Arc<Mutex<Detector>>);

impl DetectGuard {
    pub async fn new(addr: SocketAddr, detector: Arc<Mutex<Detector>>) -> Self {
        detector.lock().await.insert(addr);
        Self(addr, detector)
    }

    pub async fn drop_manually(self) {
        self.1.lock().await.remove(&self.0);
    }
}

async fn handle_tcp_stream(listener: TcpListener) -> io::Result<()> {
    let routing_loop_detector = Arc::new(Mutex::new(Detector::new()));
    loop {
        match listener.accept().await {
            Ok((mut stream, _)) => {
                if let Err(e) = stream.set_nodelay(true) {
                    tracing::error!("error trying to set TCP nodelay: {}", e);
                }
                let remote_addr = stream.local_addr().unwrap();
                let local_addr = stream.peer_addr().unwrap();
                if routing_loop_detector.lock().await.contains(&local_addr) {
                    tracing::warn!("routing loop detected, drop connection from {:?}", local_addr);
                    continue;
                }

                let detector = routing_loop_detector.clone();

                tokio::spawn(async move {
                    let mut proxy_to_target = new_tcp_stream(remote_addr).await.unwrap();
                    let guard  = DetectGuard::new(proxy_to_target.local_addr().unwrap(), detector).await;

                    tracing::info!(
                        "proxy to target: {:?} => {:?}",
                        proxy_to_target.local_addr(),
                        proxy_to_target.peer_addr()
                    );
                    let status =
                        tokio::io::copy_bidirectional(&mut stream, &mut proxy_to_target).await;
                    tracing::debug!(
                        "serve stream: local: {:?} =>remote: {:?} finished, send:{}, receive:{}",
                        &remote_addr,
                        &local_addr,
                        status.as_ref().map(|(a, _)| *a).unwrap_or_default(),
                        status.as_ref().map(|(_, b)| *b).unwrap_or_default(),
                    );
                    guard.drop_manually().await;
                });
            }
            Err(e) => {
                // Connection errors can be ignored directly, continue by
                // accepting the next request.
                tracing::error!("accept error: {}", e);
                continue;
            }
        };
    }
}

#[derive(Debug)]
pub struct UdpPacket {
    data: Vec<u8>,
    src_addr: SocketAddr,
    dst_addr: SocketAddr,
}

async fn handle_udp_packet(tsocket: UdpSocket) -> io::Result<()> {
    let tsocket = Arc::new(tsocket);

    let (tsocket_to_proxy_socket_tx, mut tsocket_to_proxy_socket_rx) =
        tokio::sync::mpsc::unbounded_channel::<UdpPacket>();
    let (proxy_to_tsocket_socket_tx, mut proxy_to_tsocket_socket_rx) =
        tokio::sync::mpsc::unbounded_channel::<UdpPacket>();

    let tsocket_f1 = tokio::spawn(async move {
        let mut buf = vec![0_u8; 1024 * 64];
        while let Ok(meta) = tsocket.recv_msg(&mut buf).await {
            match meta.orig_dst {
                Some(orig_dst) => {
                    if orig_dst.ip().is_multicast()
                        || match orig_dst.ip() {
                            std::net::IpAddr::V4(ip) => ip.is_broadcast(),
                            std::net::IpAddr::V6(_) => false,
                        }
                    {
                        continue;
                    }

                    let pkt = UdpPacket {
                        data: buf[..meta.len].to_vec(),
                        src_addr: meta.addr.into(),
                        dst_addr: orig_dst.into(),
                    };
                    trace!(
                        "tproxy -> dispatcher: {:?}",
                        (pkt.src_addr, pkt.dst_addr, pkt.data.len())
                    );
                    match tsocket_to_proxy_socket_tx.send(pkt) {
                        Ok(_) => {}
                        Err(e) => {
                            warn!("failed to send udp packet to proxy: {}", e);
                            continue;
                        }
                    }
                }
                None => {
                    warn!("failed to get orig_dst");
                    continue;
                }
            }
        }
    });

    // recv from dispatcher + write back to kernel stack
    let tsocket_f2 = tokio::spawn(async move {
        while let Some(pkt) = proxy_to_tsocket_socket_rx.recv().await {
            trace!(
                "tproxy -> write back: {:?}",
                (pkt.src_addr, pkt.dst_addr, pkt.data.len())
            );
            let write_back_socket = new_udp_packet(Some(pkt.src_addr), pkt.dst_addr, None)
                .await
                .unwrap();
            let meta = write_back_socket.send_to(&pkt.data[..], pkt.dst_addr).await;
            match meta {
                Ok(_) => {}
                Err(e) => {
                    warn!(
                        "failed to send msg:{:?} to {:?}, error: {}",
                        pkt, pkt.dst_addr, e
                    );
                }
            }
        }
    });

    let proxy_socket = tokio::spawn(async move {
        // recv from tproxy + send to remote
        while let Some(pkt) = tsocket_to_proxy_socket_rx.recv().await {
            let new_socket = new_udp_packet(None, pkt.dst_addr.into(), None)
                .await
                .unwrap();
            let sender = proxy_to_tsocket_socket_tx.clone();
            match new_socket.send(&pkt.data[..]).await {
                Ok(_) => {
                    let mut buf = vec![0_u8; 1024];
                    // recv from remote + send to tproxy
                    tokio::spawn(async move {
                        while let Ok((len, addr)) = new_socket.recv_from(&mut buf).await {
                            let pkt = UdpPacket {
                                data: buf[..len].to_vec(),
                                src_addr: addr.into(),
                                dst_addr: pkt.src_addr,
                            };
                            trace!(
                                "dispatcher -> tproxy: {:?}",
                                (pkt.src_addr, pkt.dst_addr, pkt.data.len())
                            );
                            match sender.send(pkt) {
                                Ok(_) => {}
                                Err(e) => {
                                    warn!("failed to send udp packet to tproxy: {}", e);
                                    continue;
                                }
                            }
                        }
                    });
                }
                Err(e) => {
                    warn!(
                        "failed to send msg:{:?} to {:?}, error: {}",
                        pkt, pkt.dst_addr, e
                    );
                }
            }
        }
    });

    let _ = tokio::join!(tsocket_f1, tsocket_f2, proxy_socket);

    Ok(())
}
