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

use super::vpn_device::VpnDevice;
use smoltcp::iface::{Interface, SocketHandle};
use smoltcp::socket::{TcpSocket, TcpSocketBuffer, UdpSocket, UdpSocketBuffer};
use smoltcp::wire::{IpAddress, IpEndpoint, Ipv4Address};

pub(crate) enum Protocol {
    Tcp,
    Udp,
}

pub(crate) struct Socket {
    pub(crate) handle: SocketHandle,
}

impl Socket {
    pub(crate) fn new(protocol: Protocol, ip: [u8; 4], port: u16, interface: &mut Interface<VpnDevice>) -> Option<Socket> {
        let ip = IpAddress::from(Ipv4Address::from_bytes(&ip));
        let endpoint = IpEndpoint::new(ip, port);

        let handle = match protocol {
            Protocol::Tcp => {
                let socket = Self::create_tcp_socket(endpoint).unwrap();
                interface.add_socket(socket)
            }
            Protocol::Udp => {
                let socket = Self::create_udp_socket(endpoint).unwrap();
                interface.add_socket(socket)
            }
        };

        let socket = Socket { handle };

        Some(socket)
    }

    fn create_tcp_socket<'a>(endpoint: IpEndpoint) -> Option<TcpSocket<'a>> {
        let mut socket = TcpSocket::new(
            TcpSocketBuffer::new(vec![0; 1048576]),
            TcpSocketBuffer::new(vec![0; 1048576]),
        );

        if socket.listen(endpoint).is_err() {
            log::error!("failed to listen on socket, endpoint=[{}]", endpoint);
            return None;
        }

        socket.set_ack_delay(None);

        Some(socket)
    }

    fn create_udp_socket<'a>(endpoint: IpEndpoint) -> Option<UdpSocket<'a>> {
        let mut socket = UdpSocket::new(
            UdpSocketBuffer::new(Vec::new(), vec![0; 1048576]),
            UdpSocketBuffer::new(Vec::new(), vec![0; 1048576]),
        );

        if socket.bind(endpoint).is_err() {
            log::error!("failed to bind socket, endpoint=[{}]", endpoint);
            return None;
        }

        Some(socket)
    }
}
