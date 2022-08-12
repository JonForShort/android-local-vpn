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

use super::channel::IpLayerChannel;
use crate::smoltcp_ext::wire::log_packet;
use crate::std_ext::fs::FileExt;
use crate::vpn::channel::types::TryRecvError;
use std::fs::File;
use std::io::Write;
use std::os::unix::io::FromRawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

pub struct IpLayerProcessor {
    file_descriptor: i32,
    channel: IpLayerChannel,
    is_thread_running: Arc<AtomicBool>,
    thread_join_handle: Option<JoinHandle<()>>,
}

impl IpLayerProcessor {
    pub fn new(file_descriptor: i32, channel: IpLayerChannel) -> IpLayerProcessor {
        IpLayerProcessor {
            file_descriptor: file_descriptor,
            channel: channel,
            is_thread_running: Arc::new(AtomicBool::new(false)),
            thread_join_handle: None,
        }
    }

    pub fn start(&mut self) {
        log::trace!("starting ip layer processor");
        self.is_thread_running.store(true, Ordering::SeqCst);
        let is_thread_running = self.is_thread_running.clone();
        let channel = self.channel.clone();
        let mut file = unsafe { File::from_raw_fd(self.file_descriptor) };
        self.thread_join_handle = Some(std::thread::spawn(move || {
            while is_thread_running.load(Ordering::SeqCst) {
                IpLayerProcessor::poll_outgoing_data(&mut file, &channel);
                IpLayerProcessor::poll_incoming_data(&mut file, &channel)
            }
            log::trace!("ip layer processor is stopping");
        }));
    }

    fn poll_outgoing_data(file: &mut File, channel: &IpLayerChannel) {
        match file.read_all_bytes() {
            Ok(bytes) => {
                if bytes.len() > 0 {
                    log_packet("ip layer : outgoing", &bytes);
                    let send_result = channel.0.send(bytes);
                    match send_result {
                        Ok(_) => {
                            // nothing to do here.
                        }
                        Err(error) => {
                            log::error!("failed to send outgoing data, error={:?}", error)
                        }
                    }
                }
            }
            Err(error) => {
                log::error!("failed to read from file descriptor, error={:?}", error);
            }
        }
    }

    fn poll_incoming_data(file: &mut File, channel: &IpLayerChannel) {
        let result = channel.1.try_recv();
        match result {
            Ok(bytes) => {
                log_packet("ip layer : incoming", &bytes);
                match file.write_all(&bytes[..]) {
                    Ok(_) => {
                        // nothing to do here.
                    }
                    Err(error) => {
                        log::error!("failed to write to file descriptor, error=[{:?}]", error);
                    }
                }
            }
            Err(error) => {
                if error == TryRecvError::Empty {
                    // nothing to do.
                } else {
                    log::error!("failed to read incoming data, error={:?}", error);
                }
            }
        }
    }

    pub fn stop(&mut self) {
        log::trace!("stopping ip layer processor");
        self.is_thread_running.store(false, Ordering::SeqCst);
        self.thread_join_handle.take().unwrap().join().unwrap();
        log::trace!("ip layer processor is stopped");
    }
}
