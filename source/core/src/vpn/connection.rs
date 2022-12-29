// This is free and unencumbered software released into the public domain.
//
// Anyone is free to copy, modify, publish, use, compile, sell, or
// distribute this software, either in source code form or as a compiled
// binary, for any purpose, commercial or non-commercial, and by any
// means.
//
// In jurisdictions that recognize copyright laws, the author or authors
// of this software dedicate any and all copyright interest in the
// software to the public domain. We make this dedication for the benefit
// of the public at large and to the detriment of our heirs and
// successors. We intend this dedication to be an overt act of
// relinquishment in perpetuity of all present and future rights to this
// software under copyright law.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS BE LIABLE FOR ANY CLAIM, DAMAGES OR
// OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE,
// ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR
// OTHER DEALINGS IN THE SOFTWARE.
//
// For more information, please refer to <https://unlicense.org>

use crate::tun_callbacks::on_socket_created;
use mio::event;
use mio::net::TcpStream as MioTcpStream;
use mio::net::UdpSocket as MioUdpSocket;
use mio::{Interest, Poll, Token};
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::io::{ErrorKind, Result};
use std::net::{Shutdown, SocketAddr};
use std::os::unix::io::{AsRawFd, FromRawFd};

pub(crate) enum ConnectionProtocol {
    Tcp,
    Udp,
}

pub(crate) struct Connection {
    _socket: Socket, // Need to retain so socket does not get closed.
    connection: ConnectionType,
}

enum ConnectionType {
    Tcp(MioTcpStream),
    Udp(MioUdpSocket),
}

impl Connection {
    pub(crate) fn new(protocol: ConnectionProtocol, ip: [u8; 4], port: u16) -> Option<Connection> {
        let socket = Self::create_socket(&protocol);

        on_socket_created(socket.as_raw_fd());

        let address = SockAddr::from(SocketAddr::from((ip, port)));

        log::debug!("connecting to host, address={:?}", address);

        match socket.connect(&address) {
            Ok(_) => {
                log::debug!("connected to host, address={:?}", address);
            }
            Err(error) => {
                if error.kind() == ErrorKind::WouldBlock || error.raw_os_error() == Some(libc::EINPROGRESS) {
                    // do nothing.
                } else {
                    log::error!(
                        "failed to connect to host, error={:?} address={:?}",
                        error,
                        address
                    );
                    return None;
                }
            }
        }

        let connection = Self::create_connection(&protocol, &socket);

        Some(Connection {
            _socket: socket,
            connection,
        })
    }

    pub(crate) fn register_poll(&mut self, poll: &mut Poll, token: Token) -> std::io::Result<()> {
        match &mut self.connection {
            ConnectionType::Tcp(connection) => Self::register_poll_with_source(poll, connection, token),
            ConnectionType::Udp(connection) => Self::register_poll_with_source(poll, connection, token),
        }
    }

    pub(crate) fn deregister_poll(&mut self, poll: &mut Poll) -> std::io::Result<()> {
        match &mut self.connection {
            ConnectionType::Tcp(connection) => Self::deregister_poll_with_source(poll, connection),
            ConnectionType::Udp(connection) => Self::deregister_poll_with_source(poll, connection),
        }
    }

    pub(crate) fn write(&mut self, bytes: &[u8]) -> Result<usize> {
        match &mut self.connection {
            ConnectionType::Tcp(connection) => connection.write(bytes),
            ConnectionType::Udp(connection) => connection.write(bytes),
        }
    }

    pub(crate) fn read(&mut self) -> Result<(Vec<u8>, bool)> {
        match &mut self.connection {
            ConnectionType::Tcp(connection) => Self::read_all(connection),
            ConnectionType::Udp(connection) => Self::read_all(connection),
        }
    }

    pub(crate) fn close(&self) {
        match &self.connection {
            ConnectionType::Tcp(connection) => {
                if let Err(error) = connection.shutdown(Shutdown::Both) {
                    log::trace!("failed to shutdown tcp stream, error={:?}", error);
                }
            }
            ConnectionType::Udp(_) => {
                // UDP connections do not require to be closed.
            }
        }
    }

    fn create_socket(protocol: &ConnectionProtocol) -> Socket {
        let connection_protocol = match protocol {
            ConnectionProtocol::Tcp => Protocol::TCP,
            ConnectionProtocol::Udp => Protocol::UDP,
        };
        let connection_type = match protocol {
            ConnectionProtocol::Tcp => Type::STREAM,
            ConnectionProtocol::Udp => Type::DGRAM,
        };
        let socket = Socket::new(Domain::IPV4, connection_type, Some(connection_protocol)).unwrap();
        socket.set_nonblocking(true).unwrap();
        socket
    }

    fn create_connection(protocol: &ConnectionProtocol, socket: &Socket) -> ConnectionType {
        match protocol {
            ConnectionProtocol::Tcp => {
                let tcp_stream = unsafe { MioTcpStream::from_raw_fd(socket.as_raw_fd()) };
                ConnectionType::Tcp(tcp_stream)
            }
            ConnectionProtocol::Udp => {
                let udp_socket = unsafe { MioUdpSocket::from_raw_fd(socket.as_raw_fd()) };
                ConnectionType::Udp(udp_socket)
            }
        }
    }

    fn register_poll_with_source<S>(poll: &mut Poll, source: &mut S, token: Token) -> std::io::Result<()>
    where
        S: event::Source,
    {
        poll.registry()
            .register(source, token, Interest::READABLE | Interest::WRITABLE)
    }

    fn deregister_poll_with_source<S>(poll: &mut Poll, source: &mut S) -> std::io::Result<()>
    where
        S: event::Source,
    {
        poll.registry().deregister(source)
    }

    fn read_all<R>(reader: &mut R) -> Result<(Vec<u8>, bool)>
    where
        R: Read,
    {
        let mut bytes: Vec<u8> = Vec::new();
        let mut buffer = [0; 1024];
        let mut is_closed = false;
        loop {
            match reader.read(&mut buffer[..]) {
                Ok(count) => {
                    if count == 0 {
                        is_closed = true;
                        break;
                    }
                    bytes.extend_from_slice(&buffer[..count]);
                }
                Err(error_code) => {
                    if error_code.kind() == ErrorKind::WouldBlock {
                        break;
                    } else {
                        return Err(error_code);
                    }
                }
            }
        }
        Ok((bytes, is_closed))
    }
}

trait Read {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;
}

impl Read for mio::net::UdpSocket {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.recv(buf)
    }
}

impl Read for mio::net::TcpStream {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        <mio::net::TcpStream as std::io::Read>::read(self, buf)
    }
}

trait Write {
    fn write(&mut self, buf: &[u8]) -> Result<usize>;
}

impl Write for mio::net::UdpSocket {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.send(buf)
    }
}

impl Write for mio::net::TcpStream {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        <mio::net::TcpStream as std::io::Write>::write(self, buf)
    }
}
