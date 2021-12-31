pub mod vpn {

    pub struct Vpn {
        pub file_descriptor: i32,
    }

    impl Vpn {
        pub fn on_start(&self) {}

        pub fn on_stop(&self) {
            unsafe {
                libc::close(self.file_descriptor);
            }
        }
    }
}
