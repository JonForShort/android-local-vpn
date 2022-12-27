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

use super::buffers::{IncomingDataEvent, IncomingDirection, OutgoingDirection};
use super::session::Session;
use super::session_info::SessionInfo;
use super::utils::log_packet;
use crate::vpn::vpn_device::VpnDevice;
use mio::event::Event;
use mio::unix::SourceFd;
use mio::{Events, Interest, Poll, Token, Waker};
use smoltcp::iface::{Interface, InterfaceBuilder, Routes};
use smoltcp::socket::{TcpSocket, TcpState};
use smoltcp::time::Instant;
use smoltcp::wire::{IpAddress, IpCidr, Ipv4Address};
use std::collections::btree_map::BTreeMap;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fs::File;
use std::io::ErrorKind;
use std::io::{Read, Write};
use std::os::unix::io::FromRawFd;

type Sessions = HashMap<SessionInfo, Session>;
type TokensToSessions = HashMap<Token, SessionInfo>;

const EVENTS_CAPACITY: usize = 1024;

const TOKEN_TUN: Token = Token(0);
const TOKEN_WAKER: Token = Token(1);
const TOKEN_START_ID: usize = 2;

pub(crate) struct Processor<'a> {
    file_descriptor: i32,
    file: File,
    poll: Poll,
    sessions: Sessions,
    tokens_to_sessions: TokensToSessions,
    next_token_id: usize,
    interface: Interface<'a, VpnDevice>,
}

impl<'a> Processor<'a> {
    pub(crate) fn new(file_descriptor: i32) -> Processor<'a> {
        Processor {
            file_descriptor,
            file: unsafe { File::from_raw_fd(file_descriptor) },
            poll: Poll::new().unwrap(),
            sessions: Sessions::new(),
            tokens_to_sessions: TokensToSessions::new(),
            next_token_id: TOKEN_START_ID,
            interface: Processor::create_interface(),
        }
    }

    pub(crate) fn new_stop_waker(&self) -> Waker {
        Waker::new(self.poll.registry(), TOKEN_WAKER).unwrap()
    }

    pub(crate) fn run(&mut self) {
        let registry = self.poll.registry();
        registry
            .register(
                &mut SourceFd(&self.file_descriptor),
                TOKEN_TUN,
                Interest::READABLE,
            )
            .unwrap();

        let mut events = Events::with_capacity(EVENTS_CAPACITY);

        'poll_loop: loop {
            let _ = self.poll.poll(&mut events, None);

            log::trace!("handling events, count={:?}", events.iter().count());

            for event in events.iter() {
                if event.token() == TOKEN_TUN {
                    self.handle_tun_event(event);
                } else if event.token() == TOKEN_WAKER {
                    break 'poll_loop;
                } else {
                    self.handle_server_event(event);
                }
            }

            log::trace!("finished handling events");
        }
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

    fn create_session(&mut self, bytes: &Vec<u8>) -> Option<SessionInfo> {
        if let Some(session_info) = SessionInfo::new(bytes) {
            match self.sessions.entry(session_info) {
                Entry::Vacant(entry) => {
                    let token = Token(self.next_token_id);
                    if let Some(session) = Session::new(&session_info, &mut self.interface, &mut self.poll, token) {
                        self.tokens_to_sessions.insert(token, session_info);
                        self.next_token_id += 1;

                        entry.insert(session);

                        log::debug!("created session, session={:?}", session_info);

                        return Some(session_info);
                    }
                }
                Entry::Occupied(_) => {
                    return Some(session_info);
                }
            }
        } else {
            log::error!("failed to get session for bytes, len={:?}", bytes.len());
        }
        None
    }

    fn destroy_session(&mut self, session_info: SessionInfo) {
        log::trace!("destroying session, session={:?}", session_info);

        // push any pending data back to tun device before destroying session.
        self.write_to_mio(&session_info);
        self.write_to_tun();

        if let Some(session) = self.sessions.get_mut(&session_info) {
            let socket = self
                .interface
                .get_socket::<TcpSocket>(session.socket_handle);
            socket.abort();

            let tcp_stream = &mut session.tcp_stream;
            tcp_stream.close();
            tcp_stream.deregister_poll(&mut self.poll);

            self.tokens_to_sessions.remove(&session.token);

            self.sessions.remove(&session_info);
        }

        log::trace!("finished destroying session, session={:?}", session_info);
    }

    fn handle_tun_event(&mut self, event: &Event) {
        if event.is_readable() {
            log::trace!("handle tun event");

            let mut buffer: [u8; 65535] = [0; 65535];
            loop {
                match self.file.read(&mut buffer) {
                    Ok(count) => {
                        if count == 0 {
                            break;
                        }
                        let read_buffer = buffer[..count].to_vec();
                        log_packet("out", &read_buffer);

                        if let Some(session) = self.create_session(&read_buffer) {
                            self.interface.device_mut().receive(read_buffer);

                            self.write_to_tun();
                            self.read_from_mio(&session);
                            self.write_to_server(&session);
                        }
                    }
                    Err(error) => {
                        if error.kind() == ErrorKind::WouldBlock {
                            // do nothing.
                        } else {
                            log::error!("failed to read from tun, error={:?}", error);
                        }
                        break;
                    }
                }
            }

            log::trace!("finished handle tun event");
        }
    }

    fn write_to_tun(&mut self) {
        log::trace!("write to tun");

        self.interface.poll(Instant::now()).unwrap();
        while let Some(bytes) = self.interface.device_mut().transmit() {
            log_packet("in", &bytes);
            self.file.write_all(&bytes[..]).unwrap();
        }

        log::trace!("finished write to tun");
    }

    fn handle_server_event(&mut self, event: &Event) {
        if let Some(session_info) = self.tokens_to_sessions.get(&event.token()) {
            let session_info = *session_info;
            if event.is_readable() {
                log::trace!("handle server event read, session={:?}", session_info);

                self.read_from_server(&session_info);
                self.write_to_mio(&session_info);
                self.write_to_tun();

                log::trace!("finished server event read, session={:?}", session_info);
            }
            if event.is_writable() {
                log::trace!("handle server event write, session={:?}", session_info);

                self.read_from_mio(&session_info);
                self.write_to_server(&session_info);

                log::trace!("finished server event write, session={:?}", session_info);
            }
            if event.is_read_closed() || event.is_write_closed() {
                log::trace!("handle server event closed, session={:?}", session_info);

                self.destroy_session(session_info);

                log::trace!("finished server event closed, session={:?}", session_info);
            }
        }
    }

    fn read_from_server(&mut self, session_info: &SessionInfo) {
        if let Some(session) = self.sessions.get_mut(session_info) {
            log::trace!("read from server, session={:?}", session_info);

            let is_session_closed = match session.tcp_stream.read() {
                Ok((bytes, is_closed)) => {
                    if !bytes.is_empty() {
                        let event = IncomingDataEvent {
                            direction: IncomingDirection::FromServer,
                            buffer: &bytes[..],
                        };
                        session.buffers.push_data(event);
                    }
                    is_closed
                }
                Err(error) => {
                    if error.kind() == ErrorKind::WouldBlock {
                        false
                    } else if error.kind() == ErrorKind::ConnectionReset {
                        true
                    } else {
                        log::error!("failed to read from tcp stream, errro={:?}", error);
                        true
                    }
                }
            };
            if is_session_closed {
                self.destroy_session(*session_info);
            }

            log::trace!("finished read from server, session={:?}", session_info);
        }
    }

    fn write_to_server(&mut self, session_info: &SessionInfo) {
        if let Some(session) = self.sessions.get_mut(session_info) {
            log::trace!("write to server, session={:?}", session_info);

            let buffer = session
                .buffers
                .peek_data(OutgoingDirection::ToServer)
                .buffer
                .to_vec();
            match session.tcp_stream.write(&buffer[..]) {
                Ok(consumed) => {
                    session
                        .buffers
                        .consume_data(OutgoingDirection::ToServer, consumed);
                }
                Err(error) => {
                    if error.kind() == ErrorKind::WouldBlock {
                        // do nothing.
                    } else {
                        log::error!("failed to to server, error={:?}", error);
                    }
                }
            }

            log::trace!("finished write to server, session={:?}", session_info);
        }
    }

    fn read_from_mio(&mut self, session_info: &SessionInfo) {
        if let Some(session) = self.sessions.get_mut(session_info) {
            log::trace!("read from mio, session={:?}", session_info);

            let tcp_socket = self
                .interface
                .get_socket::<TcpSocket>(session.socket_handle);
            while tcp_socket.can_recv() {
                let result = tcp_socket.recv(|data| {
                    let event = IncomingDataEvent {
                        direction: IncomingDirection::FromClient,
                        buffer: data,
                    };
                    session.buffers.push_data(event);
                    (data.len(), (data))
                });
                if let Err(error) = result {
                    log::error!("failed to receive from tcp socket, error={:?}", error);
                    break;
                }
            }
            if tcp_socket.state() == TcpState::CloseWait {
                self.destroy_session(*session_info);
            }

            log::trace!("finished read from mio, session={:?}", session_info);
        }
    }

    fn write_to_mio(&mut self, session_info: &SessionInfo) {
        if let Some(session) = self.sessions.get_mut(session_info) {
            log::trace!("write to mio, session={:?}", session_info);

            let tcp_socket = self
                .interface
                .get_socket::<TcpSocket>(session.socket_handle);
            if tcp_socket.may_send() {
                let event = session.buffers.peek_data(OutgoingDirection::ToClient);
                match tcp_socket.send_slice(event.buffer) {
                    Ok(consumed) => {
                        session
                            .buffers
                            .consume_data(OutgoingDirection::ToClient, consumed);
                    }
                    Err(error) => {
                        log::error!("failed to write to client, error={:?}", error);
                    }
                }
            }

            log::trace!("finished write to mio, session={:?}", session_info);
        }
    }
}
