use std::os::raw::{c_char};
use std::ffi::{CString, CStr};

fn greeting(to: *const c_char) -> *mut c_char {
    let c_str = unsafe { CStr::from_ptr(to) };
    let recipient = match c_str.to_str() {
        Err(_) => "there",
        Ok(string) => string,
    };
    CString::new("Hello ".to_owned() + recipient).unwrap().into_raw()
}

#[allow(non_snake_case)]
pub mod android {
    extern crate jni;

    use self::jni::JNIEnv;
    use self::jni::objects::{JClass, JString};
    use self::jni::sys::{jstring};
    use std::ffi::CString;
    use crate::greeting;

    #[no_mangle]
    pub unsafe extern fn Java_com_github_jonforshort_vpn_greeting(env: JNIEnv, _: JClass, java_pattern: JString) -> jstring {
        let world = greeting(env.get_string(java_pattern).expect("invalid pattern string").as_ptr());
        let world_ptr = CString::from_raw(world);
        let output = env.new_string(world_ptr.to_str().unwrap()).expect("Couldn't create java string!");
        output.into_inner()
    }
}