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

use super::buffers::{IncomingDataEvent, IncomingDirection, OutgoingDirection, WriteError};
use super::session::Session;
use super::session_info::SessionInfo;
use super::utils::log_packet;
use mio::event::Event;
use mio::unix::SourceFd;
use mio::{Events, Interest, Poll, Token, Waker};
use smoltcp::time::Instant;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fs::File;
use std::io::ErrorKind;
use std::io::{Read, Write};
use std::os::unix::io::FromRawFd;

type Sessions<'a> = HashMap<SessionInfo, Session<'a>>;
type TokensToSessions = HashMap<Token, SessionInfo>;

const EVENTS_CAPACITY: usize = 1024;

const TOKEN_TUN: Token = Token(0);
const TOKEN_WAKER: Token = Token(1);
const TOKEN_START_ID: usize = 2;

pub(crate) struct Processor<'a> {
    file_descriptor: i32,
    file: File,
    poll: Poll,
    sessions: Sessions<'a>,
    tokens_to_sessions: TokensToSessions,
    next_token_id: usize,
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

    fn create_session(&mut self, bytes: &Vec<u8>) -> Option<SessionInfo> {
        if let Some(session_info) = SessionInfo::new(bytes) {
            match self.sessions.entry(session_info) {
                Entry::Vacant(entry) => {
                    let token = Token(self.next_token_id);
                    if let Some(session) = Session::new(&session_info, &mut self.poll, token) {
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

    fn destroy_session(&mut self, session_info: &SessionInfo) {
        log::trace!("destroying session, session={:?}", session_info);

        // push any pending data back to tun device before destroying session.
        self.write_to_smoltcp(session_info);
        self.write_to_tun(session_info);

        if let Some(session) = self.sessions.get_mut(session_info) {
            let mut smoltcp_socket = session.smoltcp_socket.get(&mut session.interface);
            smoltcp_socket.close();

            let mio_socket = &mut session.mio_socket;
            mio_socket.close();
            mio_socket.deregister_poll(&mut self.poll).unwrap();

            self.tokens_to_sessions.remove(&session.token);

            self.sessions.remove(session_info);
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

                        if let Some(session_info) = self.create_session(&read_buffer) {
                            let session = self.sessions.get_mut(&session_info).unwrap();
                            session.interface.device_mut().receive(read_buffer);

                            self.write_to_tun(&session_info);
                            self.read_from_smoltcp(&session_info);
                            self.write_to_server(&session_info);
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

    fn write_to_tun(&mut self, session_info: &SessionInfo) {
        if let Some(session) = self.sessions.get_mut(session_info) {
            log::trace!("write to tun");

            if let Err(error) = session.interface.poll(Instant::now()) {
                log::error!("failed to poll interface, error={:?}", error);
            }

            while let Some(bytes) = session.interface.device_mut().transmit() {
                log_packet("in", &bytes);
                self.file.write_all(&bytes[..]).unwrap();
            }

            log::trace!("finished write to tun");
        }
    }

    fn handle_server_event(&mut self, event: &Event) {
        if let Some(session_info) = self.tokens_to_sessions.get(&event.token()) {
            let session_info = *session_info;
            if event.is_readable() {
                log::trace!("handle server event read, session={:?}", session_info);

                self.read_from_server(&session_info);
                self.write_to_smoltcp(&session_info);
                self.write_to_tun(&session_info);

                log::trace!("finished server event read, session={:?}", session_info);
            }
            if event.is_writable() {
                log::trace!("handle server event write, session={:?}", session_info);

                self.read_from_smoltcp(&session_info);
                self.write_to_server(&session_info);

                log::trace!("finished server event write, session={:?}", session_info);
            }
            if event.is_read_closed() || event.is_write_closed() {
                log::trace!("handle server event closed, session={:?}", session_info);

                self.destroy_session(&session_info);

                log::trace!("finished server event closed, session={:?}", session_info);
            }
        }
    }

    fn read_from_server(&mut self, session_info: &SessionInfo) {
        if let Some(session) = self.sessions.get_mut(session_info) {
            log::trace!("read from server, session={:?}", session_info);

            let is_session_closed = match session.mio_socket.read() {
                Ok((read_seqs, is_closed)) => {
                    for bytes in read_seqs {
                        if !bytes.is_empty() {
                            let event = IncomingDataEvent {
                                direction: IncomingDirection::FromServer,
                                buffer: &bytes[..],
                            };
                            session.buffers.push_data(event);
                        }
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
                self.destroy_session(session_info);
            }

            log::trace!("finished read from server, session={:?}", session_info);
        }
    }

    fn write_to_server(&mut self, session_info: &SessionInfo) {
        if let Some(session) = self.sessions.get_mut(session_info) {
            log::trace!("write to server, session={:?}", session_info);

            session
                .buffers
                .write_data(OutgoingDirection::ToServer, |b| {
                    session.mio_socket.write(b).map_err(WriteError::Stderr)
                });

            log::trace!("finished write to server, session={:?}", session_info);
        }
    }

    fn read_from_smoltcp(&mut self, session_info: &SessionInfo) {
        if let Some(session) = self.sessions.get_mut(session_info) {
            log::trace!("read from smoltcp, session={:?}", session_info);

            let mut data: [u8; 65535] = [0; 65535];
            loop {
                let mut socket = session.smoltcp_socket.get(&mut session.interface);
                if !socket.can_receive() {
                    break;
                }
                match socket.receive(&mut data) {
                    Ok(data_len) => {
                        let event = IncomingDataEvent {
                            direction: IncomingDirection::FromClient,
                            buffer: &data[..data_len],
                        };
                        session.buffers.push_data(event);
                    }
                    Err(error) => {
                        log::error!("failed to receive from smoltcp, error={:?}", error);
                        break;
                    }
                }
            }

            log::trace!("finished read from smoltcp, session={:?}", session_info);
        }
    }

    fn write_to_smoltcp(&mut self, session_info: &SessionInfo) {
        if let Some(session) = self.sessions.get_mut(session_info) {
            log::trace!("write to smoltcp, session={:?}", session_info);

            let mut socket = session.smoltcp_socket.get(&mut session.interface);
            if socket.can_send() {
                session
                    .buffers
                    .write_data(OutgoingDirection::ToClient, |b| {
                        socket.send(b).map_err(WriteError::SmoltcpErr)
                    });
            }

            log::trace!("finished write to smoltcp, session={:?}", session_info);
        }
    }
}
