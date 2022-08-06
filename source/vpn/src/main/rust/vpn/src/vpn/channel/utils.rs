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

extern crate log;

use super::types::{Receiver, Sender, TryRecvError};
use crate::smoltcp_ext::wire::log_packet;
use crate::std_ext::fs::FileExt;
use mio::unix::SourceFd;
use mio::{Events, Interest, Poll, Token};
use std::fs::File;
use std::io::Write;
use std::os::unix::io::FromRawFd;

pub struct FileDescriptorChannel {
    log_tag: String,
    file: File,
    poll: Poll,
    events: Events,
    data_written_sender: Sender<Vec<u8>>,
    data_read_receiver: Receiver<Vec<u8>>,
}

impl FileDescriptorChannel {
    const TOKEN: Token = Token(0);
    const EVENTS_SIZE: usize = 16;

    pub fn new(
        log_tag: &str,
        file_descriptor: i32,
        data_written_sender: Sender<Vec<u8>>,
        data_read_receiver: Receiver<Vec<u8>>,
    ) -> FileDescriptorChannel {
        let poll = Poll::new().unwrap();
        poll.registry()
            .register(
                &mut SourceFd(&file_descriptor),
                Self::TOKEN,
                Interest::READABLE,
            )
            .expect("register file descriptor for polling");
        FileDescriptorChannel {
            log_tag: log_tag.to_string(),
            file: unsafe { File::from_raw_fd(file_descriptor) },
            poll: poll,
            events: Events::with_capacity(Self::EVENTS_SIZE),
            data_written_sender: data_written_sender,
            data_read_receiver: data_read_receiver,
        }
    }

    pub fn poll(&mut self, timeout: Option<std::time::Duration>) {
        self.poll_written_data(timeout);
        self.poll_read_data();
    }

    fn poll_written_data(&mut self, timeout: Option<std::time::Duration>) {
        let poll_result = self.poll.poll(&mut self.events, timeout);
        match poll_result {
            Ok(_) => {
                let events_count = self.events.iter().count();
                log::trace!("{} : polled for {:?} events", self.log_tag, events_count);
                let received_bytes =
                    FileDescriptorChannel::process_events(&mut self.file, &self.events);
                for bytes in received_bytes {
                    let log_message = format!("{} : written data", self.log_tag);
                    log_packet(&log_message[..], &bytes);
                    let send_result = self.data_written_sender.send(bytes);
                    match send_result {
                        Ok(_) => {
                            // nothing to do here.
                        }
                        Err(error) => {
                            log::error!("failed to send data, error={:?}", error)
                        }
                    }
                }
            }
            Err(error_code) => {
                log::error!("failed to poll, error={:?}", error_code);
            }
        }
    }

    fn process_events(file: &mut File, events: &Events) -> Vec<Vec<u8>> {
        let mut events_data = Vec::new();
        for (_, event) in events.iter().enumerate() {
            assert_eq!(event.token(), Self::TOKEN);
            assert_eq!(event.is_readable(), true);
            match file.read_all_bytes() {
                Ok(bytes) => {
                    events_data.push(bytes);
                }
                Err(error) => {
                    log::error!("failed to read all bytes from file, error={:?}", error);
                }
            }
        }
        return events_data;
    }

    fn poll_read_data(&mut self) {
        let result = self.data_read_receiver.try_recv();
        match result {
            Ok(bytes) => {
                let log_message = format!("{} : read data", self.log_tag);
                log_packet(&log_message[..], &bytes);
                match self.file.write_all(&bytes[..]) {
                    Ok(_) => {
                        log::trace!("successfully wrote bytes to file descriptor");
                    }
                    Err(error_code) => {
                        log::error!(
                            "failed to write bytes to file descriptor, error=[{:?}]",
                            error_code
                        );
                    }
                }
            }
            Err(error) => {
                if error == TryRecvError::Empty {
                    // wait for before trying again.
                    std::thread::sleep(std::time::Duration::from_millis(500))
                } else {
                    log::error!("failed to receive read data, error={:?}", error);
                }
            }
        }
    }
}
