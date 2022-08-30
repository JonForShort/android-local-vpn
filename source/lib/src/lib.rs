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

mod smoltcp_ext;
mod vpn;

#[macro_use]
extern crate lazy_static;

pub mod tun {

    extern crate log;

    use crate::vpn::vpn::Vpn;
    use std::process;
    use std::sync::Mutex;
    use std::sync::RwLock;

    lazy_static! {
        static ref VPN: Mutex<Option<Vpn>> = Mutex::new(None);
    }

    lazy_static! {
        static ref CALLBACK: RwLock<Option<fn(i32)>> = RwLock::new(None);
    }

    macro_rules! vpn {
        () => {
            VPN.lock().unwrap().as_mut().unwrap()
        };
    }

    pub fn create() {
        log::trace!("create");
    }

    pub fn destroy() {
        log::trace!("destroy");
    }

    pub fn start(file_descriptor: i32) {
        log::trace!("start, pid={}, fd={}", process::id(), file_descriptor);
        update_vpn(file_descriptor);
        vpn!().start();
    }

    pub fn stop() {
        log::trace!("stop, pid={}", process::id());
        vpn!().stop();
    }

    pub fn set_socket_created_callback(new_callback: Option<fn(i32)>) {
        let mut callback = CALLBACK.write().unwrap();
        *callback = new_callback;
    }

    pub fn on_socket_created(socket: i32) {
        let callback = CALLBACK.read().unwrap();
        if let Some(on_socket_created_callback) = *callback {
            on_socket_created_callback(socket);
        }
    }

    fn update_vpn(file_descriptor: i32) {
        let mut vpn = VPN.lock().unwrap();
        *vpn = Some(Vpn::new(file_descriptor));
    }
}
