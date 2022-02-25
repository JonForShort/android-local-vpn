mod mio_helper;
mod vpn_device;

pub mod vpn {

    extern crate log;

    use super::mio_helper::MioHelper;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::thread::JoinHandle;

    pub struct Vpn {
        file_descriptor: i32,
        thread_handle: Option<JoinHandle<()>>,
        is_thread_running: Arc<AtomicBool>,
    }

    impl Vpn {
        pub fn new(file_descriptor: i32) -> Self {
            Self {
                file_descriptor: file_descriptor,
                thread_handle: None,
                is_thread_running: Arc::new(AtomicBool::new(false)),
            }
        }

        pub fn start(&mut self) {
            log::trace!("starting native vpn");
            self.start_thread();
        }

        fn start_thread(&mut self) {
            log::trace!("starting vpn thread");
            self.is_thread_running.store(true, Ordering::SeqCst);
            let is_thread_running = self.is_thread_running.clone();
            let file_descriptor = self.file_descriptor;
            self.thread_handle = Some(std::thread::spawn(move || {
                let mut mio_helper = MioHelper::new(file_descriptor, 256);
                while is_thread_running.load(Ordering::SeqCst) {
                    mio_helper.poll();
                }
                log::trace!("vpn thread is stopping");
            }));
        }

        pub fn stop(&mut self) {
            log::trace!("stopping native vpn");
            self.stop_thread();
            unsafe {
                libc::close(self.file_descriptor);
            }
        }

        fn stop_thread(&mut self) {
            log::trace!("stopping vpn thread");
            self.is_thread_running.store(false, Ordering::SeqCst);
            self.thread_handle
                .take()
                .expect("failed to stop thread; thread is not running")
                .join()
                .expect("failed to join thread");
            log::trace!("vpn thread is stopped");
        }
    }
}
