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

use super::mpsc_helper::{Channels, SyncChannels};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::TryRecvError;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread::JoinHandle;

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
            while is_thread_running.load(Ordering::SeqCst) {
                SessionManager::process_incoming_ip_layer_data(ip_layer_channels.clone());
                SessionManager::process_incoming_tcp_layer_data(tcp_layer_channels.clone());
            }
            log::trace!("session manager is stopping");
        }));
    }

    fn process_incoming_ip_layer_data(channels: SyncChannels) {
        let receive_result = channels.lock().unwrap().1.try_recv();
        match receive_result {
            Ok(incoming_data) => {
                log::trace!(
                    "processing incoming ip layer data, count={:?}",
                    incoming_data.len()
                );
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

    fn process_incoming_tcp_layer_data(channels: SyncChannels) {
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
