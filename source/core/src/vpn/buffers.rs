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
use std::io::ErrorKind;

pub(crate) enum Buffers {
    TCP(TCPBuffers),
    UDP(UDPBuffers),
}

pub(crate) enum WriteError {
    Stderr(std::io::Error),
    SmoltcpErr(smoltcp::Error),
}

impl Buffers {
    pub(crate) fn push_data(&mut self, event: IncomingDataEvent<'_>) {
        match self {
            Buffers::TCP(tcp_buf) => tcp_buf.push_data(event),
            Buffers::UDP(udp_buf) => udp_buf.push_data(event),
        }
    }

    pub(crate) fn write_data<F>(&mut self, direction: OutgoingDirection, mut write_fn: F)
    where
        F: FnMut(&[u8]) -> Result<usize, WriteError>,
    {
        match self {
            Buffers::TCP(tcp_buf) => {
                let buffer = tcp_buf.peek_data(&direction).to_vec();
                match write_fn(&buffer[..]) {
                    Ok(consumed) => {
                        tcp_buf.consume_data(&direction, consumed);
                    }
                    Err(error) => match error {
                        WriteError::Stderr(err) => {
                            if err.kind() == ErrorKind::WouldBlock {
                            } else {
                                log::error!(
                                    "failed to write tcp, direction: {:?}, error={:?}",
                                    direction,
                                    err
                                );
                            }
                        }
                        WriteError::SmoltcpErr(err) => {
                            log::error!(
                                "failed to write tcp, direction: {:?}, error={:?}",
                                direction,
                                err
                            );
                        }
                    },
                }
            }
            Buffers::UDP(udp_buf) => {
                let all_datagrams = udp_buf.peek_data(&direction);
                let mut consumed: usize = 0;
                // write udp packets one by one
                for datagram in all_datagrams {
                    if let Err(error) = write_fn(&datagram[..]) {
                        match error {
                            WriteError::Stderr(err) => {
                                if err.kind() == ErrorKind::WouldBlock {
                                    break;
                                } else {
                                    log::error!(
                                        "failed to write udp, direction: {:?}, error={:?}",
                                        direction,
                                        err
                                    );
                                }
                            }
                            WriteError::SmoltcpErr(err) => {
                                if err == smoltcp::Error::Exhausted || err == smoltcp::Error::Truncated {
                                    break;
                                } else {
                                    log::error!(
                                        "failed to write udp, direciton: {:?}, error={:?}",
                                        direction,
                                        err
                                    );
                                }
                            }
                        }
                    }
                    consumed += 1;
                }
                udp_buf.consume_data(&direction, consumed);
            }
        }
    }
}

pub(crate) struct TCPBuffers {
    client: VecDeque<u8>,
    server: VecDeque<u8>,
}

impl TCPBuffers {
    pub(crate) fn new() -> TCPBuffers {
        TCPBuffers {
            client: Default::default(),
            server: Default::default(),
        }
    }

    pub(crate) fn peek_data(&mut self, direction: &OutgoingDirection) -> &[u8] {
        let buffer = match direction {
            OutgoingDirection::ToServer => &mut self.server,
            OutgoingDirection::ToClient => &mut self.client,
        };
        buffer.make_contiguous()
    }

    pub(crate) fn consume_data(&mut self, direction: &OutgoingDirection, size: usize) {
        let buffer = match direction {
            OutgoingDirection::ToServer => &mut self.server,
            OutgoingDirection::ToClient => &mut self.client,
        };
        buffer.drain(0..size);
    }

    pub(crate) fn push_data(&mut self, event: IncomingDataEvent<'_>) {
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

pub(crate) struct UDPBuffers {
    client: VecDeque<Vec<u8>>,
    server: VecDeque<Vec<u8>>,
}

impl UDPBuffers {
    pub(crate) fn new() -> UDPBuffers {
        UDPBuffers {
            client: Default::default(),
            server: Default::default(),
        }
    }

    pub(crate) fn peek_data(&mut self, direction: &OutgoingDirection) -> &[Vec<u8>] {
        let buffer = match direction {
            OutgoingDirection::ToServer => &mut self.server,
            OutgoingDirection::ToClient => &mut self.client,
        };
        buffer.make_contiguous()
    }

    pub(crate) fn consume_data(&mut self, direction: &OutgoingDirection, size: usize) {
        let buffer = match direction {
            OutgoingDirection::ToServer => &mut self.server,
            OutgoingDirection::ToClient => &mut self.client,
        };
        buffer.drain(0..size);
    }

    pub(crate) fn push_data(&mut self, event: IncomingDataEvent<'_>) {
        let direction = event.direction;
        let buffer = event.buffer;
        match direction {
            IncomingDirection::FromServer => self.client.push_back(buffer.to_vec()),
            IncomingDirection::FromClient => self.server.push_back(buffer.to_vec()),
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
