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

use std::fs::File;
use std::io::{ErrorKind, Read, Result};

const BUFFER_READ_SIZE: usize = 1024;

pub trait FileExt {
    fn read_all_bytes(&mut self) -> Result<Vec<u8>>;
}

impl FileExt for File {
    fn read_all_bytes(&mut self) -> Result<Vec<u8>> {
        let mut bytes: Vec<u8> = Vec::new();
        let mut read_buffer = [0; BUFFER_READ_SIZE];
        loop {
            let read_result = self.read(&mut read_buffer[..]);
            match read_result {
                Ok(read_bytes_count) => {
                    if read_bytes_count == 0 {
                        break;
                    }
                    bytes.extend_from_slice(&read_buffer[..read_bytes_count]);
                }
                Err(error_code) => {
                    if error_code.kind() == ErrorKind::WouldBlock {
                        break;
                    } else {
                        return Err(error_code);
                    }
                }
            }
        }
        return Ok(bytes);
    }
}
