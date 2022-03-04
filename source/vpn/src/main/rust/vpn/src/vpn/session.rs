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
use smoltcp::iface::{Interface, InterfaceBuilder, SocketHandle};
use smoltcp::socket::{TcpSocket, TcpSocketBuffer};
use std::hash::Hash;

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct Session {
    pub src_ip: [u8; 4],
    pub src_port: u16,
    pub dst_ip: [u8; 4],
    pub dst_port: u16,
    pub protocol: u8,
}

pub struct SessionData<'a> {
    interface: Interface<'a, VpnDevice>,
    socket_handle: SocketHandle,
}

impl<'a> SessionData<'a> {
    pub fn new() -> SessionData<'a> {
        let socket = SessionData::create_tcp_socket();
        let mut interface = InterfaceBuilder::new(VpnDevice::new(), vec![]).finalize();
        let socket_handle = interface.add_socket(socket);
        SessionData {
            interface: interface,
            socket_handle: socket_handle,
        }
    }

    fn create_tcp_socket() -> TcpSocket<'a> {
        let tcp_rx_buffer = TcpSocketBuffer::new(vec![0; 1024]);
        let tcp_tx_buffer = TcpSocketBuffer::new(vec![0; 1024]);
        return TcpSocket::new(tcp_rx_buffer, tcp_tx_buffer);
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
