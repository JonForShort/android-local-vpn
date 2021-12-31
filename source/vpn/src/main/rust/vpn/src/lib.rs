#[allow(non_snake_case)]
pub mod android {
    extern crate android_logger;
    extern crate jni;
    extern crate log;

    use self::jni::objects::JClass;
    use self::jni::JNIEnv;

    use android_logger::Config;
    use log::trace;
    use log::Level;
    use std::process;

    static mut FILE_DESCRIPTOR: i32 = -1;

    #[no_mangle]
    pub unsafe extern "C" fn Java_com_github_jonforshort_androidlocalvpn_vpn_LocalVpnService_onCreateNative(
        _: JNIEnv,
        _: JClass,
    ) {
        android_logger::init_once(
            Config::default()
                .with_tag("nativeVpn")
                .with_min_level(Level::Trace),
        );
        trace!("onCreateNative")
    }

    #[no_mangle]
    pub unsafe extern "C" fn Java_com_github_jonforshort_androidlocalvpn_vpn_LocalVpnService_onDestroyNative(
        _: JNIEnv,
        _: JClass,
    ) {
        trace!("onDestroyNative");
    }

    #[no_mangle]
    pub unsafe extern "C" fn Java_com_github_jonforshort_androidlocalvpn_vpn_LocalVpnService_onStartVpn(
        _: JNIEnv,
        _: JClass,
        file_descriptor: i32,
    ) {
        trace!("onStartVpn, pid={}, fd={}", process::id(), file_descriptor);
        FILE_DESCRIPTOR = file_descriptor;
    }

    #[no_mangle]
    pub unsafe extern "C" fn Java_com_github_jonforshort_androidlocalvpn_vpn_LocalVpnService_onStopVpn(
        _: JNIEnv,
        _: JClass,
    ) {
        trace!("onStopVpn, pid={}, fd={}", process::id(), FILE_DESCRIPTOR);
        libc::close(FILE_DESCRIPTOR);
    }
}
