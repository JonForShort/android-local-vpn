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

use super::channel_utils::{Channels, SyncChannels, TryRecvError};
use super::session::Session;
use super::session_data::SessionData;
use crate::smoltcp_ext::wire::log_packet;
use smoltcp::time::Instant;
use smoltcp::wire::{IpProtocol, Ipv4Packet, TcpPacket};
use std::collections::HashMap;
use std::convert::TryInto;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread::JoinHandle;

type Sessions<'a> = HashMap<Session, SessionData<'a>>;

pub struct SessionManager {
    ip_layer_channels: SyncChannels<Vec<u8>>,
    tcp_layer_channels: SyncChannels<(Session, Vec<u8>)>,
    is_thread_running: Arc<AtomicBool>,
    thread_join_handle: Option<JoinHandle<()>>,
}

impl SessionManager {
    pub fn new(
        ip_layer_channels: Channels<Vec<u8>>,
        tcp_layer_channels: Channels<(Session, Vec<u8>)>,
    ) -> SessionManager {
        SessionManager {
            ip_layer_channels: Arc::new(Mutex::new(ip_layer_channels)),
            tcp_layer_channels: Arc::new(Mutex::new(tcp_layer_channels)),
            is_thread_running: Arc::new(AtomicBool::new(false)),
            thread_join_handle: None,
        }
    }

    pub fn start(&mut self) {
        log::trace!("starting session manager");
        self.is_thread_running.store(true, Ordering::SeqCst);
        let is_thread_running = self.is_thread_running.clone();
        let ip_layer_channels = self.ip_layer_channels.clone();
        let tcp_layer_channels = self.tcp_layer_channels.clone();
        self.thread_join_handle = Some(std::thread::spawn(move || {
            let mut sessions: Sessions = HashMap::new();
            while is_thread_running.load(Ordering::SeqCst) {
                SessionManager::process_outgoing_ip_layer_data(
                    &mut sessions,
                    ip_layer_channels.clone(),
                );
                SessionManager::process_incoming_tcp_layer_data(
                    &mut sessions,
                    tcp_layer_channels.clone(),
                );
                SessionManager::poll_sessions(
                    &mut sessions,
                    ip_layer_channels.clone(),
                    tcp_layer_channels.clone(),
                );
            }
            log::trace!("session manager is stopping");
        }));
    }

    fn poll_sessions(
        sessions: &mut Sessions,
        ip_layer_channels: SyncChannels<Vec<u8>>,
        tcp_layer_channels: SyncChannels<(Session, Vec<u8>)>,
    ) {
        for (session, session_data) in sessions.iter_mut() {
            let interface = session_data.interface();
            let is_packets_ready = interface.poll(Instant::now()).unwrap();
            if is_packets_ready {
                log::trace!("[{}] session is ready", session);
                let tcp_socket = session_data.tcp_socket();
                if tcp_socket.can_recv() {
                    log::trace!(
                        "[{}] socket can receive, queue_size={:?}",
                        session,
                        tcp_socket.recv_queue()
                    );
                }
                if tcp_socket.can_send() {
                    log::trace!(
                        "[{}] socket can send, queue_size={:?}",
                        session,
                        tcp_socket.send_queue()
                    );
                }
                let device = session_data.interface().device_mut();
                log::trace!("[{}] rx_queue size {}", session, device.rx_queue.len());
                log::trace!("[{}] tx_queue size {}", session, device.tx_queue.len());
                let ip_layer_channels = ip_layer_channels.lock().unwrap();
                for bytes in device.tx_queue.pop_front() {
                    if let Err(error) = ip_layer_channels.0.send(bytes.clone()) {
                        log::error!("failed to send bytes to ip layer, error=[{:?}]", error);
                    }
                }
            }
        }
    }

    fn process_outgoing_ip_layer_data(sessions: &mut Sessions, channels: SyncChannels<Vec<u8>>) {
        let result = channels.lock().unwrap().1.try_recv();
        match result {
            Ok(bytes) => {
                log_packet("outgoing ip packet", &bytes);
                if let Some(session) = SessionManager::build_session(&bytes) {
                    if sessions.contains_key(&session) {
                        log::trace!("session already exists, session=[{:?}]", session);
                    } else {
                        log::trace!("starting new session, session=[{:?}]", session);
                        sessions.insert(session.clone(), SessionData::new(&session));
                    };
                    if let Some(session_data) = sessions.get_mut(&session) {
                        let interface = session_data.interface();
                        interface.device_mut().rx_queue.push_back(bytes);
                    } else {
                        log::error!("unable to find session; session is expected to be created.")
                    }
                }
            }
            Err(error) => {
                if error == TryRecvError::Empty {
                    // wait for before trying again.
                    std::thread::sleep(std::time::Duration::from_millis(500))
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
        let ip_packet = Ipv4Packet::new_checked(&bytes).expect("truncated ip packet");
        if ip_packet.protocol() == IpProtocol::Tcp {
            let payload = ip_packet.payload();
            let tcp_packet = TcpPacket::new_checked(payload).expect("invalid tcp packet");
            let src_ip_bytes = ip_packet.src_addr().as_bytes().clone().try_into().unwrap();
            let dst_ip_bytes = ip_packet.dst_addr().as_bytes().clone().try_into().unwrap();
            Some(Session {
                src_ip: src_ip_bytes,
                src_port: tcp_packet.src_port(),
                dst_ip: dst_ip_bytes,
                dst_port: tcp_packet.dst_port(),
                protocol: u8::from(ip_packet.protocol()),
            })
        } else {
            None
        }
    }

    fn process_incoming_tcp_layer_data(
        sessions: &mut Sessions,
        channels: SyncChannels<(Session, Vec<u8>)>,
    ) {
        let receive_result = channels.lock().unwrap().1.try_recv();
        match receive_result {
            Ok(incoming_data) => {
                let received_bytes = incoming_data.1;
                let session = incoming_data.0;
                log::trace!(
                    "processing incoming tcp layer data, count={:?}, session={:?}",
                    received_bytes.len(),
                    session
                );
            }
            Err(error) => {
                if error == TryRecvError::Empty {
                    // wait for before trying again.
                    std::thread::sleep(std::time::Duration::from_millis(500))
                } else {
                    log::error!(
                        "failed to receive incoming tcp layer data, error={:?}",
                        error
                    );
                }
            }
        }
    }

    pub fn stop(&mut self) {
        log::trace!("stopping session manager");
        self.is_thread_running.store(false, Ordering::SeqCst);
        self.thread_join_handle
            .take()
            .expect("stop session manager thread")
            .join()
            .expect("join session manager thread");
        log::trace!("session manager is stopped");
    }
}
