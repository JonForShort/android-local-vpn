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

use crate::vpn::{
    buffers::{Buffers, TcpBuffers, UdpBuffers},
    mio_socket::{
        InternetProtocol as MioInternetProtocol, Socket as MioSocket,
        TransportProtocol as MioTransportProtocol,
    },
    session_info::{InternetProtocol, SessionInfo, TransportProtocol},
    smoltcp_socket::{Socket as SmoltcpSocket, TransportProtocol as SmoltcpProtocol},
    vpn_device::VpnDevice,
};
use mio::{Poll, Token};
use smoltcp::{
    iface::{Config, Interface, SocketSet},
    time::Instant,
    wire::{HardwareAddress, IpAddress, IpCidr, Ipv4Address},
};

pub(crate) struct Session<'a> {
    pub(crate) smoltcp_socket: SmoltcpSocket,
    pub(crate) mio_socket: MioSocket,
    pub(crate) token: Token,
    pub(crate) buffers: Buffers,
    pub(crate) interface: Interface,
    pub(crate) sockets: SocketSet<'a>,
    pub(crate) device: VpnDevice,
}

impl<'a> Session<'a> {
    pub(crate) fn new(
        session_info: &SessionInfo,
        poll: &mut Poll,
        token: Token,
    ) -> Option<Session<'a>> {
        let mut device = VpnDevice::new();
        let interface = Self::create_interface(&mut device);
        let mut sockets = SocketSet::new([]);

        let session = Session {
            smoltcp_socket: Self::create_smoltcp_socket(session_info, &mut sockets)?,
            mio_socket: Self::create_mio_socket(session_info, poll, token)?,
            token,
            buffers: Self::create_buffer(session_info),
            interface,
            sockets,
            device,
        };

        Some(session)
    }

    fn create_smoltcp_socket(
        session_info: &SessionInfo,
        sockets: &mut SocketSet<'_>,
    ) -> Option<SmoltcpSocket> {
        let transport_protocol = match session_info.transport_protocol {
            TransportProtocol::Tcp => SmoltcpProtocol::Tcp,
            TransportProtocol::Udp => SmoltcpProtocol::Udp,
        };

        SmoltcpSocket::new(
            transport_protocol,
            session_info.source,
            session_info.destination,
            sockets,
        )
    }

    fn create_mio_socket(
        session_info: &SessionInfo,
        poll: &mut Poll,
        token: Token,
    ) -> Option<MioSocket> {
        let transport_protocol = match session_info.transport_protocol {
            TransportProtocol::Tcp => MioTransportProtocol::Tcp,
            TransportProtocol::Udp => MioTransportProtocol::Udp,
        };

        let internet_protocol = match session_info.internet_protocol {
            InternetProtocol::Ipv4 => MioInternetProtocol::Ipv4,
            InternetProtocol::Ipv6 => MioInternetProtocol::Ipv6,
        };

        let mut mio_socket = MioSocket::new(
            transport_protocol,
            internet_protocol,
            session_info.destination,
        )?;

        if let Err(error) = mio_socket.register_poll(poll, token) {
            log::error!("failed to register poll, error={:?}", error);
            return None;
        }

        Some(mio_socket)
    }

    fn create_interface<D>(device: &mut D) -> Interface
    where
        D: ::smoltcp::phy::Device + ?Sized,
    {
        let default_gateway_ipv4 = Ipv4Address::new(0, 0, 0, 1);
        let config = Config::new(HardwareAddress::Ip);

        let mut interface = Interface::new(config, device, Instant::now());
        interface.set_any_ip(true);
        interface.update_ip_addrs(|ip_addrs| {
            ip_addrs
                .push(IpCidr::new(IpAddress::v4(0, 0, 0, 1), 0))
                .unwrap();
        });
        interface
            .routes_mut()
            .add_default_ipv4_route(default_gateway_ipv4)
            .unwrap();

        interface
    }

    fn create_buffer(session_info: &SessionInfo) -> Buffers {
        match session_info.transport_protocol {
            TransportProtocol::Tcp => Buffers::Tcp(TcpBuffers::new()),
            TransportProtocol::Udp => Buffers::Udp(UdpBuffers::new()),
        }
    }
}
