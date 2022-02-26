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
            let file_descriptor = unsafe { libc::dup(self.file_descriptor) };
            self.thread_handle = Some(std::thread::spawn(move || {
                let mut mio_helper = MioHelper::new(file_descriptor, 256);
                while is_thread_running.load(Ordering::SeqCst) {
                    mio_helper.poll(Some(std::time::Duration::from_secs(1)));
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
