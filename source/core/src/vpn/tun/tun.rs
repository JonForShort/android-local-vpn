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

use super::channel::TunChannel;
use crate::smoltcp_ext::wire::log_packet;
use crate::vpn::channel::types::TryRecvError;
use std::fs::File;
use std::io::ErrorKind;
use std::io::{Read, Write};
use std::os::unix::io::FromRawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

pub struct Tun {
    file_descriptor: i32,
    channel: TunChannel,
    is_thread_running: Arc<AtomicBool>,
    thread_join_handle: Option<JoinHandle<()>>,
}

impl Tun {
    pub fn new(file_descriptor: i32, channel: TunChannel) -> Tun {
        Tun {
            file_descriptor,
            channel,
            is_thread_running: Arc::new(AtomicBool::new(false)),
            thread_join_handle: None,
        }
    }

    pub fn start(&mut self) {
        log::trace!("starting");
        self.is_thread_running.store(true, Ordering::SeqCst);
        let is_thread_running = self.is_thread_running.clone();
        let channel = self.channel.clone();
        let mut file = unsafe { File::from_raw_fd(self.file_descriptor) };
        self.thread_join_handle = Some(std::thread::spawn(move || {
            while is_thread_running.load(Ordering::SeqCst) {
                Tun::poll_outgoing_data(&mut file, &channel);
                Tun::poll_incoming_data(&mut file, &channel)
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

    fn poll_outgoing_data(file: &mut File, channel: &TunChannel) {
        let mut read_buffer: [u8; 65535] = [0; 65535];
        match file.read(&mut read_buffer) {
            Ok(read_count) => {
                let bytes = read_buffer[..read_count].to_vec();
                log_packet("tun : outgoing", &bytes);
                let result = channel.0.send(bytes);
                match result {
                    Ok(_) => {
                        // nothing to do here.
                    }
                    Err(error) => {
                        log::error!("failed to send outgoing data, error={:?}", error)
                    }
                }
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                // nothing to do here.
            }
            Err(error) => {
                log::error!("failed to read from file descriptor, error={:?}", error);
            }
        }
    }

    fn poll_incoming_data(file: &mut File, channel: &TunChannel) {
        let result = channel.1.try_recv();
        match result {
            Ok(bytes) => {
                log_packet("tun : incoming", &bytes);
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
}
