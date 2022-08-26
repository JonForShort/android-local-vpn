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
use smoltcp::iface::{Interface, InterfaceBuilder, Routes, SocketHandle};
use smoltcp::phy::Device;
use smoltcp::socket::{TcpSocket, TcpSocketBuffer};
use smoltcp::wire::IpEndpoint;
use smoltcp::wire::{IpAddress, IpCidr, Ipv4Address};
use std::collections::btree_map::BTreeMap;

pub struct SessionData<'a, DeviceT>
where
    DeviceT: for<'d> Device<'d>,
{
    interface: Interface<'a, DeviceT>,
    socket_handle: SocketHandle,
}

impl<'a, DeviceT> SessionData<'a, DeviceT>
where
    DeviceT: for<'d> Device<'d>,
{
    pub fn new(session: &Session, device: DeviceT) -> SessionData<'a, DeviceT> {
        let mut socket = create_socket();
        set_up_socket(session, &mut socket);

        let mut interface = InterfaceBuilder::new(device, vec![])
            .any_ip(true)
            .ip_addrs([IpCidr::new(IpAddress::v4(0, 0, 0, 1), 0)])
            .routes(create_routes())
            .finalize();

        let socket_handle = interface.add_socket(socket);

        SessionData {
            interface,
            socket_handle,
        }
    }

    pub fn interface(&mut self) -> &mut Interface<'a, DeviceT> {
        &mut self.interface
    }

    pub fn tcp_socket(&mut self) -> &mut TcpSocket<'a> {
        let socket_handle = self.socket_handle;
        let interface = self.interface();
        interface.get_socket::<TcpSocket<'a>>(socket_handle)
    }
}

fn create_routes<'a>() -> Routes<'a> {
    let mut routes = Routes::new(BTreeMap::new());
    let default_gateway_ipv4 = Ipv4Address::new(0, 0, 0, 1);
    routes.add_default_ipv4_route(default_gateway_ipv4).unwrap();
    routes
}

fn create_socket<'a>() -> TcpSocket<'a> {
    let tcp_rx_buffer = TcpSocketBuffer::new(vec![0; 1048576]);
    let tcp_tx_buffer = TcpSocketBuffer::new(vec![0; 1048576]);
    TcpSocket::new(tcp_rx_buffer, tcp_tx_buffer)
}

fn set_up_socket(session: &Session, socket: &mut TcpSocket) {
    let dst_ip = Ipv4Address::from_bytes(&session.dst_ip);
    let dst_endpoint = IpEndpoint::new(IpAddress::from(dst_ip), session.dst_port);
    if socket.listen(dst_endpoint).is_err() {
        log::error!("failed to listen for session, session=[{}]", session);
    }

    socket.set_ack_delay(None);
}
