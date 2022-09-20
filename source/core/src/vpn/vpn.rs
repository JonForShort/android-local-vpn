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

extern crate log;

use super::tun::Tun;

pub struct Vpn {
    file_descriptor: i32,
    tun: Tun,
}

impl Vpn {
    pub fn new(file_descriptor: i32) -> Self {
        Self {
            file_descriptor,
            tun: Tun::new(file_descriptor),
        }
    }

    pub fn start(&mut self) {
        log::trace!("starting native vpn");
        self.tun.start();
        log::trace!("started native vpn");
    }

    pub fn stop(&mut self) {
        log::trace!("stopping native vpn");
        self.tun.stop();
        unsafe {
            libc::close(self.file_descriptor);
        }
        log::trace!("stopped native vpn");
    }
}
