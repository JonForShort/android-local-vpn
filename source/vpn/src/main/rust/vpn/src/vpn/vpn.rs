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

use super::ip_layer::processor::IpLayerProcessor;
use super::session_manager::session_manager::SessionManager;
use super::tcp_layer::processor::TcpLayerProcessor;
use crossbeam::channel::unbounded;

pub struct Vpn {
    file_descriptor: i32,
    ip_layer_processor: IpLayerProcessor,
    tcp_layer_processor: TcpLayerProcessor,
    session_manager: SessionManager,
}

impl Vpn {
    pub fn new(file_descriptor: i32) -> Self {
        let ip_layer_channel = unbounded();
        let tcp_layer_data_channel = unbounded();
        let tcp_layer_control_channel = unbounded();
        let session_manager_tcp_layer_channel = unbounded();
        let session_manager_ip_layer_channel = unbounded();

        let session_manager = SessionManager::new(
            (ip_layer_channel.0, session_manager_ip_layer_channel.1),
            (tcp_layer_data_channel.0, session_manager_tcp_layer_channel.1),
            tcp_layer_control_channel.clone(),
        );
        let tcp_layer_processor = TcpLayerProcessor::new(
            (session_manager_tcp_layer_channel.0, tcp_layer_data_channel.1),
            tcp_layer_control_channel.clone(),
        );
        let ip_layer_processor = IpLayerProcessor::new(
            file_descriptor,
            (session_manager_ip_layer_channel.0, ip_layer_channel.1),
        );

        Self {
            file_descriptor: file_descriptor,
            ip_layer_processor: ip_layer_processor,
            tcp_layer_processor: tcp_layer_processor,
            session_manager: session_manager,
        }
    }

    pub fn start(&mut self) {
        log::trace!("starting native vpn");
        self.ip_layer_processor.start();
        self.tcp_layer_processor.start();
        self.session_manager.start();
        log::trace!("started native vpn");
    }

    pub fn stop(&mut self) {
        log::trace!("stopping native vpn");
        self.ip_layer_processor.stop();
        self.tcp_layer_processor.stop();
        self.session_manager.stop();
        unsafe {
            libc::close(self.file_descriptor);
        }
        log::trace!("stopped native vpn");
    }
}
