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
use smoltcp::socket::TcpState;
use smoltcp::socket::{TcpSocket, TcpSocketBuffer};
use smoltcp::time::Instant;
use smoltcp::wire::{IpAddress, IpCidr, IpEndpoint, Ipv4Address};
use smoltcp::Error;
use std::collections::btree_map::BTreeMap;
use std::collections::HashMap;
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
                SessionManager::process_data_from_tun(&mut sessions, &mut interface, &tun_channel);
                SessionManager::process_data_to_tun(&mut interface, &tun_channel);
                SessionManager::process_tcp_stream_to_tcp_socket(&mut sessions, &mut interface);
                SessionManager::process_tcp_socket_to_tcp_stream(&mut sessions, &mut interface);
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

    fn process_data_from_tun(sessions: &mut Sessions, interface: &mut Interface<'a, VpnDevice>, channel: &TunChannel) {
        match channel.1.try_recv() {
            Ok(bytes) => {
                log_packet("session manager : from tun", &bytes);
                if let Some(session) = Session::new(&bytes) {
                    sessions.entry(session).or_insert_with_key(|session| {
                        let socket = SessionManager::create_socket(session).unwrap();
                        let socket_handle = interface.add_socket(socket);
                        SessionData::new(session, socket_handle)
                    });
                    interface.device_mut().receive(bytes);
                }
            }
            Err(error) if error == TryRecvError::Empty => {
                // nothing to do.
            }
            Err(error) => {
                log::error!("failed to receive data from tun, error={:?}", error);
            }
        }
    }

    fn process_data_to_tun(interface: &mut Interface<'a, VpnDevice>, channel: &TunChannel) {
        match interface.poll(Instant::now()) {
            Ok(is_readiness_changed) => {
                if is_readiness_changed {
                    let device = interface.device_mut();
                    while let Some(bytes) = device.transmit() {
                        log_packet("session manager : to tun", &bytes);
                        let result = channel.0.send(bytes);
                        if let Err(error) = result {
                            log::error!("failed to send data to tun, error=[{:?}]", error);
                        }
                    }
                }
            }
            Err(error) if error == Error::Unrecognized => {
                // nothing to do.
            }
            Err(error) => {
                log::error!("failed to poll interface, error={:?}", error);
            }
        }
    }

    fn process_tcp_stream_to_tcp_socket(sessions: &mut Sessions, interface: &mut Interface<'a, VpnDevice>) {
        for (_, session_data) in sessions.iter_mut() {
            let tcp_socket = interface.get_socket::<TcpSocket>(session_data.socket_handle());
            let tcp_stream = session_data.tcp_stream();
            if tcp_stream.can_read() && tcp_socket.can_send() {
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

    fn process_tcp_socket_to_tcp_stream(sessions: &mut Sessions, interface: &mut Interface<'a, VpnDevice>) {
        for (_, session_data) in sessions.iter_mut() {
            let tcp_socket = interface.get_socket::<TcpSocket>(session_data.socket_handle());
            while tcp_socket.can_recv() {
                let result = tcp_socket.recv(|received_data| {
                    let tcp_stream = session_data.tcp_stream();
                    if let Err(error) = tcp_stream.write(received_data) {
                        log::error!("failed to write to tcp stream, error={:?}", error);
                    }
                    (received_data.len(), received_data)
                });
                if let Err(error) = result {
                    log::error!("failed to receive from tcp socket, error={:?}", error);
                    break;
                }
            }
        }
    }

    fn clean_up_sessions(sessions: &mut Sessions, interface: &mut Interface<'a, VpnDevice>) {
        sessions.retain(|session, session_data| {
            let socket_handle = session_data.socket_handle();
            let tcp_socket = interface.get_socket::<TcpSocket>(socket_handle);
            match tcp_socket.state() {
                TcpState::CloseWait => {
                    log::trace!("removing closed session, session=[{:?}]", session);
                    false
                }
                _ => true,
            }
        });
    }
}
