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

use super::session::Session;
use super::vpn_device::VpnDevice;
use smoltcp::iface::{Context, Interface, InterfaceBuilder, Routes, SocketHandle};
use smoltcp::socket::{TcpSocket, TcpSocketBuffer};
use smoltcp::wire::IpEndpoint;
use smoltcp::wire::{IpAddress, IpCidr, Ipv4Address};
use std::collections::btree_map::BTreeMap;

pub struct SessionData<'a> {
    interface: Interface<'a, VpnDevice>,
    socket_handle: SocketHandle,
}

impl<'a> SessionData<'a> {
    pub fn new(session: &Session) -> SessionData<'a> {
        let mut interface = InterfaceBuilder::new(VpnDevice::new(), vec![])
            .any_ip(true)
            .ip_addrs([IpCidr::new(IpAddress::v4(10, 0, 0, 2), 8)])
            .routes(SessionData::create_routes())
            .finalize();
        let socket_handle = interface.add_socket(SessionData::create_tcp_socket());
        let (socket, context) = interface.get_socket_and_context::<TcpSocket<'a>>(socket_handle);
        SessionData::connect_session(session, socket, context);
        SessionData {
            interface: interface,
            socket_handle: socket_handle,
        }
    }

    fn create_routes() -> Routes<'a> {
        let mut routes = Routes::new(BTreeMap::new());
        let default_gateway_ipv4 = Ipv4Address::new(10, 0, 0, 2);
        routes.add_default_ipv4_route(default_gateway_ipv4).unwrap();
        return routes;
    }

    fn create_tcp_socket() -> TcpSocket<'a> {
        let tcp_rx_buffer = TcpSocketBuffer::new(vec![0; 1024]);
        let tcp_tx_buffer = TcpSocketBuffer::new(vec![0; 1024]);
        return TcpSocket::new(tcp_rx_buffer, tcp_tx_buffer);
    }

    fn connect_session(session: &Session, socket: &mut TcpSocket, context: &mut Context) {
        let src_ip = Ipv4Address::from_bytes(&session.src_ip);
        let dst_ip = Ipv4Address::from_bytes(&session.dst_ip);
        let src_endpoint = IpEndpoint::new(IpAddress::from(src_ip), session.src_port);
        let dst_endpoint = IpEndpoint::new(IpAddress::from(dst_ip), session.dst_port);
        let connect_result = socket.connect(context, src_endpoint, dst_endpoint);
        if let Err(_) = connect_result {
            log::error!("failed to connect to session, session=[{}]", session);
        }
    }

    pub fn interface(&mut self) -> &mut Interface<'a, VpnDevice> {
        return &mut self.interface;
    }

    pub fn tcp_socket(&mut self) -> &mut TcpSocket<'a> {
        let socket_handle = self.socket_handle;
        let interface = self.interface();
        return interface.get_socket::<TcpSocket<'a>>(socket_handle);
    }
}
