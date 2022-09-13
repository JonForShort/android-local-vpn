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

extern crate smoltcp;

use super::session::Session;
use super::session::SessionData;
use crate::smoltcp_ext::wire::log_packet;
use crate::vpn::channel::types::TryRecvError;
use crate::vpn::tun::channel::TunChannel;
use crate::vpn::vpn_device::VpnDevice;
use smoltcp::iface::{Interface, InterfaceBuilder, Routes};
use smoltcp::socket::{TcpSocket, TcpSocketBuffer};
use smoltcp::time::Instant;
use smoltcp::wire::{IpAddress, IpCidr, IpEndpoint, IpProtocol, Ipv4Address, Ipv4Packet, TcpPacket};
use smoltcp::Error;
use std::collections::btree_map::BTreeMap;
use std::collections::HashMap;
use std::convert::TryInto;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

type Sessions<'a> = HashMap<Session, SessionData>;

pub struct SessionManager {
    tun_channel: TunChannel,
    is_thread_running: Arc<AtomicBool>,
    thread_join_handle: Option<JoinHandle<()>>,
}

impl<'a> SessionManager {
    pub fn new(tun_channel: TunChannel) -> SessionManager {
        SessionManager {
            tun_channel,
            is_thread_running: Arc::new(AtomicBool::new(false)),
            thread_join_handle: None,
        }
    }

    pub fn start(&mut self) {
        log::trace!("starting");
        self.is_thread_running.store(true, Ordering::SeqCst);
        let is_thread_running = self.is_thread_running.clone();
        let tun_channel = self.tun_channel.clone();
        self.thread_join_handle = Some(std::thread::spawn(move || {
            let mut sessions = Sessions::new();
            let tun_channel = tun_channel;
            let mut interface = SessionManager::create_interface();
            while is_thread_running.load(Ordering::SeqCst) {
                SessionManager::process_outgoing_tun_data(&mut sessions, &mut interface, &tun_channel);
                SessionManager::process_incoming_tcp_layer_data(&mut sessions, &mut interface);
                SessionManager::process_received_tcp_data(&mut sessions, &mut interface);
                SessionManager::clean_up_sessions(&mut sessions, &mut interface);
            }
            log::trace!("stopping");
        }));
    }

    pub fn stop(&mut self) {
        log::trace!("stopping");
        self.is_thread_running.store(false, Ordering::SeqCst);
        self.thread_join_handle.take().unwrap().join().unwrap();
        log::trace!("stopped");
    }

    fn build_session(bytes: &Vec<u8>) -> Option<Session> {
        let result = Ipv4Packet::new_checked(&bytes);
        match result {
            Ok(ip_packet) => {
                if ip_packet.protocol() == IpProtocol::Tcp {
                    let payload = ip_packet.payload();
                    let tcp_packet = TcpPacket::new_checked(payload).unwrap();
                    let src_ip_bytes = ip_packet.src_addr().as_bytes().try_into().unwrap();
                    let dst_ip_bytes = ip_packet.dst_addr().as_bytes().try_into().unwrap();
                    return Some(Session {
                        src_ip: src_ip_bytes,
                        src_port: tcp_packet.src_port(),
                        dst_ip: dst_ip_bytes,
                        dst_port: tcp_packet.dst_port(),
                        protocol: u8::from(ip_packet.protocol()),
                    });
                }
            }
            Err(error) => {
                log::error!(
                    "failed to build session, len={:?}, error={:?}",
                    bytes.len(),
                    error
                );
            }
        }
        None
    }

    fn create_interface() -> Interface<'a, VpnDevice> {
        let mut routes = Routes::new(BTreeMap::new());
        let default_gateway_ipv4 = Ipv4Address::new(0, 0, 0, 1);
        routes.add_default_ipv4_route(default_gateway_ipv4).unwrap();

        let interface = InterfaceBuilder::new(VpnDevice::new(), vec![])
            .any_ip(true)
            .ip_addrs([IpCidr::new(IpAddress::v4(0, 0, 0, 1), 0)])
            .routes(routes)
            .finalize();
        interface
    }

    fn create_socket(session: &Session) -> Option<TcpSocket<'a>> {
        let mut socket = TcpSocket::new(
            TcpSocketBuffer::new(vec![0; 1048576]),
            TcpSocketBuffer::new(vec![0; 1048576]),
        );

        let dst_ip = Ipv4Address::from_bytes(&session.dst_ip);
        let dst_endpoint = IpEndpoint::new(IpAddress::from(dst_ip), session.dst_port);
        if socket.listen(dst_endpoint).is_err() {
            log::error!("failed to listen for session, session=[{}]", session);
            return None;
        }

        socket.set_ack_delay(None);

        Some(socket)
    }

    fn process_outgoing_tun_data(sessions: &mut Sessions, interface: &mut Interface<'a, VpnDevice>, channel: &TunChannel) {
        let result = channel.1.try_recv();
        match result {
            Ok(bytes) => {
                log_packet("session manager : from tun", &bytes);
                if let Some(session) = SessionManager::build_session(&bytes) {
                    if sessions.contains_key(&session) {
                        log::trace!("session already exists, session=[{:?}]", session);
                    } else {
                        log::trace!("starting new session, session=[{:?}]", session);
                        let socket = SessionManager::create_socket(&session).unwrap();
                        let socket_handle = interface.add_socket(socket);
                        let session_data = SessionData::new(&session, socket_handle);
                        sessions.insert(session, session_data);
                    };
                    interface.device_mut().receive(bytes);

                    match interface.poll(Instant::now()) {
                        Ok(is_readiness_changed) => {
                            if is_readiness_changed {
                                SessionManager::process_sent_tcp_data(interface, channel)
                            }
                        }
                        Err(error) if error == Error::Unrecognized => {
                            // nothing to do.
                        }
                        Err(error) => {
                            log::error!("received error when polling interfaces, error={:?}", error);
                        }
                    }
                }
            }
            Err(error) => {
                if error == TryRecvError::Empty {
                    // do nothing.
                } else {
                    log::error!("failed to receive outgoing tun data, error={:?}", error);
                }
            }
        }
    }

    fn process_sent_tcp_data(interface: &mut Interface<'a, VpnDevice>, channel: &TunChannel) {
        let device = interface.device_mut();
        while let Some(bytes) = device.transmit() {
            log_packet("session manager : to tun", &bytes);
            let result = channel.0.send(bytes);
            if let Err(error) = result {
                log::error!("failed to send bytes to tun, error=[{:?}]", error);
            }
        }
    }

    fn process_incoming_tcp_layer_data(sessions: &mut Sessions, interface: &mut Interface<'a, VpnDevice>) {
        for (_, session_data) in sessions.iter_mut() {
            let tcp_socket = interface.get_socket::<TcpSocket>(session_data.socket_handle());
            let tcp_stream = session_data.tcp_stream();
            if tcp_stream.is_ready() && tcp_socket.can_send() {
                match tcp_stream.read() {
                    Ok(bytes) => {
                        tcp_socket.send_slice(&bytes[..]).unwrap();
                    }
                    Err(error) => {
                        log::error!("failed to read data from tcp session, error={:?}", error);
                    }
                }
            }
        }
    }

    fn process_received_tcp_data(sessions: &mut Sessions, interface: &mut Interface<'a, VpnDevice>) {
        for (_, session_data) in sessions.iter_mut() {
            let tcp_socket = interface.get_socket::<TcpSocket>(session_data.socket_handle());
            let mut buffer = vec![];
            while tcp_socket.can_recv() {
                let result = tcp_socket.recv(|received_data| {
                    buffer.extend_from_slice(received_data);
                    (received_data.len(), received_data)
                });
                if let Err(error) = result {
                    log::error!("failed to receive from tcp socket, error={:?}", error);
                    break;
                }
            }
            let tcp_stream = session_data.tcp_stream();
            if let Err(error) = tcp_stream.write(&buffer[..]) {
                log::error!("failed to write to tcp stream, error={:?}", error);
            }
        }
    }

    fn clean_up_sessions(sessions: &mut Sessions, interface: &mut Interface<'a, VpnDevice>) {
        sessions.retain(|session, session_data| {
            let socket_handle = session_data.socket_handle();
            let tcp_socket = interface.get_socket::<TcpSocket>(socket_handle);
            match tcp_socket.state() {
                smoltcp::socket::TcpState::CloseWait => {
                    log::trace!("removing closed session, session=[{:?}]", session);
                    false
                }
                _ => true,
            }
        });
    }
}
