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

use super::channel_utils::{Channels, TryRecvError};
use super::session::Session;
use crate::smoltcp_ext::wire::log_packet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

type TcpLayerChannels = Channels<(Session, Vec<u8>)>;

pub struct TcpLayerProcessor {
    channels: TcpLayerChannels,
    is_thread_running: Arc<AtomicBool>,
    thread_join_handle: Option<JoinHandle<()>>,
}

impl TcpLayerProcessor {
    pub fn new(channels: TcpLayerChannels) -> TcpLayerProcessor {
        TcpLayerProcessor {
            channels: channels,
            is_thread_running: Arc::new(AtomicBool::new(false)),
            thread_join_handle: None,
        }
    }

    pub fn start(&mut self) {
        log::trace!("starting tcp layer processor");
        self.is_thread_running.store(true, Ordering::SeqCst);
        let is_thread_running = self.is_thread_running.clone();
        let channels = self.channels.clone();
        self.thread_join_handle = Some(std::thread::spawn(move || {
            let channels = channels.clone();
            while is_thread_running.load(Ordering::SeqCst) {
                TcpLayerProcessor::process_incoming_tcp_layer_data(&channels);
            }
            log::trace!("tcp layer processor is stopping");
        }));
    }

    fn process_incoming_tcp_layer_data(channels: &TcpLayerChannels) {
        let result = channels.1.try_recv();
        match result {
            Ok((_, bytes)) => {
                log_packet("outgoing tcp layer", &bytes);
            }
            Err(error) => {
                if error == TryRecvError::Empty {
                    // wait for before trying again.
                    std::thread::sleep(std::time::Duration::from_millis(500))
                } else {
                    log::error!(
                        "failed to receive outgoing tcp layer data, error={:?}",
                        error
                    );
                }
            }
        }
    }

    pub fn stop(&mut self) {
        log::trace!("stopping tcp layer processor");
        self.is_thread_running.store(false, Ordering::SeqCst);
        self.thread_join_handle
            .take()
            .expect("stop tcp layer processor thread")
            .join()
            .expect("join tcp layer processor thread");
        log::trace!("tcp layer processor is stopped");
    }
}
