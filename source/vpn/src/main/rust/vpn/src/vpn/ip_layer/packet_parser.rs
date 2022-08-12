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

use smoltcp::wire::Ipv4Packet;

pub struct PacketParser {
    queue: Vec<u8>,
}

impl PacketParser {
    pub fn new() -> PacketParser {
        PacketParser { queue: Vec::new() }
    }

    pub fn input_bytes(&mut self, bytes: &mut Vec<u8>) {
        self.queue.append(bytes);
    }

    pub fn next_packet(&mut self) -> Option<Vec<u8>> {
        let result = Ipv4Packet::new_checked(&self.queue);
        match result {
            Ok(packet) => {
                let len = packet.to_owned().total_len() as usize;
                return Some(self.queue.drain(0..len).collect());
            }
            Err(error) if error == smoltcp::Error::Truncated => {
                return None;
            }
            Err(error) => {
                log::error!("failed to parse packet, error={:?}", error);
                return None;
            }
        }
    }
}
