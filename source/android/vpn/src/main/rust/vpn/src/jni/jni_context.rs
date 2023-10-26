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

use jni::objects::{JMethodID, JObject, JValue};
use jni::signature::{Primitive, ReturnType};
use jni::JNIEnv;

pub struct JniContext<'a> {
    pub(super) jni_env: JNIEnv<'a>,
    pub(super) object: &'a JObject<'a>,
    pub(super) protect_method_id: JMethodID,
}

impl<'a> JniContext<'a> {
    pub fn protect_socket(&mut self, socket: i32) -> bool {
        if socket <= 0 {
            log::error!("invalid socket, socket={:?}", socket);
            return false;
        }
        let return_type = ReturnType::Primitive(Primitive::Boolean);
        let arguments = [JValue::Int(socket).as_jni()];
        let result = unsafe {
            self.jni_env.call_method_unchecked(
                self.object,
                self.protect_method_id,
                return_type,
                &arguments[..],
            )
        };
        match result {
            Ok(value) => {
                log::trace!("protected socket, result={:?}", value);
                value.z().unwrap()
            }
            Err(error_code) => {
                log::error!("failed to protect socket, error={:?}", error_code);
                false
            }
        }
    }
}
