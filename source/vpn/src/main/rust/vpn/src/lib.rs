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

    #[no_mangle]
    pub unsafe extern "C" fn Java_com_github_jonforshort_androidlocalvpn_vpn_LocalVpnService_initializeNative(
        _: JNIEnv,
        _: JClass,
    ) {
        android_logger::init_once(
            Config::default()
                .with_tag("nativeVpn")
                .with_min_level(Level::Trace),
        );
        trace!("initializing native");
    }

    #[no_mangle]
    pub unsafe extern "C" fn Java_com_github_jonforshort_androidlocalvpn_vpn_LocalVpnService_uninitializeNative(
        _: JNIEnv,
        _: JClass,
    ) {
        trace!("uninitializing native");
    }
}
