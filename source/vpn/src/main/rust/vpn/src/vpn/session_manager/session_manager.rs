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
use super::session_data::SessionData;
use crate::smoltcp_ext::wire::log_packet;
use crate::vpn::channel::types::TryRecvError;
use crate::vpn::ip_layer::channel::IpLayerChannel;
use crate::vpn::tcp_layer::channel::TcpLayerChannel;
use crate::vpn::vpn_device::VpnDevice;
use smoltcp::time::Instant;
use smoltcp::wire::{IpProtocol, Ipv4Packet, TcpPacket};
use std::collections::HashMap;
use std::convert::TryInto;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

type Sessions<'a> = HashMap<Session, SessionData<'a, VpnDevice>>;

pub struct SessionManager {
    ip_layer_channel: IpLayerChannel,
    tcp_layer_channel: TcpLayerChannel,
    is_thread_running: Arc<AtomicBool>,
    thread_join_handle: Option<JoinHandle<()>>,
}

impl SessionManager {
    pub fn new(ip_layer_channel: IpLayerChannel, tcp_layer_channel: TcpLayerChannel) -> SessionManager {
        SessionManager {
            ip_layer_channel: ip_layer_channel,
            tcp_layer_channel: tcp_layer_channel,
            is_thread_running: Arc::new(AtomicBool::new(false)),
            thread_join_handle: None,
        }
    }

    pub fn start(&mut self) {
        log::trace!("starting session manager");
        self.is_thread_running.store(true, Ordering::SeqCst);
        let is_thread_running = self.is_thread_running.clone();
        let ip_layer_channel = self.ip_layer_channel.clone();
        let tcp_layer_channel = self.tcp_layer_channel.clone();
        self.thread_join_handle = Some(std::thread::spawn(move || {
            let mut sessions = Sessions::new();
            let ip_layer_channel = ip_layer_channel;
            let tcp_layer_channel = tcp_layer_channel;
            while is_thread_running.load(Ordering::SeqCst) {
                SessionManager::process_outgoing_ip_layer_data(&mut sessions, &ip_layer_channel);
                SessionManager::process_incoming_tcp_layer_data(&mut sessions, &tcp_layer_channel);
                SessionManager::poll_sessions(&mut sessions, &ip_layer_channel, &tcp_layer_channel);
                SessionManager::clean_up_sessions(&mut sessions);
            }
            log::trace!("session manager is stopping");
        }));
    }

    fn poll_sessions(sessions: &mut Sessions, ip_layer_channel: &IpLayerChannel, tcp_layer_channel: &TcpLayerChannel) {
        for (session, session_data) in sessions.iter_mut() {
            let interface = session_data.interface();
            interface.poll(Instant::now()).unwrap();
            SessionManager::process_received_tcp_data(session, session_data, tcp_layer_channel);
            SessionManager::process_sent_tcp_data(session_data, ip_layer_channel);
        }
    }

    fn process_received_tcp_data(session: &Session, session_data: &mut SessionData<VpnDevice>, channel: &TcpLayerChannel) {
        let tcp_socket = session_data.tcp_socket();
        while tcp_socket.can_recv() {
            let result = tcp_socket.recv(|buffer| {
                let tcp_data = (
                    session.dst_ip,
                    session.dst_port,
                    session.src_ip,
                    session.src_port,
                    buffer.to_vec(),
                );
                if let Err(error) = channel.0.send(tcp_data) {
                    log::error!("failed to send buffer to tcp layer, error={:?}", error);
                }
                (buffer.len(), buffer)
            });
            if let Err(error) = result {
                log::error!("failed to receive from tcp socket, error={:?}", error);
                break;
            }
        }
    }

    fn process_sent_tcp_data(session_data: &mut SessionData<VpnDevice>, channel: &IpLayerChannel) {
        let device = session_data.interface().device_mut();
        for bytes in device.transmit() {
            log_packet("session manager : to ip layer", &bytes);
            let result = channel.0.send(bytes);
            if let Err(error) = result {
                log::error!("failed to send bytes to ip layer, error=[{:?}]", error);
            }
        }
    }

    fn process_outgoing_ip_layer_data(sessions: &mut Sessions, channel: &IpLayerChannel) {
        let result = channel.1.try_recv();
        match result {
            Ok(bytes) => {
                log_packet("session manager : from ip layer", &bytes);
                if let Some(session) = SessionManager::build_session(&bytes) {
                    if sessions.contains_key(&session) {
                        log::trace!("session already exists, session=[{:?}]", session);
                    } else {
                        log::trace!("starting new session, session=[{:?}]", session);
                        sessions.insert(
                            session.clone(),
                            SessionData::new(&session, VpnDevice::new()),
                        );
                    };
                    if let Some(session_data) = sessions.get_mut(&session) {
                        let interface = session_data.interface();
                        interface.device_mut().receive(bytes);
                    } else {
                        log::error!("unable to find session; session is expected to be created.")
                    }
                }
            }
            Err(error) => {
                if error == TryRecvError::Empty {
                    // do nothing.
                } else {
                    log::error!(
                        "failed to receive outgoing ip layer data, error={:?}",
                        error
                    );
                }
            }
        }
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
        return None;
    }

    fn process_incoming_tcp_layer_data(sessions: &mut Sessions, channel: &TcpLayerChannel) {
        let receive_result = channel.1.try_recv();
        match receive_result {
            Ok((dst_ip, dst_port, src_ip, src_port, bytes)) => {
                log::trace!(
                    "processing incoming tcp layer data, count={:?}, dst_ip={:?}, dst_port={:?}, src_ip={:?}, src_port={:?}",
                    bytes.len(),
                    dst_ip,
                    dst_port,
                    src_ip,
                    src_port
                );
                let session = Session {
                    dst_ip: dst_ip,
                    dst_port: dst_port,
                    src_ip: src_ip,
                    src_port: src_port,
                    protocol: u8::from(IpProtocol::Tcp),
                };
                if let Some(session_data) = sessions.get_mut(&session) {
                    let tcp_socket = session_data.tcp_socket();
                    if tcp_socket.can_send() {
                        tcp_socket.send_slice(&bytes[..]).unwrap();
                    } else {
                        log::error!(
                            "failed to process incoming tcp layer data; cannot write to session socket, session={:?} count={:?} state={:?} capacity={:?} queue={:?}",
                            session,
                            bytes.len(),
                            tcp_socket.state(),
                            tcp_socket.send_capacity(),
                            tcp_socket.send_queue()
                        );
                    }
                } else {
                    log::error!(
                        "failed to process incoming tcp layer data; unable to find session{:?}",
                        session
                    );
                }
            }
            Err(error) => {
                if error == TryRecvError::Empty {
                    // do nothing.
                } else {
                    log::error!(
                        "failed to receive incoming tcp layer data, error={:?}",
                        error
                    );
                }
            }
        }
    }

    fn clean_up_sessions(sessions: &mut Sessions) {
        sessions.retain(
            |session, session_data| match session_data.tcp_socket().state() {
                smoltcp::socket::TcpState::CloseWait => {
                    log::trace!("removing closed session, session=[{:?}]", session);
                    return false;
                }
                _ => {
                    return true;
                }
            },
        );
    }

    pub fn stop(&mut self) {
        log::trace!("stopping session manager");
        self.is_thread_running.store(false, Ordering::SeqCst);
        self.thread_join_handle.take().unwrap().join().unwrap();
        log::trace!("session manager is stopped");
    }
}
