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

use crate::utils::macros::enclose;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

pub struct ServiceContext {
    name: String,
    thread_join_handle: JoinHandle<()>,
    is_thread_running: Arc<AtomicBool>,
}

pub trait Service<WorkContextT> {
    fn start(&self, name: String) -> ServiceContext {
        log::trace!("starting service, name=[{}]", name);
        let is_thread_running = Arc::new(AtomicBool::new(true));
        let thread_join_handle = std::thread::spawn(enclose! { (is_thread_running) move || loop {
            let mut work_context = Self::create_work_context();
            while is_thread_running.load(Ordering::SeqCst) {
                Self::do_work(&mut work_context);
            }
        }});
        log::trace!("started service, name=[{}]", name);
        return ServiceContext {
            name: name,
            thread_join_handle: thread_join_handle,
            is_thread_running: is_thread_running,
        };
    }

    fn stop(context: ServiceContext) {
        log::trace!("stopping service, name=[{}]", context.name);
        context.is_thread_running.store(false, Ordering::SeqCst);
        context
            .thread_join_handle
            .join()
            .expect("joining service thread");
        log::trace!("stopped service, name=[{}]", context.name);
    }

    fn create_work_context() -> WorkContextT;

    fn do_work(context: &mut WorkContextT);
}
