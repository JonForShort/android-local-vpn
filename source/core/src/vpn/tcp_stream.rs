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
use mio::net::TcpStream as MioTcpStream;
use mio::{Interest, Poll, Token};
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::io::{ErrorKind, Read, Result, Write};
use std::net::{Shutdown, SocketAddr};
use std::os::unix::io::{AsRawFd, FromRawFd};

pub(crate) struct TcpStream {
    socket: Option<Socket>,
    tcp_stream: Option<MioTcpStream>,
}

impl TcpStream {
    pub(crate) fn new() -> TcpStream {
        TcpStream {
            socket: None,
            tcp_stream: None,
        }
    }

    pub(crate) fn connect(&mut self, ip: [u8; 4], port: u16) {
        let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP)).unwrap();

        socket.set_nonblocking(true).unwrap();

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
                }
            }
        }

        let tcp_stream = unsafe { MioTcpStream::from_raw_fd(socket.as_raw_fd()) };

        self.socket = Some(socket);
        self.tcp_stream = Some(tcp_stream);
    }

    pub(crate) fn register_poll(&mut self, poll: &mut Poll, token: Token) {
        poll.registry()
            .register(
                self.tcp_stream.as_mut().unwrap(),
                token,
                Interest::READABLE | Interest::WRITABLE,
            )
            .unwrap()
    }

    pub(crate) fn deregister_poll(&mut self, poll: &mut Poll) {
        poll.registry()
            .deregister(self.tcp_stream.as_mut().unwrap())
            .unwrap()
    }

    pub(crate) fn write(&mut self, bytes: &[u8]) -> Result<usize> {
        self.tcp_stream.as_ref().unwrap().write(bytes)
    }

    pub(crate) fn read(&mut self) -> Result<(Vec<u8>, bool)> {
        let mut bytes: Vec<u8> = Vec::new();
        let mut buffer = [0; 1024];
        let mut is_closed = false;
        if let Some(tcp_stream) = &mut self.tcp_stream {
            loop {
                match tcp_stream.read(&mut buffer[..]) {
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
        }
        Ok((bytes, is_closed))
    }

    pub(crate) fn close(&self) {
        let tcp_stream = self.tcp_stream.as_ref().unwrap();
        if let Err(error) = tcp_stream.shutdown(Shutdown::Both) {
            log::trace!("failed to shutdown tcp stream, error={:?}", error);
        }
    }
}
