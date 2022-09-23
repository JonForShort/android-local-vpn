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
use mio::unix::SourceFd;
use mio::{Interest, Poll, Token};
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::io::{ErrorKind, Read, Result};
use std::net::SocketAddr;
use std::os::unix::io::AsRawFd;

pub(crate) struct TcpStream {
    socket: Option<Socket>,
}

impl TcpStream {
    pub(crate) fn new() -> TcpStream {
        TcpStream { socket: None }
    }

    pub(crate) fn connect(&mut self, ip: [u8; 4], port: u16) {
        let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP)).unwrap();

        on_socket_created(socket.as_raw_fd());

        let address = SockAddr::from(SocketAddr::from((ip, port)));

        log::trace!("connecting to host, address={:?}", address);

        match socket.connect(&address) {
            Ok(_) => {
                log::trace!("connected to host, address={:?}", address);
            }
            Err(error) => {
                log::error!(
                    "failed to connect to host, error={:?} address={:?}",
                    error,
                    address
                );
            }
        }

        socket.set_nonblocking(true).unwrap();

        self.socket = Some(socket);
    }

    pub(crate) fn register_poll(&self, poll: &mut Poll, token: Token) {
        let raw_fd = &self.socket.as_ref().unwrap().as_raw_fd();
        poll.registry()
            .register(
                &mut SourceFd(raw_fd),
                token,
                Interest::READABLE | Interest::WRITABLE,
            )
            .unwrap()
    }

    pub(crate) fn deregister_poll(&self, poll: &mut Poll) {
        let raw_fd = &self.socket.as_ref().unwrap().as_raw_fd();
        poll.registry().deregister(&mut SourceFd(raw_fd)).unwrap()
    }

    pub(crate) fn write(&mut self, bytes: &[u8]) -> Result<usize> {
        self.socket.as_ref().unwrap().send(bytes)
    }

    pub(crate) fn read(&mut self) -> Result<Vec<u8>> {
        let mut bytes: Vec<u8> = Vec::new();
        let mut read_buffer = [0; 1024];
        if let Some(socket) = &mut self.socket {
            loop {
                let result = socket.read(&mut read_buffer[..]);
                match result {
                    Ok(read_bytes) => {
                        if read_bytes == 0 {
                            break;
                        }
                        bytes.extend_from_slice(&read_buffer[..read_bytes]);
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
        Ok(bytes)
    }
}
