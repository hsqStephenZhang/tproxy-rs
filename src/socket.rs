use std::io;
use std::net::SocketAddr;
use std::os::unix::io::AsRawFd;

use socket2::Protocol;
use tokio::net::{TcpListener, TcpSocket, TcpStream};

use crate::mark::get_mark;

pub fn new_tcp_listener(addr: SocketAddr) -> io::Result<TcpListener> {
    let socket = match addr {
        SocketAddr::V4(_) => {
            socket2::Socket::new(socket2::Domain::IPV4, socket2::Type::STREAM, None)?
        }
        SocketAddr::V6(_) => {
            socket2::Socket::new(socket2::Domain::IPV6, socket2::Type::STREAM, None)?
        }
    };

    socket.set_ip_transparent(true)?;
    socket.set_nonblocking(true)?;
    socket.bind(&addr.into())?;
    socket.listen(1024)?;

    let listener = TcpListener::from_std(socket.into())?;

    Ok(listener)
}

pub async fn new_tcp_stream(remote_addr: SocketAddr) -> io::Result<TcpStream> {
    let socket = match remote_addr {
        SocketAddr::V4(_) => {
            socket2::Socket::new(socket2::Domain::IPV4, socket2::Type::STREAM, None)?
        }
        SocketAddr::V6(_) => {
            socket2::Socket::new(socket2::Domain::IPV6, socket2::Type::STREAM, None)?
        }
    };

    socket.set_keepalive(true)?;
    socket.set_nodelay(true)?;
    socket.set_nonblocking(true)?;
    socket.set_mark(get_mark())?;

    TcpSocket::from_std_stream(socket.into())
        .connect(remote_addr)
        .await
}

pub fn new_udp_listener(addr: SocketAddr) -> io::Result<unix_udp_sock::UdpSocket> {
    let socket = match addr {
        SocketAddr::V4(_) => {
            socket2::Socket::new(socket2::Domain::IPV4, socket2::Type::DGRAM, Some(Protocol::UDP))?
        }
        SocketAddr::V6(_) => {
            socket2::Socket::new(socket2::Domain::IPV6, socket2::Type::DGRAM, Some(Protocol::UDP))?
        }
    };

    socket.set_ip_transparent(true)?;
    socket.set_nonblocking(true)?;
    socket.set_broadcast(true)?;
    socket.set_reuse_port(true)?;
    socket.set_reuse_address(true)?;
    socket.set_mark(get_mark())?;

    let enable = 1u32;
    let payload = std::ptr::addr_of!(enable).cast();
    let (level, name) = match addr {
        SocketAddr::V4(_) => (libc::IPPROTO_IP, libc::IP_ORIGDSTADDR),
        SocketAddr::V6(_) => (libc::IPPROTO_IPV6, libc::IPV6_ORIGDSTADDR),
    };
    unsafe {
        if libc::setsockopt(
            socket.as_raw_fd(),
            level,
            name,
            payload,
            std::mem::size_of_val(&enable) as libc::socklen_t,
        ) < 0
        {
            return Err(io::Error::last_os_error());
        }
    }
    socket.bind(&addr.into())?;

    unix_udp_sock::UdpSocket::from_std(socket.into())
}

pub async fn new_udp_packet(
    local_addr: Option<SocketAddr>,
    addr: SocketAddr,
    iface: Option<&str>,
) -> std::io::Result<tokio::net::UdpSocket> {
    use socket2_ext::{AddressBinding, BindDeviceOption};

    let socket = if addr.is_ipv4() {
        let socket = socket2::Socket::new(socket2::Domain::IPV4, socket2::Type::DGRAM, None)?;
        if let Some(iface) = iface {
            socket.bind_to_device(BindDeviceOption::v4(iface))?;
        }
        socket
    } else {
        let socket = socket2::Socket::new(socket2::Domain::IPV6, socket2::Type::DGRAM, None)?;
        if let Some(iface) = iface {
            socket.bind_to_device(BindDeviceOption::v6(iface))?;
        }
        socket
    };
    socket.set_nonblocking(true)?;
    socket.set_reuse_address(true)?;
    socket.set_mark(get_mark())?;
    socket.set_ip_transparent(true)?;
    if let Some(local_addr) = local_addr {
        socket.bind(&local_addr.into())?;
    }

    let socket = tokio::net::UdpSocket::from_std(socket.into());
    if let Ok(ref socket) = socket {
        socket.connect(addr).await?;
    }
    socket
}
