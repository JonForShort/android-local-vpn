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

#[macro_use]
mod jni;

#[macro_use]
mod socket_protector;

#[macro_use]
extern crate lazy_static;

pub mod android {
    extern crate android_logger;
    extern crate jni;
    extern crate log;

    use crate::jni::jni::Jni;
    use crate::socket_protector::socket_protector::SocketProtector;

    use android_logger::Config;
    use core::tun;
    use core::tun_callbacks;
    use jni::objects::{JClass, JObject};
    use jni::JNIEnv;
    use std::process;

    #[no_mangle]
    pub unsafe extern "C" fn Java_com_github_jonforshort_androidlocalvpn_vpn_LocalVpnService_onCreateNative(env: JNIEnv, class: JClass, object: JObject) {
        android_logger::init_once(
            Config::default()
                .with_tag("nativeVpn")
                .with_min_level(log::Level::Trace),
        );
        log::trace!("onCreateNative");
        set_panic_handler();
        Jni::init(env, class, object);
        SocketProtector::init();
        tun::create();
    }

    #[no_mangle]
    pub unsafe extern "C" fn Java_com_github_jonforshort_androidlocalvpn_vpn_LocalVpnService_onDestroyNative(_: JNIEnv, _: JClass) {
        log::trace!("onDestroyNative");
        tun::destroy();
        SocketProtector::release();
        Jni::release();
        remove_panic_handler();
    }

    #[no_mangle]
    pub unsafe extern "C" fn Java_com_github_jonforshort_androidlocalvpn_vpn_LocalVpnService_onStartVpn(_: JNIEnv, _: JClass, file_descriptor: i32) {
        log::trace!("onStartVpn, pid={}, fd={}", process::id(), file_descriptor);
        tun_callbacks::set_socket_created_callback(Some(on_socket_created));
        socket_protector!().start();
        tun::start(file_descriptor);
    }

    #[no_mangle]
    pub unsafe extern "C" fn Java_com_github_jonforshort_androidlocalvpn_vpn_LocalVpnService_onStopVpn(_: JNIEnv, _: JClass) {
        log::trace!("onStopVpn, pid={}", process::id());
        tun::stop();
        socket_protector!().stop();
        tun_callbacks::set_socket_created_callback(None);
    }

    fn set_panic_handler() {
        std::panic::set_hook(Box::new(|panic_info| {
            log::error!("*** PANIC [{:?}]", panic_info);
        }));
    }

    fn remove_panic_handler() {
        let _ = std::panic::take_hook();
    }

    fn on_socket_created(socket: i32) {
        socket_protector!().protect_socket(socket);
    }
}
