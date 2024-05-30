use std::net::SocketAddr;
use std::os::unix::io::AsRawFd;
use std::{io, mem};

use socket2::Socket;
use tokio::net::{TcpListener, TcpSocket, TcpStream};

use super::mark::get_mark;

pub fn new_listener(addr: SocketAddr) -> io::Result<TcpListener> {
    let socket = Socket::new(socket2::Domain::IPV4, socket2::Type::STREAM, None)?;
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

    set_mark(socket.as_raw_fd(), get_mark())?;

    TcpSocket::from_std_stream(socket.into())
        .connect(remote_addr)
        .await
}

fn set_mark(socket_fd: i32, mark: i32) -> io::Result<()> {
    unsafe {
        let mark: libc::c_int = mark as _;
        let ret = libc::setsockopt(
            socket_fd,
            libc::SOL_SOCKET,
            libc::SO_MARK,
            &mark as *const _ as *const _,
            mem::size_of_val(&mark) as libc::socklen_t,
        );

        if ret != 0 {
            return Err(io::Error::last_os_error());
        }
    };
    Ok(())
}
