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

use crate::vpn::channel::types::IpLayerChannels;
use crate::vpn::channel::utils::FileDescriptorChannel;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

pub struct IpLayerProcessor {
    file_descriptor: i32,
    channels: IpLayerChannels,
    is_thread_running: Arc<AtomicBool>,
    thread_join_handle: Option<JoinHandle<()>>,
}

impl IpLayerProcessor {
    pub fn new(file_descriptor: i32, channels: IpLayerChannels) -> IpLayerProcessor {
        IpLayerProcessor {
            file_descriptor: file_descriptor,
            channels: channels,
            is_thread_running: Arc::new(AtomicBool::new(false)),
            thread_join_handle: None,
        }
    }

    pub fn start(&mut self) {
        log::trace!("starting ip layer processor");
        self.is_thread_running.store(true, Ordering::SeqCst);
        let is_thread_running = self.is_thread_running.clone();
        let file_descriptor = self.file_descriptor;
        let channels = self.channels.clone();
        self.thread_join_handle = Some(std::thread::spawn(move || {
            let mut mio_helper =
                FileDescriptorChannel::new("ip layer", file_descriptor, channels.0, channels.1);
            while is_thread_running.load(Ordering::SeqCst) {
                mio_helper.poll(Some(std::time::Duration::from_secs(1)));
            }
            log::trace!("ip layer processor is stopping");
        }));
    }

    pub fn stop(&mut self) {
        log::trace!("stopping ip layer processor");
        self.is_thread_running.store(false, Ordering::SeqCst);
        self.thread_join_handle
            .take()
            .expect("stop ip layer processor thread")
            .join()
            .expect("join ip layer processor thread");
        log::trace!("ip layer processor is stopped");
    }
}
