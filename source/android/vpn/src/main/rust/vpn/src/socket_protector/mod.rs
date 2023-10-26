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

extern crate crossbeam;
extern crate jni;

use crate::jni::JniContext;
use crossbeam::channel::unbounded;
use crossbeam::channel::{Receiver, Sender};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread::JoinHandle;

lazy_static! {
    pub static ref SOCKET_PROTECTOR: Mutex<Option<SocketProtector>> = Mutex::new(None);
}

macro_rules! socket_protector {
    () => {
        crate::socket_protector::SOCKET_PROTECTOR
            .lock()
            .unwrap()
            .as_mut()
            .unwrap()
    };
}

type SenderChannel = Sender<(i32, Sender<bool>)>;
type ReceiverChannel = Receiver<(i32, Sender<bool>)>;
type ChannelPair = (SenderChannel, ReceiverChannel);

pub struct SocketProtector {
    is_thread_running: Arc<AtomicBool>,
    thread_join_handle: Option<JoinHandle<()>>,
    channel: ChannelPair,
}

impl SocketProtector {
    pub fn init() {
        let mut socket_protector = SOCKET_PROTECTOR.lock().unwrap();
        *socket_protector = Some(SocketProtector {
            is_thread_running: Arc::new(AtomicBool::new(false)),
            thread_join_handle: None,
            channel: unbounded(),
        });
    }

    pub fn release() {
        let mut socket_protector = SOCKET_PROTECTOR.lock().unwrap();
        *socket_protector = None;
    }

    pub fn start(&mut self) {
        log::trace!("starting socket protecting thread");
        self.is_thread_running.store(true, Ordering::SeqCst);
        let is_thread_running = self.is_thread_running.clone();
        let receiver_channel = self.channel.1.clone();
        self.thread_join_handle = Some(std::thread::spawn(move || {
            log::trace!("socket protecting thread is started");
            if let Some(mut jni_context) = jni!().new_context() {
                while is_thread_running.load(Ordering::SeqCst) {
                    SocketProtector::handle_protect_socket_request(&receiver_channel, &mut jni_context);
                }
            }
            log::trace!("socket protecting thread is stopping");
        }));
        log::trace!("successfully started socket protecting thread");
    }

    pub fn stop(&mut self) {
        self.is_thread_running.store(false, Ordering::SeqCst);
        //
        // solely used for unblocking thread responsible for protecting sockets.
        //
        self.protect_socket(-1);
        self.thread_join_handle.take().unwrap().join().unwrap();
    }

    fn handle_protect_socket_request(receiver: &ReceiverChannel, jni_context: &mut JniContext) {
        let (socket, reply_sender) = receiver.recv().unwrap();
        let is_socket_protected = if socket <= 0 {
            log::trace!("found invalid socket, socket={:?}", socket);
            false
        } else if jni_context.protect_socket(socket) {
            log::trace!("finished protecting socket, socket={:?}", socket);
            true
        } else {
            log::error!("failed to protect socket, socket={:?}", socket);
            false
        };
        match reply_sender.send(is_socket_protected) {
            Ok(_) => {
                log::trace!("finished sending result, socket={:?}", socket)
            }
            Err(error) => {
                log::error!(
                    "failed to send result, socket={:?} error={:?}",
                    socket,
                    error
                );
            }
        }
    }

    pub fn protect_socket(&self, socket: i32) -> bool {
        let reply_channel: (Sender<bool>, Receiver<bool>) = unbounded();
        match self.channel.0.send((socket, reply_channel.0)) {
            Ok(_) => {
                let result = reply_channel.1.recv();
                match result {
                    Ok(is_socket_protected) => {
                        if is_socket_protected {
                            log::trace!("successfully protected socket, socket={:?}", socket);
                        } else {
                            log::error!("failed to protect socket, socket={:?}", socket);
                        }
                        return is_socket_protected;
                    }
                    Err(error) => {
                        log::error!("failed to protect socket, error={:?}", error);
                    }
                }
            }
            Err(error) => {
                log::error!(
                    "failed to protect socket, socket={:?} error={:?}",
                    socket,
                    error
                );
            }
        }
        false
    }
}
