extern crate log;

use mio::unix::SourceFd;
use mio::{Events, Interest, Poll, Token};

const TOKEN: Token = Token(0);

pub struct MioHelper {
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
            poll: poll,
            events: Events::with_capacity(capacity),
        }
    }

    pub fn poll(&mut self) {
        let timeout = std::time::Duration::from_secs(1);
        let poll_result = self.poll.poll(&mut self.events, Some(timeout));
        match poll_result {
            Ok(_) => {
                let events_count = self.events.iter().count();
                log::trace!("vpn thread polled for {:?} events", events_count);
            }
            Err(error_code) => {
                log::error!("failed to poll, error={:?}", error_code);
            }
        }
    }
}
