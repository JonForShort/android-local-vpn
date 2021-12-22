#[no_mangle]
pub extern "C" fn jni_test(x: i32) -> bool {
    x % 3 == 0
}
