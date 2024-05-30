use std::{
    io,
    net::{IpAddr, SocketAddr},
    str::FromStr,
};

use tcp::{
    mark::set_mark,
    transparent_socket::{new_listener, new_tcp_stream},
};
use tracing_subscriber::EnvFilter;

pub mod tcp;

#[tokio::main]
async fn main() -> io::Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("debug"))
        .unwrap();

    tracing_subscriber::fmt().with_env_filter(filter).init();

    // avoid infinite loop in iptables
    set_mark(0xff);

    let addr = SocketAddr::new(IpAddr::from_str("0.0.0.0").unwrap(), 7893);
    let listener = new_listener(addr)?;

    loop {
        match listener.accept().await {
            Ok((mut stream, _)) => {
                if let Err(e) = stream.set_nodelay(true) {
                    tracing::error!("error trying to set TCP nodelay: {}", e);
                }
                let remote_addr = stream.local_addr().unwrap();

                tokio::spawn(async move {
                    let local_addr = stream.peer_addr().unwrap();
                    tracing::info!(
                        "serve stream: local: {:?} =>remote: {:?}, ",
                        &remote_addr,
                        &local_addr
                    );
                    let mut proxy_to_target = new_tcp_stream(remote_addr).await.unwrap();

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
