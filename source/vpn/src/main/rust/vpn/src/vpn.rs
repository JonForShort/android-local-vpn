pub mod vpn {

    extern crate log;

    pub struct Vpn {
        file_descriptor: i32,
    }

    impl Vpn {
        pub fn new(file_descriptor: i32) -> Self {
            Self {
                file_descriptor: file_descriptor,
            }
        }

        pub fn start(&self) {
            log::trace!("starting native vpn");
        }

        pub fn stop(&self) {
            log::trace!("stopping native vpn");
            unsafe {
                libc::close(self.file_descriptor);
            }
        }
    }
}
