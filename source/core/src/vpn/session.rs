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
use super::mio_socket::{Protocol as MioProtocol, Socket as MioSocket};
use super::session_info::SessionInfo;
use super::smoltcp_socket::{Protocol as SmoltcpProtocol, Socket as SmoltcpSocket};
use super::vpn_device::VpnDevice;
use mio::{Poll, Token};
use smoltcp::iface::Interface;
use smoltcp::wire::IpProtocol;

pub(crate) struct Session {
    pub(crate) smoltcp_socket: SmoltcpSocket,
    pub(crate) mio_socket: MioSocket,
    pub(crate) token: Token,
    pub(crate) buffers: Buffers,
}

impl Session {
    pub(crate) fn new(session_info: &SessionInfo, interface: &mut Interface<VpnDevice>, poll: &mut Poll, token: Token) -> Option<Session> {
        let protocol = IpProtocol::from(session_info.protocol);
        let ip = session_info.dst_ip;
        let port = session_info.dst_port;

        let session = Session {
            smoltcp_socket: Self::create_smoltcp_socket(protocol, ip, port, interface)?,
            mio_socket: Self::create_mio_socket(protocol, ip, port, poll, token)?,
            token,
            buffers: Buffers::new(),
        };

        Some(session)
    }

    fn create_smoltcp_socket(ip_protocol: IpProtocol, ip: [u8; 4], port: u16, interface: &mut Interface<VpnDevice>) -> Option<SmoltcpSocket> {
        let protocol = match ip_protocol {
            IpProtocol::Tcp => SmoltcpProtocol::Tcp,
            IpProtocol::Udp => SmoltcpProtocol::Udp,
            _ => return None,
        };

        SmoltcpSocket::new(protocol, ip, port, interface)
    }

    fn create_mio_socket(ip_protocol: IpProtocol, ip: [u8; 4], port: u16, poll: &mut Poll, token: Token) -> Option<MioSocket> {
        let protocol = match ip_protocol {
            IpProtocol::Tcp => MioProtocol::Tcp,
            IpProtocol::Udp => MioProtocol::Udp,
            _ => return None,
        };

        let mut mio_socket = MioSocket::new(protocol, ip, port)?;

        if let Err(error) = mio_socket.register_poll(poll, token) {
            log::error!("failed to register poll, error={:?}", error);
            return None;
        }

        Some(mio_socket)
    }
}
