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
use mio::{
    net::{TcpStream, UdpSocket},
    Interest, Poll, Token,
};
use std::{
    io::{ErrorKind, Result},
    net::{Shutdown, SocketAddr},
    os::unix::io::{AsRawFd, FromRawFd},
};

pub(crate) enum TransportProtocol {
    Tcp,
    Udp,
}

pub(crate) enum InternetProtocol {
    Ipv4,
    Ipv6,
}

pub(crate) struct Socket {
    _socket: socket2::Socket, // Need to retain so socket does not get closed.
    connection: Connection,
}

enum Connection {
    Tcp(TcpStream),
    Udp(UdpSocket),
}

impl Socket {
    pub(crate) fn new(transport_protocol: TransportProtocol, internet_protocol: InternetProtocol, remote_address: SocketAddr) -> Option<Socket> {
        let socket = Self::create_socket(&transport_protocol, &internet_protocol);

        on_socket_created(socket.as_raw_fd());

        let socket_address = socket2::SockAddr::from(remote_address);

        log::debug!("connecting to host, address={:?}", remote_address);

        match socket.connect(&socket_address) {
            Ok(_) => {
                log::debug!("connected to host, address={:?}", remote_address);
            }
            Err(error) => {
                if error.kind() == ErrorKind::WouldBlock || error.raw_os_error() == Some(libc::EINPROGRESS) {
                    // do nothing.
                } else {
                    log::error!(
                        "failed to connect to host, error={:?} address={:?}",
                        error,
                        remote_address
                    );
                    return None;
                }
            }
        }

        let connection = Self::create_connection(&transport_protocol, &socket);

        Some(Socket {
            _socket: socket,
            connection,
        })
    }

    pub(crate) fn register_poll(&mut self, poll: &mut Poll, token: Token) -> std::io::Result<()> {
        match &mut self.connection {
            Connection::Tcp(connection) => {
                let interests = Interest::READABLE | Interest::WRITABLE;
                poll.registry().register(connection, token, interests)
            }
            Connection::Udp(connection) => {
                let interests = Interest::READABLE;
                poll.registry().register(connection, token, interests)
            }
        }
    }

    pub(crate) fn deregister_poll(&mut self, poll: &mut Poll) -> std::io::Result<()> {
        match &mut self.connection {
            Connection::Tcp(connection) => poll.registry().deregister(connection),
            Connection::Udp(connection) => poll.registry().deregister(connection),
        }
    }

    pub(crate) fn write(&mut self, bytes: &[u8]) -> Result<usize> {
        match &mut self.connection {
            Connection::Tcp(connection) => connection.write(bytes),
            Connection::Udp(connection) => connection.write(bytes),
        }
    }

    pub(crate) fn read(&mut self) -> Result<(Vec<Vec<u8>>, bool)> {
        match &mut self.connection {
            Connection::Tcp(connection) => Self::read_all(connection),
            Connection::Udp(connection) => Self::read_all(connection),
        }
    }

    pub(crate) fn close(&self) {
        match &self.connection {
            Connection::Tcp(connection) => {
                if let Err(error) = connection.shutdown(Shutdown::Both) {
                    log::trace!("failed to shutdown tcp stream, error={:?}", error);
                }
            }
            Connection::Udp(_) => {
                // UDP connections do not require to be closed.
            }
        }
    }

    fn create_socket(transport_protocol: &TransportProtocol, internet_protocol: &InternetProtocol) -> socket2::Socket {
        let domain = match internet_protocol {
            InternetProtocol::Ipv4 => socket2::Domain::IPV4,
            InternetProtocol::Ipv6 => socket2::Domain::IPV6,
        };

        let protocol = match transport_protocol {
            TransportProtocol::Tcp => socket2::Protocol::TCP,
            TransportProtocol::Udp => socket2::Protocol::UDP,
        };

        let socket_type = match transport_protocol {
            TransportProtocol::Tcp => socket2::Type::STREAM,
            TransportProtocol::Udp => socket2::Type::DGRAM,
        };

        let socket = socket2::Socket::new(domain, socket_type, Some(protocol)).unwrap();

        socket.set_nonblocking(true).unwrap();

        socket
    }

    fn create_connection(transport_protocol: &TransportProtocol, socket: &socket2::Socket) -> Connection {
        match transport_protocol {
            TransportProtocol::Tcp => {
                let tcp_stream = unsafe { TcpStream::from_raw_fd(socket.as_raw_fd()) };
                Connection::Tcp(tcp_stream)
            }
            TransportProtocol::Udp => {
                let udp_socket = unsafe { UdpSocket::from_raw_fd(socket.as_raw_fd()) };
                Connection::Udp(udp_socket)
            }
        }
    }

    fn read_all<R>(reader: &mut R) -> Result<(Vec<Vec<u8>>, bool)>
    where
        R: Read,
    {
        let mut bytes: Vec<Vec<u8>> = Vec::new();
        let mut buffer = [0; 1 << 16]; // maximum UDP packet size
        let mut is_closed = false;
        loop {
            match reader.read(&mut buffer[..]) {
                Ok(count) => {
                    if count == 0 {
                        is_closed = true;
                        break;
                    }
                    // bytes.extend_from_slice(&buffer[..count]);
                    let data = buffer[..count].to_vec();
                    bytes.push(data)
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
