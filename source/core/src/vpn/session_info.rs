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

use smoltcp::wire::{IpProtocol, Ipv4Packet, Ipv6Packet, TcpPacket, UdpPacket};
use std::{fmt, hash::Hash, net::SocketAddr};

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub(crate) struct SessionInfo {
    pub(crate) source: SocketAddr,
    pub(crate) destination: SocketAddr,
    pub(crate) transport_protocol: TransportProtocol,
    pub(crate) internet_protocol: InternetProtocol,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub(crate) enum TransportProtocol {
    Tcp,
    Udp,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub(crate) enum InternetProtocol {
    Ipv4,
    Ipv6,
}

impl SessionInfo {
    pub(crate) fn new(bytes: &Vec<u8>) -> Option<SessionInfo> {
        Self::new_ipv4(bytes)
            .or_else(|| Self::new_ipv6(bytes))
            .or_else(|| {
                log::error!("failed to create session info, len={:?}", bytes.len(),);
                None
            })
    }

    fn new_ipv4(bytes: &Vec<u8>) -> Option<SessionInfo> {
        if let Ok(ip_packet) = Ipv4Packet::new_checked(&bytes) {
            match ip_packet.next_header() {
                IpProtocol::Tcp => {
                    let payload = ip_packet.payload();
                    let packet = TcpPacket::new_checked(payload).unwrap();
                    let source_ip: [u8; 4] = ip_packet.src_addr().as_bytes().try_into().unwrap();
                    let destination_ip: [u8; 4] = ip_packet.dst_addr().as_bytes().try_into().unwrap();
                    return Some(SessionInfo {
                        source: SocketAddr::from((source_ip, packet.src_port())),
                        destination: SocketAddr::from((destination_ip, packet.dst_port())),
                        transport_protocol: TransportProtocol::Tcp,
                        internet_protocol: InternetProtocol::Ipv4,
                    });
                }
                IpProtocol::Udp => {
                    let payload = ip_packet.payload();
                    let packet = UdpPacket::new_checked(payload).unwrap();
                    let source_ip: [u8; 4] = ip_packet.src_addr().as_bytes().try_into().unwrap();
                    let destination_ip: [u8; 4] = ip_packet.dst_addr().as_bytes().try_into().unwrap();
                    return Some(SessionInfo {
                        source: SocketAddr::from((source_ip, packet.src_port())),
                        destination: SocketAddr::from((destination_ip, packet.dst_port())),
                        transport_protocol: TransportProtocol::Udp,
                        internet_protocol: InternetProtocol::Ipv4,
                    });
                }
                _ => {
                    log::warn!(
                        "unsupported transport protocol, protocol=${:?}",
                        ip_packet.next_header()
                    );
                    return None;
                }
            }
        }

        None
    }

    fn new_ipv6(bytes: &Vec<u8>) -> Option<SessionInfo> {
        if let Ok(ip_packet) = Ipv6Packet::new_checked(&bytes) {
            let protocol = ip_packet.next_header();
            match protocol {
                IpProtocol::Tcp => {
                    let payload = ip_packet.payload();
                    let packet = TcpPacket::new_checked(payload).unwrap();
                    let source_ip: [u8; 16] = ip_packet.src_addr().as_bytes().try_into().unwrap();
                    let destination_ip: [u8; 16] = ip_packet.dst_addr().as_bytes().try_into().unwrap();
                    return Some(SessionInfo {
                        source: SocketAddr::from((source_ip, packet.src_port())),
                        destination: SocketAddr::from((destination_ip, packet.dst_port())),
                        transport_protocol: TransportProtocol::Tcp,
                        internet_protocol: InternetProtocol::Ipv6,
                    });
                }
                IpProtocol::Udp => {
                    let payload = ip_packet.payload();
                    let packet = UdpPacket::new_checked(payload).unwrap();
                    let source_ip: [u8; 16] = ip_packet.src_addr().as_bytes().try_into().unwrap();
                    let destination_ip: [u8; 16] = ip_packet.dst_addr().as_bytes().try_into().unwrap();
                    return Some(SessionInfo {
                        source: SocketAddr::from((source_ip, packet.src_port())),
                        destination: SocketAddr::from((destination_ip, packet.dst_port())),
                        transport_protocol: TransportProtocol::Udp,
                        internet_protocol: InternetProtocol::Ipv6,
                    });
                }
                _ => {
                    log::warn!("unsupported transport protocol, protocol=${:?}", protocol);
                    return None;
                }
            }
        }

        None
    }
}

impl fmt::Display for SessionInfo {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "[{:?}][{:?}]{}:{}->{}:{}",
            self.internet_protocol,
            self.transport_protocol,
            self.source.ip(),
            self.source.port(),
            self.destination.ip(),
            self.destination.port()
        )
    }
}
