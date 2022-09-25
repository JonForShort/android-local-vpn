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

use smoltcp::wire::{IpProtocol, Ipv4Packet, TcpPacket};

extern crate log;

pub fn log_packet(message: &str, bytes: &Vec<u8>) {
    let result = Ipv4Packet::new_checked(&bytes);
    match result {
        Ok(ip_packet) => {
            if ip_packet.protocol() == IpProtocol::Tcp {
                let tcp_bytes = ip_packet.payload();
                let tcp_packet = TcpPacket::new_checked(tcp_bytes).unwrap();
                log::debug!(
                    "[{:?}] len={:?} tcp=[{}] tcp_len={:?} ip=[{}]",
                    message,
                    bytes.len(),
                    tcp_packet,
                    tcp_bytes.len(),
                    ip_packet
                );
            } else {
                log::debug!("[{:?}] len={:?} ip=[{}]", message, bytes.len(), ip_packet);
            }
        }
        Err(error) => {
            log::error!("[{:?}] failed to log packet, error={:?}", message, error);
        }
    }
}
