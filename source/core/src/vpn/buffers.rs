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

use std::collections::VecDeque;

pub(crate) struct Buffers {
    client: VecDeque<u8>,
    server: VecDeque<u8>,
}

impl Buffers {
    pub fn new() -> Buffers {
        Buffers {
            client: Default::default(),
            server: Default::default(),
        }
    }

    pub fn peek_data(&mut self, direction: OutgoingDirection) -> OutgoingDataEvent {
        let buffer = match direction {
            OutgoingDirection::ToServer => &mut self.server,
            OutgoingDirection::ToClient => &mut self.client,
        };
        OutgoingDataEvent {
            direction,
            buffer: buffer.make_contiguous(),
        }
    }

    pub fn consume_data(&mut self, direction: OutgoingDirection, size: usize) {
        let buffer = match direction {
            OutgoingDirection::ToServer => &mut self.server,
            OutgoingDirection::ToClient => &mut self.client,
        };
        buffer.drain(0..size);
    }

    pub fn push_data(&mut self, event: IncomingDataEvent<'_>) {
        let direction = event.direction;
        let buffer = event.buffer;
        match direction {
            IncomingDirection::FromServer => {
                self.client.extend(buffer.iter());
            }
            IncomingDirection::FromClient => {
                self.server.extend(buffer.iter());
            }
        }
    }
}

#[derive(Eq, PartialEq, Debug)]
pub(crate) enum IncomingDirection {
    FromServer,
    FromClient,
}

#[derive(Eq, PartialEq, Debug)]
pub(crate) enum OutgoingDirection {
    ToServer,
    ToClient,
}

pub(crate) struct DataEvent<'a, T> {
    pub direction: T,
    pub buffer: &'a [u8],
}

pub(crate) type IncomingDataEvent<'a> = DataEvent<'a, IncomingDirection>;
pub(crate) type OutgoingDataEvent<'a> = DataEvent<'a, OutgoingDirection>;
