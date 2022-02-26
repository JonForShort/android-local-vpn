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

use mio::unix::SourceFd;
use mio::{Events, Interest, Poll, Token};
use std::fs::File;
use std::io::Read;
use std::os::unix::io::FromRawFd;

const TOKEN: Token = Token(0);
const BUFFER_READ_SIZE: usize = 256;

pub struct MioHelper {
    file: File,
    poll: Poll,
    events: Events,
}

impl MioHelper {
    pub fn new(file_descriptor: i32, capacity: usize) -> MioHelper {
        let poll = Poll::new().unwrap();
        poll.registry()
            .register(&mut SourceFd(&file_descriptor), TOKEN, Interest::READABLE)
            .expect("register file descriptor for polling");

        MioHelper {
            file: unsafe { File::from_raw_fd(file_descriptor) },
            poll: poll,
            events: Events::with_capacity(capacity),
        }
    }

    pub fn poll(&mut self, timeout: Option<std::time::Duration>) {
        let poll_result = self.poll.poll(&mut self.events, timeout);
        match poll_result {
            Ok(_) => {
                let events_count = self.events.iter().count();
                log::trace!("vpn thread polled for {:?} events", events_count);
                MioHelper::process_events(&self.file, &self.events);
            }
            Err(error_code) => {
                log::error!("failed to poll, error={:?}", error_code);
            }
        }
    }

    fn process_events(mut file: &File, events: &Events) {
        for (i, event) in events.iter().enumerate() {
            assert_eq!(event.token(), TOKEN);
            assert_eq!(event.is_readable(), true);

            log::trace!("processing event #{:?}", i);

            let mut bytes = [0; BUFFER_READ_SIZE];
            let read_result = file.read(&mut bytes[..]);
            match read_result {
                Ok(read_bytes_count) => {
                    log::trace!("read {:?} bytes from stream", read_bytes_count);
                }
                Err(error_code) => {
                    log::trace!("failed to read stream, error={:?}", error_code);
                }
            }
        }
    }
}
