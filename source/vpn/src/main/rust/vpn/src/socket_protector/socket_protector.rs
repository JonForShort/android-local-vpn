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

extern crate crossbeam;
extern crate jni;

use self::jni::objects::{GlobalRef, JClass, JMethodID, JObject, JValue};
use self::jni::signature::{JavaType, Primitive};
use self::jni::{JNIEnv, JavaVM};
use crossbeam::channel::unbounded;
use crossbeam::channel::{Receiver, Sender};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread::JoinHandle;

lazy_static! {
    pub static ref SOCKET_PROTECTOR: Mutex<Option<SocketProtector>> = Mutex::new(None);
}

type SenderChannel = Sender<(i32, Sender<bool>)>;
type ReceiverChannel = Receiver<(i32, Sender<bool>)>;
type ChannelPair = (SenderChannel, ReceiverChannel);

pub struct SocketProtector {
    java_vm: Arc<JavaVM>,
    object: GlobalRef,
    is_thread_running: Arc<AtomicBool>,
    thread_join_handle: Option<JoinHandle<()>>,
    channel: ChannelPair,
}

impl SocketProtector {
    pub fn init(env: JNIEnv, _: JClass, object: JObject) {
        let mut socket_protector = SOCKET_PROTECTOR.lock().unwrap();
        let java_vm = Arc::new(env.get_java_vm().unwrap());
        *socket_protector = Some(SocketProtector {
            java_vm,
            object: env.new_global_ref(object).unwrap(),
            is_thread_running: Arc::new(AtomicBool::new(false)),
            thread_join_handle: None,
            channel: unbounded(),
        });
    }

    pub fn start(&mut self) {
        log::trace!("starting socket protecting thread");
        self.is_thread_running.store(true, Ordering::SeqCst);
        let is_thread_running = self.is_thread_running.clone();
        let java_vm = self.java_vm.clone();
        let object = self.object.clone();
        let receiver_channel = self.channel.1.clone();
        self.thread_join_handle = Some(std::thread::spawn(move || {
            log::trace!("socket protecting thread is started");
            let jni_env: JNIEnv = java_vm.attach_current_thread_permanently().unwrap();
            if let Some(method_id) = SocketProtector::create_protect_jni_method_id(jni_env) {
                log::trace!("successfully created protect jni method ID");
                while is_thread_running.load(Ordering::SeqCst) {
                    SocketProtector::handle_protect_socket_request(&receiver_channel, jni_env, object.as_obj(), method_id)
                }
            }
            log::trace!("socket protecting thread is stopping");
        }));
        log::trace!("successfully started socket protecting thread");
    }

    fn handle_protect_socket_request(receiver: &ReceiverChannel, jni_env: JNIEnv, vpn_service_object: JObject, protect_method_id: JMethodID) {
        let (socket, reply_sender) = receiver.recv().unwrap();
        log::trace!("handling protect socket request, socket={:?}", socket);
        if socket <= 0 {
            log::trace!("found invalid socket");
            return;
        }
        let is_successful = JavaType::Primitive(Primitive::Boolean);
        let arguments = [JValue::Int(socket)];
        let result = jni_env.call_method_unchecked(
            vpn_service_object,
            protect_method_id,
            is_successful,
            &arguments[..],
        );
        match result {
            Ok(value) => {
                log::trace!("finished protecting socket, result={:?}", value);
                match reply_sender.send(value.z().unwrap()) {
                    Ok(_) => {
                        log::trace!("finished replying back to sender")
                    }
                    Err(_) => {
                        log::trace!("failed to replyback to sender")
                    }
                }
            }
            Err(error_code) => {
                log::error!("failed to protect socket, error={:?}", error_code);
            }
        }
    }

    fn create_protect_jni_method_id(jni_env: JNIEnv) -> Option<JMethodID> {
        if let Ok(class) = jni_env.find_class("android/net/VpnService") {
            log::trace!("found vpn service class");
            if let Ok(method_id) = jni_env.get_method_id(class, "protect", "(I)Z") {
                log::trace!("found protect method id");
                return Some(method_id);
            }
        }
        None
    }

    pub fn release() {
        let mut socket_protector = SOCKET_PROTECTOR.lock().unwrap();
        *socket_protector = None;
    }

    pub fn stop(&mut self) {
        self.is_thread_running.store(false, Ordering::SeqCst);
        //
        // solely used for unblocking thread responsible for protecting sockets.
        //
        self.protect_socket(-1);
        self.thread_join_handle.take().unwrap().join().unwrap();
    }

    pub fn protect_socket(&self, socket: i32) -> bool {
        let reply_channel: (Sender<bool>, Receiver<bool>) = unbounded();
        match self.channel.0.send((socket, reply_channel.0)) {
            Ok(_) => {
                let result = reply_channel.1.recv();
                match result {
                    Ok(is_socket_protected) => {
                        if is_socket_protected {
                            log::trace!("successfully protected socket, socket={:?}", socket);
                        } else {
                            log::error!("failed to protect socket, socket={:?}", socket);
                        }
                        return is_socket_protected;
                    }
                    Err(error) => {
                        log::error!("failed to protect socket, error={:?}", error);
                    }
                }
            }
            Err(error_code) => {
                log::error!(
                    "failed to protect socket, socket={:?} error={:?}",
                    socket,
                    error_code
                );
            }
        }
        false
    }
}
