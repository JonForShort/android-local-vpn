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

use super::channel::TcpLayerControl;
use super::channel::{TcpLayerControlChannel, TcpLayerDataChannel};
use super::session::Session as TcpSession;
use super::session_data::SessionData as TcpSessionData;
use crate::vpn::channel::types::TryRecvError;
use std::collections::hash_map::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

type TcpSessions<'a> = HashMap<TcpSession, TcpSessionData>;

pub struct TcpLayerProcessor {
    data_channel: TcpLayerDataChannel,
    control_channel: TcpLayerControlChannel,
    is_thread_running: Arc<AtomicBool>,
    thread_join_handle: Option<JoinHandle<()>>,
}

impl TcpLayerProcessor {
    pub fn new(
        data_channel: TcpLayerDataChannel,
        control_channel: TcpLayerControlChannel,
    ) -> TcpLayerProcessor {
        TcpLayerProcessor {
            data_channel: data_channel,
            control_channel: control_channel,
            is_thread_running: Arc::new(AtomicBool::new(false)),
            thread_join_handle: None,
        }
    }

    pub fn start(&mut self) {
        log::trace!("starting tcp layer processor");
        self.is_thread_running.store(true, Ordering::SeqCst);
        let is_thread_running = self.is_thread_running.clone();
        let data_channel = self.data_channel.clone();
        let control_channel = self.control_channel.clone();
        self.thread_join_handle = Some(std::thread::spawn(move || {
            let data_channel = data_channel.clone();
            let mut sessions = TcpSessions::new();
            while is_thread_running.load(Ordering::SeqCst) {
                TcpLayerProcessor::process_incoming_tcp_layer_data(&mut sessions, &data_channel);
                TcpLayerProcessor::poll_sessions(&mut sessions, &data_channel);
                TcpLayerProcessor::clean_up_sessions(&mut sessions, &control_channel);
            }
            log::trace!("tcp layer processor is stopping");
        }));
    }

    fn process_incoming_tcp_layer_data(
        sessions: &mut TcpSessions,
        data_channel: &TcpLayerDataChannel,
    ) {
        let result = data_channel.1.try_recv();
        match result {
            Ok((dst_ip, dst_port, src_ip, src_port, bytes)) => {
                log::trace!(
                    "processing incoming tcp layer data, count={:?}, dst_ip={:?}, dst_port={:?}, src_ip={:?}, src_port={:?}",
                    bytes.len(),
                    dst_ip,
                    dst_port,
                    src_ip,
                    src_port
                );
                let session = TcpSession {
                    dst_ip: dst_ip,
                    dst_port: dst_port,
                    src_ip: src_ip,
                    src_port: src_port,
                };
                if sessions.contains_key(&session) {
                    log::trace!("tcp session already exists, session=[{:?}]", session);
                } else {
                    log::trace!("starting new tcp session, session=[{:?}]", session);
                    let mut session_data = TcpSessionData::new();
                    session_data.connect_stream(dst_ip, dst_port);
                    sessions.insert(session.clone(), session_data);
                };
                log::trace!(
                    "session is ready for processing incoming data, session=[{:?}]",
                    session
                );
                if let Some(session_data) = sessions.get_mut(&session) {
                    let result = session_data.send_data(&bytes);
                    match result {
                        Ok(sent_bytes) => {
                            log::trace!(
                                "sent data, count={:?}, session=[{:?}]",
                                sent_bytes,
                                session
                            )
                        }
                        Err(error) => {
                            log::error!(
                                "failed to send data, error={:?}, session=[{:?}]",
                                error,
                                session
                            )
                        }
                    }
                }
                log::trace!(
                    "finished processing incoming data for session, session=[{:?}]",
                    session
                );
            }
            Err(error) => {
                if error == TryRecvError::Empty {
                    // wait for before trying again.
                    std::thread::sleep(std::time::Duration::from_millis(100))
                } else {
                    log::error!(
                        "failed to receive outgoing tcp layer data, error={:?}",
                        error
                    );
                }
            }
        }
    }

    fn poll_sessions(sessions: &mut TcpSessions, data_channel: &TcpLayerDataChannel) {
        for (session, session_data) in sessions.iter_mut() {
            if session_data.is_data_available() {
                log::trace!("data is available, session=[{:?}]", session);
                let data = session_data.read_data();
                log::trace!("read data, count={:?}, session=[{:?}]", data.len(), session);
                let result = data_channel.0.send((
                    session.dst_ip,
                    session.dst_port,
                    session.src_ip,
                    session.src_port,
                    data,
                ));
                match result {
                    Ok(_) => {
                        log::trace!("successfully sent data to session manager")
                    }
                    Err(error) => {
                        log::error!("failed to send data to session manager, error={:?}", error)
                    }
                }
            }
        }
    }

    fn clean_up_sessions(sessions: &mut TcpSessions, control_channel: &TcpLayerControlChannel) {
        sessions.retain(|session, session_data| {
            if session_data.is_session_closed() {
                log::trace!("cleaning up closed session, session=[{:?}]", session);
                let session_closed_control = TcpLayerControl::SessionClosed(
                    session.dst_ip,
                    session.dst_port,
                    session.src_ip,
                    session.src_port,
                );
                let result = control_channel.0.send(session_closed_control);
                match result {
                    Ok(_) => {
                        log::trace!(
                            "successfully sent session closed control, session=[{:?}]",
                            session
                        );
                    }
                    Err(error) => {
                        log::error!("failed to send session closed control, error={:?}", error)
                    }
                }
                return false;
            } else {
                return true;
            }
        });
    }

    pub fn stop(&mut self) {
        log::trace!("stopping tcp layer processor");
        self.is_thread_running.store(false, Ordering::SeqCst);
        self.thread_join_handle.take().unwrap().join().unwrap();
        log::trace!("tcp layer processor is stopped");
    }
}
