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

use super::mpsc_helper::{Channels, SyncChannels};
use super::session::{Session, SessionData};
use smoltcp::socket::TcpSocket;
use smoltcp::time::Instant;
use smoltcp::wire::{IpProtocol, Ipv4Packet, TcpPacket};
use std::collections::HashMap;
use std::convert::TryInto;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::TryRecvError;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread::JoinHandle;

type Sessions<'a> = HashMap<Session, SessionData<'a>>;

pub struct SessionManager {
    ip_layer_channels: SyncChannels,
    tcp_layer_channels: SyncChannels,
    is_thread_running: Arc<AtomicBool>,
    thread_join_handle: Option<JoinHandle<()>>,
}

impl SessionManager {
    pub fn new(ip_layer_channels: Channels, tcp_layer_channels: Channels) -> SessionManager {
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
                SessionManager::process_incoming_ip_layer_data(
                    &mut sessions,
                    ip_layer_channels.clone(),
                );
                SessionManager::process_incoming_tcp_layer_data(
                    &mut sessions,
                    tcp_layer_channels.clone(),
                );
            }
            log::trace!("session manager is stopping");
        }));
    }

    fn process_incoming_ip_layer_data(sessions: &mut Sessions, channels: SyncChannels) {
        let receive_result = channels.lock().unwrap().1.try_recv();
        match receive_result {
            Ok(bytes) => {
                SessionManager::log_packet(&bytes);
                if let Some(session) = SessionManager::build_session(&bytes) {
                    if sessions.contains_key(&session) {
                        log::trace!("session already exists, session=[{:?}]", session);
                    } else {
                        log::trace!("starting new session, session=[{:?}]", session);
                        sessions.insert(session.clone(), SessionData::new());
                    };
                    if let Some(session_data) = sessions.get_mut(&session) {
                        let interface = session_data.interface();
                        interface.device_mut().rx_queue.push_back(bytes);
                        let is_packets_ready = interface.poll(Instant::now()).unwrap();
                        if is_packets_ready {
                            log::trace!("session is ready, session[{:?}]", session);
                        }
                    }
                }
            }
            Err(error) => {
                if error == TryRecvError::Empty {
                    // wait for before trying again.
                    std::thread::sleep(std::time::Duration::from_millis(500))
                } else {
                    log::error!(
                        "failed to receive incoming ip layer data, error={:?}",
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

    fn log_packet(bytes: &Vec<u8>) {
        let ip_packet = Ipv4Packet::new_checked(&bytes).expect("truncated ip packet");
        if ip_packet.protocol() == IpProtocol::Tcp {
            let tcp_bytes = ip_packet.payload();
            let tcp_packet = TcpPacket::new_checked(tcp_bytes).expect("invalid tcp packet");
            log::trace!(
                "tcp packet, ip_length=[{:?}], tcp_length=[{:?}], src_ip=[{:?}], src_port=[{:?}], dst_ip=[{:?}], dst_port=[{:?}], protocol=[{:?}]",
                bytes.len(),
                tcp_bytes.len(),
                ip_packet.src_addr(),
                tcp_packet.src_port(),
                ip_packet.dst_addr(),
                tcp_packet.dst_port(),
                ip_packet.protocol()
            )
        } else {
            log::trace!(
                "ip packet, ip_length=[{:?}], src_ip=[{:?}], dst_ip=[{:?}], protocol=[{:?}]",
                bytes.len(),
                ip_packet.src_addr(),
                ip_packet.dst_addr(),
                ip_packet.protocol()
            );
        }
    }

    fn process_incoming_tcp_layer_data(_sessions: &mut Sessions, channels: SyncChannels) {
        let receive_result = channels.lock().unwrap().1.try_recv();
        match receive_result {
            Ok(incoming_data) => {
                log::trace!(
                    "processing incoming tcp layer data, count={:?}",
                    incoming_data.len()
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
