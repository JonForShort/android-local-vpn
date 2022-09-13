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

use super::tcp_stream::TcpStream;
use smoltcp::iface::SocketHandle;
use std::fmt;
use std::hash::Hash;

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct Session {
    pub src_ip: [u8; 4],
    pub src_port: u16,
    pub dst_ip: [u8; 4],
    pub dst_port: u16,
    pub protocol: u8,
}

impl fmt::Display for Session {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "{}:{}->{}:{}",
            ip_octet_to_string(&self.src_ip),
            self.src_port,
            ip_octet_to_string(&self.dst_ip),
            self.dst_port
        )
    }
}

fn ip_octet_to_string(ip: &[u8; 4]) -> String {
    ip.iter().map(|&i| i.to_string() + ".").collect()
}

pub struct SessionData {
    socket_handle: SocketHandle,
    tcp_stream: TcpStream,
}

impl SessionData {
    pub fn new(session: &Session, socket_handle: SocketHandle) -> SessionData {
        let mut tcp_stream = TcpStream::new();
        tcp_stream.connect(session.dst_ip, session.dst_port);
        SessionData {
            socket_handle,
            tcp_stream,
        }
    }

    pub fn tcp_stream(&mut self) -> &mut TcpStream {
        &mut self.tcp_stream
    }

    pub fn socket_handle(&mut self) -> SocketHandle {
        self.socket_handle.clone()
    }
}
