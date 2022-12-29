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

use super::buffers::Buffers;
use super::connection::{Connection, ConnectionProtocol};
use super::session_info::SessionInfo;
use super::vpn_device::VpnDevice;
use mio::{Poll, Token};
use smoltcp::iface::{Interface, SocketHandle};
use smoltcp::socket::{TcpSocket, TcpSocketBuffer};
use smoltcp::wire::{IpAddress, IpEndpoint, IpProtocol, Ipv4Address};

pub(crate) struct Session {
    pub(crate) socket_handle: SocketHandle,
    pub(crate) connection: Connection,
    pub(crate) token: Token,
    pub(crate) buffers: Buffers,
}

impl Session {
    pub(crate) fn new(session_info: &SessionInfo, interface: &mut Interface<VpnDevice>, poll: &mut Poll, token: Token) -> Option<Session> {
        let ip_protocol = IpProtocol::from(session_info.protocol);

        let socket_handle = match ip_protocol {
            IpProtocol::Tcp => {
                let socket = Self::create_tcp_socket(session_info).unwrap();
                interface.add_socket(socket)
            }
            //
            // Temporarily disabling UDP
            //
            // IpProtocol::Udp => {
            //     let socket = Self::create_udp_socket(session_info).unwrap();
            //     interface.add_socket(socket)
            // }
            _ => return None,
        };

        let transport_protocol = match ip_protocol {
            IpProtocol::Tcp => ConnectionProtocol::Tcp,
            IpProtocol::Udp => ConnectionProtocol::Udp,
            _ => return None,
        };

        let mut connection = Connection::new(
            transport_protocol,
            session_info.dst_ip,
            session_info.dst_port,
        )?;

        if let Err(error) = connection.register_poll(poll, token) {
            log::error!("failed to register poll, error={:?}", error);
            return None;
        }

        let session = Session {
            socket_handle,
            connection,
            token,
            buffers: Buffers::new(),
        };

        Some(session)
    }

    fn create_tcp_socket<'a>(session_info: &SessionInfo) -> Option<TcpSocket<'a>> {
        let mut socket = TcpSocket::new(
            TcpSocketBuffer::new(vec![0; 1048576]),
            TcpSocketBuffer::new(vec![0; 1048576]),
        );

        let dst_ip = Ipv4Address::from_bytes(&session_info.dst_ip);
        let dst_endpoint = IpEndpoint::new(IpAddress::from(dst_ip), session_info.dst_port);
        if socket.listen(dst_endpoint).is_err() {
            log::error!("failed to listen on socket, session=[{}]", session_info);
            return None;
        }

        socket.set_ack_delay(None);

        Some(socket)
    }

    //
    // Temporarily disabling UDP.
    //
    // fn create_udp_socket<'a>(session_info: &SessionInfo) -> Option<UdpSocket<'a>> {
    //     let mut socket = UdpSocket::new(
    //         UdpSocketBuffer::new(Vec::new(), vec![0; 1048576]),
    //         UdpSocketBuffer::new(Vec::new(), vec![0; 1048576]),
    //     );

    //     let dst_ip = Ipv4Address::from_bytes(&session_info.dst_ip);
    //     let dst_endpoint = IpEndpoint::new(IpAddress::from(dst_ip), session_info.dst_port);
    //     if socket.bind(dst_endpoint).is_err() {
    //         log::error!("failed to bind socket, session=[{}]", session_info);
    //         return None;
    //     }

    //     Some(socket)
    // }
}
